//! Cursor position utilities
//!
//! Provides functions for getting and working with cursor positions.

use nvim_oxi::api;

use crate::errors::Result;

/// Get current cursor position as (line, col) in 0-indexed format
///
/// Neovim returns (1, 0)-indexed positions; this converts to pure 0-indexed.
///
/// # Returns
/// - `Ok((line, col))` - 0-indexed cursor position
/// - `Err(_)` - If cursor position cannot be retrieved
///
/// # Example
/// ```rust,ignore
/// let (line, col) = cursor::get_position()?;
/// println!("Cursor at line {}, column {}", line, col);
/// ```
pub(crate) fn get_position() -> Result<(usize, usize)> {
    let win = api::get_current_win();
    let (line, col) = win
        .get_cursor()
        .map_err(|e| crate::errors::AmpError::Other(format!("Failed to get cursor: {}", e)))?;

    // Cursor positions are (1, 0)-indexed in Neovim, convert to 0-indexed
    let line_0 = (line.saturating_sub(1)) as usize;
    let col_0 = col as usize;

    Ok((line_0, col_0))
}

/// Get current cursor position as a zero-width selection range
///
/// Returns cursor position in the format expected by selectionDidChange:
/// (start_line, start_char, end_line, end_char, content)
///
/// Useful for sending cursor position as a selection to Amp CLI.
///
/// # Returns
/// Tuple of (start_line, start_char, end_line, end_char, content) where:
/// - All positions are 0-indexed
/// - start == end (zero-width selection)
/// - content is empty string
pub(crate) fn get_position_as_range() -> Result<(usize, usize, usize, usize, String)> {
    let (line, col) = get_position()?;
    Ok((line, col, line, col, String::new()))
}

// Tests for this module are in tests-integration/src/
// since they require a running Neovim instance
