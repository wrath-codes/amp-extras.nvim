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
pub mod db;
pub mod errors;
pub mod ffi;
pub mod runtime;

use nvim_oxi::{
    Dictionary, Function, Object,
};

/// Register Neovim user commands
fn register_commands() -> nvim_oxi::Result<()> {
    // No Rust-based user commands to register for now.
    // All send commands are now in Lua.
    // Server commands are removed.
    Ok(())
}

/// Plugin entry point - called when Neovim loads the plugin
///
/// This function is invoked by nvim-oxi and registers all FFI exports
/// that Lua code can call.
///
/// The function name determines the exported symbol: amp_extras_core ->
/// luaopen_amp_extras_core
#[nvim_oxi::plugin]
fn amp_extras_core() -> nvim_oxi::Result<Dictionary> {
    // Register user commands
    register_commands()?;

    // Create FFI exports dictionary with explicit type parameters
    let mut exports = Dictionary::new();

    exports.insert(
        "call",
        Function::<(String, Object), Object>::from_fn(|(command, args): (String, Object)| {
            ffi::call(command, args)
        }),
    );
    exports.insert(
        "autocomplete",
        Function::<(String, String), Vec<String>>::from_fn(|(kind, prefix): (String, String)| {
            ffi::autocomplete(kind, prefix)
        }),
    );
    exports.insert(
        "setup",
        Function::<Object, Object>::from_fn(|config| ffi::setup(config)),
    );

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