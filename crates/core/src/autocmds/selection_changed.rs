//! Selection/cursor position change notifications
//!
//! Handles CursorMoved and CursorMovedI events, sending selectionDidChange
//! notifications when cursor position or visual selection changes.
//!
//! Implements 10ms debouncing to avoid excessive notifications during
//! rapid cursor movement.
//!
//! Only broadcasts when selection actually changes (matches amp.nvim behavior).

use std::{cell::RefCell, sync::Arc, time::Duration};

use nvim_oxi::{
    api::{self, opts::CreateAutocmdOpts, types::AutocmdCallbackArgs},
    libuv::TimerHandle,
};

use crate::{
    errors::Result,
    notifications,
    nvim::{cursor, path, selection},
    server::Hub,
};

/// Debounce delay for cursor movement notifications (10ms)
pub(super) const DEBOUNCE_MS: u64 = 10;

/// Selection state for change detection
#[derive(Debug, Clone, PartialEq)]
struct SelectionState {
    uri:        String,
    start_line: usize,
    start_char: usize,
    end_line:   usize,
    end_char:   usize,
    content:    String,
}

thread_local! {
    /// Last broadcasted selection state (for change detection)
    static LAST_SELECTION: RefCell<Option<SelectionState>> = RefCell::new(None);
}

/// Register CursorMoved, CursorMovedI, and ModeChanged autocommands
///
/// # Arguments
/// * `group_id` - Autocommand group ID
/// * `hub` - WebSocket Hub for broadcasting
pub(super) fn register(group_id: u32, hub: Arc<Hub>) -> Result<()> {
    // Storage for debounce timer (RefCell for interior mutability)
    let debounce_timer = RefCell::new(None::<TimerHandle>);

    // Clone hub for cursor movement callback
    let hub_cursor = hub.clone();

    // Register cursor movement events (debounced)
    let cursor_opts = CreateAutocmdOpts::builder()
        .group(group_id)
        .desc("Send selectionDidChange notification on cursor move (debounced)")
        .callback(move |_args: AutocmdCallbackArgs| {
            // Cancel previous timer if exists
            if let Some(mut timer) = debounce_timer.borrow_mut().take() {
                let _ = timer.stop();
            }

            // Clone hub for timer callback
            let hub_clone = hub_cursor.clone();

            // Start new debounced timer
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

    // BufEnter/WinEnter catch URI changes even when cursor position doesn't change
    api::create_autocmd(["CursorMoved", "CursorMovedI", "BufEnter", "WinEnter"], &cursor_opts)
        .map_err(|e| {
            crate::errors::AmpError::Other(format!("Failed to create cursor autocmd: {}", e))
        })?;

    // Register ModeChanged event (immediate, no debounce)
    // Critical for responsive visual mode selection tracking
    let mode_opts = CreateAutocmdOpts::builder()
        .group(group_id)
        .desc("Send selectionDidChange notification on mode change (immediate)")
        .callback(move |_args: AutocmdCallbackArgs| {
            // No debouncing - immediate update for mode changes
            if let Err(e) = handle_event(&hub) {
                // Errors are silently ignored to avoid ghost text
                let _ = e;
            }

            // Keep autocommand active (return false)
            Ok::<_, nvim_oxi::Error>(false)
        })
        .build();

    api::create_autocmd(["ModeChanged"], &mode_opts).map_err(|e| {
        crate::errors::AmpError::Other(format!("Failed to create mode autocmd: {}", e))
    })?;

    Ok(())
}

/// Handle cursor/selection change event
///
/// Gets current cursor position and selection, sends notification to clients.
/// Only broadcasts if selection actually changed (prevents duplicate notifications).
pub(crate) fn handle_event(hub: &Hub) -> Result<()> {
    // Safety check: ensure Neovim is available
    if !crate::ide_ops::nvim_available() {
        return Ok(());
    }

    // Get current buffer and file path
    let buf = api::get_current_buf();
    let buf_path = buf
        .get_name()
        .map_err(|e| crate::errors::AmpError::Other(format!("Failed to get buffer name: {}", e)))?;

    // Convert to file:// URI
    let uri = if buf_path.is_absolute() {
        path::to_uri(&buf_path)?
    } else {
        // Handle unnamed/scratch buffers - skip notification
        return Ok(());
    };

    // Check if in visual mode
    let mode = selection::get_mode()?;

    let (start_line, start_char, end_line, end_char, content) = if mode.mode.is_visual() {
        // Visual mode - get actual selection
        match selection::get_visual_selection(&buf, &mode.mode) {
            Ok(Some((s_line, s_col, e_line, e_col, text))) => {
                // Marks are (1,0)-indexed, convert to 0-indexed
                let start_line = s_line.saturating_sub(1);
                let start_char = s_col;
                let end_line = e_line.saturating_sub(1);
                let end_char = e_col;
                (start_line, start_char, end_line, end_char, text)
            },
            Ok(None) | Err(_) => {
                // Fallback to cursor position if marks fail
                cursor::get_position_as_range()?
            },
        }
    } else {
        // Normal mode - send cursor position as zero-width selection
        cursor::get_position_as_range()?
    };

    // Create current state
    let current_state = SelectionState {
        uri: uri.clone(),
        start_line,
        start_char,
        end_line,
        end_char,
        content: content.clone(),
    };

    // Check if state changed (only broadcast if different)
    let should_broadcast = LAST_SELECTION.with(|last| {
        let mut last = last.borrow_mut();
        let changed = last.as_ref() != Some(&current_state);

        if changed {
            *last = Some(current_state);
        }

        changed
    });

    if should_broadcast {
        notifications::send_selection_changed(
            hub, &uri, start_line, start_char, end_line, end_char, &content,
        )?;
    }

    Ok(())
}