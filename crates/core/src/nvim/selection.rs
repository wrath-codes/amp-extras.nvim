//! Visual selection and mark utilities
//!
//! Provides functions for working with visual selections and marks.

use nvim_oxi::api::{self, types::ModeStr, Buffer};

use crate::errors::Result;

/// Type alias for selection range
/// (start_line, start_col, end_line, end_col, text)
pub(crate) type SelectionRange = (usize, usize, usize, usize, String);

/// Get visual selection range and text
///
/// Handles different visual mode types:
/// - `v` (character-wise): Uses mark positions as-is
/// - `V` (line-wise): Extends end column to end of line
/// - `Ctrl-V` (block): Currently treats as character-wise (protocol limitation)
///
/// # Arguments
/// * `buf` - Buffer containing the selection
/// * `mode_str` - Current mode string from `api::get_mode()`
///
/// # Returns
/// `Ok(Some((start_line, start_col, end_line, end_col, text)))` where:
/// - start_line, end_line are 1-indexed (Neovim convention)
/// - start_col, end_col are 0-indexed (Neovim convention)
/// - text is the selected content
///
/// Returns `Ok(None)` if marks are invalid (e.g., no active selection)
pub(crate) fn get_visual_selection(
    buf: &Buffer,
    mode_str: &ModeStr,
) -> Result<Option<SelectionRange>> {
    // Get visual selection marks
    let (start_row, start_col) = buf
        .get_mark('<')
        .map_err(|e| crate::errors::AmpError::Other(format!("Failed to get mark '<': {}", e)))?;
    let (end_row, end_col) = buf
        .get_mark('>')
        .map_err(|e| crate::errors::AmpError::Other(format!("Failed to get mark '>': {}", e)))?;

    // Check if marks are valid (> 0)
    if start_row == 0 || end_row == 0 {
        return Ok(None);
    }

    // Normalize selection direction - user can select backwards (end before start)
    let (start_row, mut start_col, end_row, mut end_col) =
        if start_row > end_row || (start_row == end_row && start_col > end_col) {
            // User selected backwards - swap start and end
            (end_row, end_col, start_row, start_col)
        } else {
            (start_row, start_col, end_row, end_col)
        };

    // For line-wise visual (V), extend selection to span entire lines
    if mode_str.is_visual_by_line() {
        // Start at beginning of first line
        start_col = 0;

        // End at end of last line
        let end_row_0 = end_row.saturating_sub(1);
        if let Ok(lines) = buf.get_lines(end_row_0..end_row_0 + 1, false) {
            if let Some(line) = lines.into_iter().next() {
                let line_str = line.to_string_lossy();
                end_col = line_str.len();
            }
        }
    }
    // Note: Block visual mode (Ctrl-V) is treated as character-wise
    // The amp.nvim protocol only supports single selection ranges

    // Convert to 0-indexed for get_text
    let start_row_0 = start_row.saturating_sub(1);
    let end_row_0 = end_row.saturating_sub(1);

    // Extract text (get_text uses 0-indexed, end-exclusive ranges)
    let text_lines: std::result::Result<Vec<String>, _> = buf
        .get_text(
            start_row_0..end_row_0 + 1,
            start_col,
            end_col + 1, // end-exclusive
            &Default::default(),
        )
        .map(|iter| iter.map(|s| s.to_string_lossy().into()).collect());

    let text_lines = text_lines
        .map_err(|e| crate::errors::AmpError::Other(format!("Failed to get text: {}", e)))?;

    let selected_text = text_lines.join("\n");

    // Return (1,0)-indexed positions
    Ok(Some((
        start_row,
        start_col,
        end_row,
        end_col,
        selected_text,
    )))
}

/// Get current mode information
///
/// Returns the current Neovim mode.
///
/// # Returns
/// - `Ok(mode)` - Mode information including mode string
/// - `Err(_)` - Failed to get mode
pub(crate) fn get_mode() -> Result<api::types::GotMode> {
    api::get_mode()
        .map_err(|e| crate::errors::AmpError::Other(format!("Failed to get mode: {}", e)))
}

// Tests for this module are in tests-integration/src/
// since they require a running Neovim instance
