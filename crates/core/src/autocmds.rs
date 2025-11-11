//! Neovim autocommand setup for WebSocket notifications
//!
//! This module sets up autocommands that trigger WebSocket notifications
//! when Neovim events occur:
//! - CursorMoved/CursorMovedI → selectionDidChange
//! - BufEnter/WinEnter → visibleFilesDidChange

use std::sync::Arc;

use nvim_oxi::api::{self, opts::CreateAutocmdOpts, opts::CreateAugroupOpts};
use nvim_oxi::api::types::AutocmdCallbackArgs;

use crate::errors::Result;
use crate::notifications;
use crate::server::Hub;

/// Autocommand group name for amp-extras notifications
const AUGROUP_NAME: &str = "AmpExtrasNotifications";

/// Setup all notification autocommands
///
/// Creates an autocommand group and registers callbacks for:
/// - Cursor movement (CursorMoved, CursorMovedI)
/// - Buffer/window changes (BufEnter, WinEnter)
///
/// # Arguments
/// * `hub` - WebSocket Hub for broadcasting notifications
///
/// # Returns
/// * `Ok(())` if setup succeeded
/// * `Err(AmpError)` if autocommand creation failed
pub fn setup_notifications(hub: Hub) -> Result<()> {
    // Create autocommand group (clear existing if present)
    let group_opts = CreateAugroupOpts::builder()
        .clear(true)
        .build();
    
    let group_id = api::create_augroup(AUGROUP_NAME, &group_opts)
        .map_err(|e| crate::errors::AmpError::Other(format!("Failed to create augroup: {}", e)))?;
    
    // Setup cursor movement notifications
    setup_cursor_moved_autocmd(group_id, Arc::new(hub.clone()))?;
    
    // Setup visible files notifications
    setup_visible_files_autocmd(group_id, Arc::new(hub))?;
    
    Ok(())
}

/// Setup autocommand for cursor movement
///
/// Triggers on CursorMoved and CursorMovedI events.
/// Sends selectionDidChange notification with current cursor position.
fn setup_cursor_moved_autocmd(group_id: u32, hub: Arc<Hub>) -> Result<()> {
    let opts = CreateAutocmdOpts::builder()
        .group(group_id)
        .desc("Send selectionDidChange notification on cursor move")
        .callback(move |args: AutocmdCallbackArgs| {
            // Get current cursor position and send notification
            if let Err(e) = handle_cursor_moved(&hub, &args) {
                eprintln!("Error in cursor moved handler: {}", e);
            }
            
            // Keep autocommand active (return false)
            Ok::<_, nvim_oxi::Error>(false)
        })
        .build();
    
    api::create_autocmd(["CursorMoved", "CursorMovedI"], &opts)
        .map_err(|e| crate::errors::AmpError::Other(format!("Failed to create cursor autocmd: {}", e)))?;
    
    Ok(())
}

/// Setup autocommand for buffer/window changes
///
/// Triggers on BufEnter and WinEnter events.
/// Sends visibleFilesDidChange notification with list of visible buffers.
fn setup_visible_files_autocmd(group_id: u32, hub: Arc<Hub>) -> Result<()> {
    let opts = CreateAutocmdOpts::builder()
        .group(group_id)
        .desc("Send visibleFilesDidChange notification on buffer/window changes")
        .callback(move |args: AutocmdCallbackArgs| {
            // Get visible files and send notification
            if let Err(e) = handle_visible_files_changed(&hub, &args) {
                eprintln!("Error in visible files handler: {}", e);
            }
            
            // Keep autocommand active (return false)
            Ok::<_, nvim_oxi::Error>(false)
        })
        .build();
    
    api::create_autocmd(["BufEnter", "WinEnter"], &opts)
        .map_err(|e| crate::errors::AmpError::Other(format!("Failed to create buffer autocmd: {}", e)))?;
    
    Ok(())
}

/// Handle cursor moved event
///
/// Gets current cursor position and selection, sends notification to clients.
fn handle_cursor_moved(hub: &Hub, _args: &AutocmdCallbackArgs) -> Result<()> {
    // Get current buffer and file path
    let buf = api::get_current_buf();
    let path = buf.get_name()
        .map_err(|e| crate::errors::AmpError::Other(format!("Failed to get buffer name: {}", e)))?;
    
    // Convert to file:// URI
    let uri = if path.is_absolute() {
        format!("file://{}", path.display())
    } else {
        // Handle unnamed/scratch buffers - skip notification
        return Ok(());
    };
    
    // Check if in visual mode
    let mode = api::get_mode()
        .map_err(|e| crate::errors::AmpError::Other(format!("Failed to get mode: {}", e)))?;
    
    let (start_line, start_char, end_line, end_char, content) = if mode.mode.is_visual() {
        // Visual mode - get actual selection
        match get_visual_selection(&buf) {
            Ok(Some((s_line, s_col, e_line, e_col, text))) => {
                // Marks are (1,0)-indexed, convert to 0-indexed
                let start_line = (s_line.saturating_sub(1)) as usize;
                let start_char = s_col as usize;
                let end_line = (e_line.saturating_sub(1)) as usize;
                let end_char = e_col as usize;
                (start_line, start_char, end_line, end_char, text)
            }
            Ok(None) | Err(_) => {
                // Fallback to cursor position if marks fail
                get_cursor_position()?
            }
        }
    } else {
        // Normal mode - send cursor position as zero-width selection
        get_cursor_position()?
    };
    
    notifications::send_selection_changed(
        hub,
        &uri,
        start_line,
        start_char,
        end_line,
        end_char,
        &content,
    );
    
    Ok(())
}

/// Get visual selection range and text
///
/// Returns (start_line, start_col, end_line, end_col, text) in (1,0)-indexed format
fn get_visual_selection(buf: &api::Buffer) -> Result<Option<(usize, usize, usize, usize, String)>> {
    // Get visual selection marks
    let (start_row, start_col) = buf.get_mark('<')
        .map_err(|e| crate::errors::AmpError::Other(format!("Failed to get mark '<': {}", e)))?;
    let (end_row, end_col) = buf.get_mark('>')
        .map_err(|e| crate::errors::AmpError::Other(format!("Failed to get mark '>': {}", e)))?;
    
    // Check if marks are valid (> 0)
    if start_row == 0 || end_row == 0 {
        return Ok(None);
    }
    
    // Convert to 0-indexed for get_text
    let start_row_0 = start_row.saturating_sub(1);
    let end_row_0 = end_row.saturating_sub(1);
    
    // Extract text (get_text uses 0-indexed, end-exclusive ranges)
    let text_lines: std::result::Result<Vec<String>, _> = buf
        .get_text(
            start_row_0..end_row_0 + 1,
            start_col,
            end_col + 1,  // end-exclusive
            &Default::default(),
        )
        .map(|iter| iter.map(|s| s.to_string_lossy().into()).collect());
    
    let text_lines = text_lines
        .map_err(|e| crate::errors::AmpError::Other(format!("Failed to get text: {}", e)))?;
    
    let selected_text = text_lines.join("\n");
    
    // Return (1,0)-indexed positions
    Ok(Some((start_row, start_col, end_row, end_col, selected_text)))
}

/// Get current cursor position as a zero-width selection
///
/// Returns (start_line, start_char, end_line, end_char, content) in 0-indexed format
fn get_cursor_position() -> Result<(usize, usize, usize, usize, String)> {
    let win = api::get_current_win();
    let (line, col) = win.get_cursor()
        .map_err(|e| crate::errors::AmpError::Other(format!("Failed to get cursor: {}", e)))?;
    
    // Cursor positions are 1-indexed in Neovim, convert to 0-indexed
    let start_line = (line.saturating_sub(1)) as usize;
    let start_char = col as usize;
    let end_line = start_line;
    let end_char = start_char;
    
    Ok((start_line, start_char, end_line, end_char, String::new()))
}

/// Handle visible files changed event
///
/// Gets list of all visible buffers, sends notification to clients.
fn handle_visible_files_changed(hub: &Hub, _args: &AutocmdCallbackArgs) -> Result<()> {
    // Get all windows
    let windows = api::list_wins();
    
    let mut uris = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();
    
    // For each window, get its buffer and file path
    for win in windows {
        if let Ok(buf) = win.get_buf() {
            if let Ok(path) = buf.get_name() {
                // Only include absolute paths (skip unnamed/scratch buffers)
                if path.is_absolute() {
                    let path_str = path.to_string_lossy().to_string();
                    
                    // Deduplicate - same file might be open in multiple windows
                    if !seen_paths.contains(&path_str) {
                        seen_paths.insert(path_str.clone());
                        
                        // Convert to file:// URI
                        let uri = format!("file://{}", path.display());
                        uris.push(uri);
                    }
                }
            }
        }
    }
    
    notifications::send_visible_files_changed(hub, uris);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_module_exists() {
        // Basic compilation test
        assert_eq!(AUGROUP_NAME, "AmpExtrasNotifications");
    }
    
    // Note: Actual autocommand tests require Neovim context
    // and should be run as integration tests
}
