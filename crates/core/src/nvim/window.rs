//! Window operation utilities
//!
//! Provides functions for working with Neovim windows.

use std::collections::HashSet;
use std::path::PathBuf;

use nvim_oxi::api;

use crate::errors::Result;

/// Get list of all visible buffer file paths
///
/// Returns absolute paths of files currently visible in any window.
/// Filters out:
/// - Unnamed/scratch buffers
/// - Non-existent files
/// - Duplicate paths (same file in multiple windows)
///
/// # Returns
/// Vector of unique absolute file paths currently visible
pub(crate) fn get_visible_buffers() -> Result<Vec<PathBuf>> {
    let windows = api::list_wins();
    let mut paths = Vec::new();
    let mut seen = HashSet::new();

    for win in windows {
        if let Ok(buf) = win.get_buf() {
            if let Ok(path) = buf.get_name() {
                // Only include absolute paths (skip unnamed/scratch buffers)
                if path.is_absolute() {
                    // Only include files that exist on filesystem
                    if !path.exists() {
                        continue;
                    }

                    let path_str = path.to_string_lossy().to_string();

                    // Deduplicate - same file might be open in multiple windows
                    if !seen.contains(&path_str) {
                        seen.insert(path_str);
                        paths.push(path);
                    }
                }
            }
        }
    }

    Ok(paths)
}

// Tests for this module are in tests-integration/src/
// since they require a running Neovim instance
