//! amp-extras-rs: Neovim plugin for Amp CLI integration
//!
//! This plugin provides a rich Neovim interface to the Amp CLI tool,
//! with features including:
//! - Thread management (list, search, archive, delete)
//! - Prompt library (create, edit, search with FTS)
//! - Permission management (visual editor)
//! - MCP server integration
//! - Autocomplete for @-mentions
//!
//! ## Architecture
//!
//! - **Rust (nvim-oxi)**: Business logic, SQLite storage, CLI integration
//! - **Lua (nui-components)**: UI components, screens, command registration
//! - **Hybrid storage**: Local JSON files for threads, SQLite for prompts
//!
//! See ARCHITECTURE.md for complete documentation.

// Module declarations
pub mod commands;
pub mod errors;
pub mod ffi;

use nvim_oxi::{Dictionary, Function, Object};

/// Plugin entry point - called when Neovim loads the plugin
///
/// This function is invoked by nvim-oxi and registers all FFI exports
/// that Lua code can call.
#[nvim_oxi::plugin]
fn amp_extras() -> nvim_oxi::Result<Dictionary> {
    // Create FFI exports dictionary
    let exports = Dictionary::from_iter([
        // Main command dispatcher
        (
            "call",
            Object::from(
                Function::<(String, Object), nvim_oxi::Result<Object>>::from_fn(
                    |(command, args): (String, Object)| ffi::call(command, args),
                ),
            ),
        ),
        // Autocomplete handler
        (
            "autocomplete",
            Object::from(
                Function::<(String, String), nvim_oxi::Result<Vec<String>>>::from_fn(
                    |(kind, prefix): (String, String)| ffi::autocomplete(kind, prefix),
                ),
            ),
        ),
    ]);

    Ok(exports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modules_exist() {
        // Ensure modules compile and are accessible
        let _error: errors::AmpError = "test".into();
    }
}
