//! IDE protocol operations - shared infrastructure
//!
//! Provides common functionality for IDE-specific methods that Amp CLI can call.
//! Individual operations are in separate modules.

use crossbeam_channel::{unbounded, Receiver, Sender};
use nvim_oxi::libuv::AsyncHandle;
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};

use crate::errors::{AmpError, Result};

// ============================================================================
// Module Exports
// ============================================================================

mod ping;
mod authenticate;
mod nvim_notify;
mod read_file;
mod edit_file;
mod get_diagnostics;

pub use ping::ping;
pub use authenticate::authenticate;
pub use nvim_notify::nvim_notify;
pub use read_file::read_file;
pub use edit_file::edit_file;
pub use get_diagnostics::get_diagnostics;

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
pub(super) fn nvim_available() -> bool {
    NVIM_READY.get().is_some()
}

// ============================================================================
// Global AsyncHandle and Notification Channel
// ============================================================================

/// Message to send to Neovim
#[derive(Debug, Clone)]
pub(super) struct NvimMessage {
    pub message: String,
}

/// Global channel for sending messages to Neovim
static NVIM_TX: OnceCell<Sender<NvimMessage>> = OnceCell::new();

/// Global AsyncHandle for triggering Neovim callbacks
static ASYNC_HANDLE: OnceCell<AsyncHandle> = OnceCell::new();

/// Work to schedule on Neovim main thread
type ScheduledWork = Box<dyn FnOnce() + Send>;

/// Global channel for scheduling work on Neovim main thread
static WORK_TX: OnceCell<Sender<ScheduledWork>> = OnceCell::new();

/// Global AsyncHandle for scheduled work
static WORK_HANDLE: OnceCell<AsyncHandle> = OnceCell::new();

/// Get the notification channel sender
///
/// Used by nvim_notify to send messages to Neovim
pub(super) fn get_nvim_tx() -> Option<&'static Sender<NvimMessage>> {
    NVIM_TX.get()
}

/// Get the AsyncHandle for notifications
///
/// Used by nvim_notify to trigger the callback
pub(super) fn get_async_handle() -> Option<&'static AsyncHandle> {
    ASYNC_HANDLE.get()
}

/// Initialize the AsyncHandle and message channel for IDE operations
///
/// Must be called before starting the server to enable nvim/notify.
pub fn init_async_handle() -> Result<()> {
    // Initialize notification channel
    let (tx, rx): (Sender<NvimMessage>, Receiver<NvimMessage>) = unbounded();

    // Create AsyncHandle with callback that processes messages from channel
    let handle = AsyncHandle::new(move || {
        use nvim_oxi::{api::{self, types::LogLevel}, Dictionary};
        
        // Process all pending messages
        while let Ok(msg) = rx.try_recv() {
            // Use type-safe api::notify instead of string-based exec
            let _ = api::notify(&msg.message, LogLevel::Info, &Dictionary::new());
        }
        Ok::<_, std::convert::Infallible>(())
    })
    .map_err(|e| AmpError::Other(format!("Failed to create AsyncHandle: {}", e)))?;

    let _ = NVIM_TX.set(tx);
    let _ = ASYNC_HANDLE.set(handle);

    // Initialize work scheduler channel
    let (work_tx, work_rx): (Sender<ScheduledWork>, Receiver<ScheduledWork>) = unbounded();

    // Create AsyncHandle for scheduled work
    let work_handle = AsyncHandle::new(move || {
        // Process all pending work
        while let Ok(work) = work_rx.try_recv() {
            work();
        }
        Ok::<_, std::convert::Infallible>(())
    })
    .map_err(|e| AmpError::Other(format!("Failed to create work AsyncHandle: {}", e)))?;

    let _ = WORK_TX.set(work_tx);
    let _ = WORK_HANDLE.set(work_handle);

    // Mark Neovim as ready for API calls
    mark_nvim_ready();

    Ok(())
}

/// Schedule work to run on Neovim's main thread
///
/// This is used to safely call Neovim APIs from background threads.
pub fn schedule_on_main_thread<F>(work: F) -> Result<()>
where
    F: FnOnce() + Send + 'static,
{
    if let Some(tx) = WORK_TX.get() {
        if let Some(handle) = WORK_HANDLE.get() {
            tx.send(Box::new(work))
                .map_err(|_| AmpError::Other("Failed to send work to scheduler".into()))?;
            handle.send()
                .map_err(|e| AmpError::Other(format!("Failed to trigger work handle: {}", e)))?;
            Ok(())
        } else {
            Err(AmpError::Other("Work handle not initialized".into()))
        }
    } else {
        Err(AmpError::Other("Work scheduler not initialized".into()))
    }
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
        let result: std::result::Result<nvim_oxi::Object, _> = api::call_function(
            "luaeval",
            (lua_expr, path),
        );

        if let Ok(obj) = result {
            use nvim_oxi::conversion::FromObject;
            if let Ok(normalized) = <String as FromObject>::from_object(obj) {
                return Ok(PathBuf::from(normalized));
            }
        }
    }

    // Fallback: use current_dir + relative path
    let current = std::env::current_dir()
        .map_err(|e| AmpError::IoError(e))?;
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
