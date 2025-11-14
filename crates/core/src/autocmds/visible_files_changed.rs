//! Visible files change notifications
//!
//! Handles BufEnter and WinEnter events, sending visibleFilesDidChange
//! notifications when the set of visible files changes.
//!
//! Implements 10ms debouncing to avoid excessive notifications during
//! rapid window switching.
//!
//! Only broadcasts when visible files actually change (matches amp.nvim behavior).

use std::{cell::RefCell, sync::Arc, time::Duration};

use nvim_oxi::{
    api::{self, opts::CreateAutocmdOpts, types::AutocmdCallbackArgs},
    libuv::TimerHandle,
};

use crate::{
    errors::Result,
    notifications,
    nvim::{path, window},
    server::Hub,
};

/// Debounce delay for visible files notifications (10ms)
pub(super) const DEBOUNCE_MS: u64 = 10;

thread_local! {
    /// Last broadcasted visible files URIs (for change detection)
    static LAST_VISIBLE_FILES: RefCell<Option<Vec<String>>> = RefCell::new(None);
}

/// Register buffer, window, and tab autocommands
///
/// Tracks all events that can change visible files (matches amp.nvim):
/// - BufWinEnter, BufWinLeave, BufRead, BufNewFile - Buffer visibility changes
/// - WinNew, WinClosed - Window lifecycle
/// - TabEnter, TabNew, TabClosed - Tab switching and lifecycle
///
/// # Arguments
/// * `group_id` - Autocommand group ID
/// * `hub` - WebSocket Hub for broadcasting
pub(super) fn register(group_id: u32, hub: Arc<Hub>) -> Result<()> {
    // Storage for debounce timer (RefCell for interior mutability)
    let debounce_timer = RefCell::new(None::<TimerHandle>);

    let opts = CreateAutocmdOpts::builder()
        .group(group_id)
        .desc("Send visibleFilesDidChange notification on buffer/window/tab changes (debounced)")
        .callback(move |_args: AutocmdCallbackArgs| {
            // Cancel previous timer if exists
            if let Some(mut timer) = debounce_timer.borrow_mut().take() {
                let _ = timer.stop();
            }

            // Clone hub for timer callback
            let hub_clone = hub.clone();

            // Start new debounced timer (10ms to let state settle)
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

    // Register all events that can change visible files (matches amp.nvim)
    api::create_autocmd(
        [
            "BufWinEnter",
            "BufWinLeave",
            "BufEnter",  // Buffer visibility (replaces BufRead for better semantics)
            "WinEnter",  // Window switches can change visible set
            "WinNew",
            "WinClosed",
            "TabEnter",
            "TabNew",
            "TabClosed",
        ],
        &opts,
    )
    .map_err(|e| crate::errors::AmpError::Other(format!("Failed to create autocmds: {}", e)))?;

    Ok(())
}

/// Handle visible files changed event
///
/// Gets list of all visible buffers, sends notification to clients.
/// Only broadcasts if visible files actually changed (prevents duplicate notifications).
pub(crate) fn handle_event(hub: &Hub) -> Result<()> {
    // Safety check: ensure Neovim is available
    if !crate::ide_ops::nvim_available() {
        return Ok(());
    }

    // Get all visible buffer paths
    let paths = window::get_visible_buffers()?;

    // Convert to file:// URIs
    let mut uris: Vec<String> = paths
        .iter()
        .map(|p| path::to_uri(p))
        .collect::<Result<Vec<String>>>()?;

    // Sort for consistent comparison (amp.nvim doesn't guarantee order)
    uris.sort();

    // Check if visible files changed (only broadcast if different)
    let should_broadcast = LAST_VISIBLE_FILES.with(|last| {
        let mut last = last.borrow_mut();

        // Compare count first (early exit optimization)
        if let Some(ref last_uris) = *last {
            if last_uris.len() != uris.len() {
                *last = Some(uris.clone());
                return true;
            }

            // Check if any URI changed
            let changed = last_uris != &uris;

            if changed {
                *last = Some(uris.clone());
            }

            changed
        } else {
            // First time - always broadcast
            *last = Some(uris.clone());
            true
        }
    });

    if should_broadcast {
        notifications::send_visible_files_changed(hub, uris)?;
    }

    Ok(())
}
