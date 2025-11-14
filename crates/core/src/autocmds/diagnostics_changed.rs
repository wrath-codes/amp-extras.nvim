//! Diagnostics change notifications
//!
//! Handles DiagnosticChanged event, sending diagnosticsDidChange notifications
//! when LSP diagnostics are updated for any buffer.
//!
//! Implements 10ms debouncing to batch rapid diagnostic updates.
//!
//! Only broadcasts when diagnostics actually change (matches amp.nvim behavior).

use std::{cell::RefCell, collections::HashMap, sync::Arc, time::Duration};

use nvim_oxi::{
    api::{self, opts::CreateAutocmdOpts, types::AutocmdCallbackArgs},
    libuv::TimerHandle,
};
use serde_json::{json, Value};

use crate::{errors::Result, notifications, nvim::path, server::Hub};

/// Debounce delay for diagnostic notifications (10ms)
pub(super) const DEBOUNCE_MS: u64 = 10;

thread_local! {
    /// Last broadcasted diagnostics (uri -> diagnostics array)
    static LAST_DIAGNOSTICS: RefCell<Option<HashMap<String, Vec<Value>>>> = RefCell::new(None);
}

/// Register DiagnosticChanged autocommand
///
/// # Arguments
/// * `group_id` - Autocommand group ID
/// * `hub` - WebSocket Hub for broadcasting
pub(super) fn register(group_id: u32, hub: Arc<Hub>) -> Result<()> {
    // Storage for debounce timer (RefCell for interior mutability)
    let debounce_timer = RefCell::new(None::<TimerHandle>);

    let opts = CreateAutocmdOpts::builder()
        .group(group_id)
        .desc("Send diagnosticsDidChange notification on diagnostic update (debounced)")
        .callback(move |_args: AutocmdCallbackArgs| {
            // Cancel previous timer if exists
            if let Some(mut timer) = debounce_timer.borrow_mut().take() {
                let _ = timer.stop();
            }

            // Clone hub for timer callback
            let hub_clone = hub.clone();

            // Start new debounced timer (10ms to batch updates)
            match TimerHandle::once(Duration::from_millis(DEBOUNCE_MS), move || {
                // Execute after debounce delay
                if let Err(e) = handle_event(&hub_clone) {
                    // Errors are silently ignored to avoid ghost text
                    let _ = e;
                }
                Ok::<_, nvim_oxi::Error>(())
            }) {
                Ok(timer) => {
                    // Store timer to keep it alive
                    *debounce_timer.borrow_mut() = Some(timer);
                },
                Err(_e) => {
                    // Timer creation failed - skip this event
                },
            }

            // Keep autocommand active (return false)
            Ok::<_, nvim_oxi::Error>(false)
        })
        .build();

    // Register DiagnosticChanged event
    api::create_autocmd(["DiagnosticChanged"], &opts).map_err(|e| {
        crate::errors::AmpError::Other(format!("Failed to create diagnostic autocmd: {}", e))
    })?;

    Ok(())
}

/// Handle diagnostic change event
///
/// Collects diagnostics from all buffers and sends notification if changed.
/// Only broadcasts if diagnostics actually changed (prevents duplicate notifications).
pub(crate) fn handle_event(hub: &Hub) -> Result<()> {
    // Safety check: ensure Neovim is available
    if !crate::ide_ops::nvim_available() {
        return Ok(());
    }

    // Get diagnostics for all buffers
    let diagnostics_map = collect_all_diagnostics()?;

    // Check if diagnostics changed (only broadcast if different)
    let should_broadcast = LAST_DIAGNOSTICS.with(|last| {
        let mut last = last.borrow_mut();

        // Compare with previous state
        let changed = last.as_ref() != Some(&diagnostics_map);

        if changed {
            *last = Some(diagnostics_map.clone());
        }

        changed
    });

    if should_broadcast {
        // Convert to entries array format
        let entries: Vec<Value> = diagnostics_map
            .into_iter()
            .map(|(uri, diagnostics)| {
                json!({
                    "uri": uri,
                    "diagnostics": diagnostics
                })
            })
            .collect();

        notifications::send_diagnostics_changed(hub, entries)?;
    }

    Ok(())
}

/// Collect diagnostics from all loaded buffers
///
/// Returns HashMap of URI -> diagnostics array
fn collect_all_diagnostics() -> Result<HashMap<String, Vec<Value>>> {
    use crate::ide_ops::{get_line_content, map_severity, NvimDiagnostic};

    let mut diagnostics_map: HashMap<String, Vec<Value>> = HashMap::new();

    // Iterate through all buffers
    for buf in api::list_bufs() {
        // Skip unloaded buffers
        if !buf.is_loaded() {
            continue;
        }

        // Get buffer path
        let Ok(buf_path) = buf.get_name() else {
            continue;
        };

        // Only consider absolute paths
        if !buf_path.is_absolute() {
            continue;
        }

        // Get diagnostics using vim.diagnostic.get(bufnr)
        let bufnr = buf.handle();
        let lua_expr = "vim.diagnostic.get(_A)";
        let result: std::result::Result<nvim_oxi::Object, _> =
            api::call_function("luaeval", (lua_expr, bufnr));

        let Ok(diag_obj) = result else {
            continue;
        };

        // Deserialize diagnostics
        let diags: Vec<NvimDiagnostic> = match crate::conversion::from_object(diag_obj) {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Skip if no diagnostics for this buffer
        if diags.is_empty() {
            continue;
        }

        // Get LSP-compliant URI with percent-encoding
        let uri = path::to_uri(&buf_path)?;

        let diagnostics: Vec<Value> = diags
            .into_iter()
            .map(|diag| {
                let line_content = get_line_content(&buf_path, diag.lnum);
                let start_line = diag.lnum;
                let start_char = diag.col;
                let end_line = diag.end_lnum.unwrap_or(diag.lnum);
                let end_char = diag.end_col.unwrap_or(diag.col);

                // Calculate character offsets (simple approach)
                let start_offset = start_char;
                let end_offset = end_char;

                json!({
                    "range": {
                        "startLine": start_line,
                        "startCharacter": start_char,
                        "endLine": end_line,
                        "endCharacter": end_char
                    },
                    "severity": map_severity(diag.severity),
                    "description": diag.message,
                    "lineContent": line_content,
                    "startOffset": start_offset,
                    "endOffset": end_offset
                })
            })
            .collect();

        diagnostics_map.insert(uri, diagnostics);
    }

    Ok(diagnostics_map)
}
