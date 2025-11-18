//! IDE protocol operations - shared infrastructure
//!
//! Provides common functionality for IDE-specific methods that Amp CLI can
//! call. Individual operations are in separate modules.

use std::path::{Path, PathBuf};

use once_cell::sync::OnceCell;

use crate::errors::{AmpError, Result};

// ============================================================================
// Module Exports
// ============================================================================

mod authenticate;
mod edit_file;
mod get_diagnostics;
mod nvim_notify;
mod ping;
mod read_file;

pub use authenticate::authenticate;
pub use edit_file::edit_file;
pub use get_diagnostics::get_diagnostics;
pub use nvim_notify::nvim_notify;
pub use ping::ping;
pub use read_file::read_file;

// Internal re-exports for autocmds module
pub(crate) use get_diagnostics::NvimDiagnostic;

// ============================================================================
// Neovim Context Detection
// ============================================================================

/// Flag indicating Neovim is initialized and ready for API calls
static NVIM_READY: OnceCell<()> = OnceCell::new();

/// Mark Neovim as ready for API calls
///
/// Should be called during plugin initialization (after AsyncHandle setup)
pub fn mark_nvim_ready() {
    let _ = NVIM_READY.set(());
}

/// Check if Neovim API is available
///
/// Returns true if we're running inside Neovim and API calls are safe
pub(crate) fn nvim_available() -> bool {
    NVIM_READY.get().is_some()
}

// ============================================================================
// Main Thread Scheduling
// ============================================================================

/// Schedule work to run on Neovim's main thread
///
/// Uses nvim-oxi's built-in schedule() function for cross-thread safety.
/// This is safe to call from any thread.
pub fn schedule_on_main_thread<F>(work: F) -> Result<()>
where
    F: FnOnce() + Send + 'static,
{
    nvim_oxi::schedule(move |_| {
        work();
        Ok::<_, std::convert::Infallible>(())
    });
    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Find a Neovim buffer by its file path
///
/// Searches all buffers for one matching the given absolute path.
/// Prefers loaded buffers over unloaded ones.
///
/// # Arguments
/// * `path` - Absolute file path to search for
///
/// # Returns
/// * `Some(Buffer)` if found (preferring loaded buffers)
/// * `None` if no buffer matches the path
#[cfg(not(test))]
pub(super) fn find_buffer_by_path(path: &Path) -> Option<nvim_oxi::api::Buffer> {
    // Use centralized buffer utilities
    crate::nvim::buffer::find_by_path(path).ok().flatten()
}

/// Normalize a path to absolute form
///
/// Uses Neovim's fnamemodify to expand relative paths.
/// Falls back to returning the path as-is if Neovim is not available.
///
/// # Arguments
/// * `path` - Path to normalize (can be relative or absolute)
///
/// # Returns
/// * Normalized absolute path
pub(super) fn normalize_path(path: &str) -> Result<PathBuf> {
    let path_buf = PathBuf::from(path);

    // If already absolute, return as-is
    if path_buf.is_absolute() {
        return Ok(path_buf);
    }

    // Try to normalize using Neovim if available
    #[cfg(not(test))]
    if nvim_available() {
        use nvim_oxi::api;

        // Use vim.fn.fnamemodify(path, ':p') to get absolute path
        let lua_expr = "vim.fn.fnamemodify(args, ':p')";
        let result: std::result::Result<nvim_oxi::Object, _> =
            api::call_function("luaeval", (lua_expr, path));

        if let Ok(obj) = result {
            use nvim_oxi::conversion::FromObject;
            if let Ok(normalized) = <String as FromObject>::from_object(obj) {
                return Ok(PathBuf::from(normalized));
            }
        }
    }

    // Fallback: use current_dir + relative path
    let current = std::env::current_dir().map_err(AmpError::IoError)?;
    Ok(current.join(path_buf))
}

/// Map Neovim diagnostic severity to string
///
/// Neovim severity levels:
/// - 1 = ERROR
/// - 2 = WARN
/// - 3 = INFO
/// - 4 = HINT
pub(super) fn map_severity(severity: Option<u8>) -> &'static str {
    match severity.unwrap_or(3) {
        1 => "ERROR",
        2 => "WARNING",
        3 => "INFO",
        4 => "HINT",
        _ => "INFO", // Default to info for unknown values
    }
}

/// Get line content for a diagnostic
///
/// Tries to read from buffer if loaded, falls back to disk
pub(super) fn get_line_content(path: &Path, line_num: u32) -> String {
    use std::fs;

    // Try buffer first (using nvim::buffer utilities)
    let line_from_buffer = crate::nvim::buffer::get_line_content(path, line_num as usize);
    if !line_from_buffer.is_empty() {
        return line_from_buffer;
    }

    // Fallback to disk
    if let Ok(content) = fs::read_to_string(path) {
        if let Some(line) = content.lines().nth(line_num as usize) {
            return line.to_string();
        }
    }

    String::new()
}
