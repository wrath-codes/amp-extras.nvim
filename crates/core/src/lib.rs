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
pub mod autocmds;
pub mod commands;
pub mod errors;
pub mod ffi;
pub mod ide_ops;
pub mod lockfile;
pub mod notifications;
pub mod rpc;
pub mod server;
pub mod util;

use nvim_oxi::{Dictionary, Function, Object};

// Wrapper functions for complex signatures to help type inference
fn send_selection_changed_wrapper(
    (uri, start_line, start_char, end_line, end_char, content): (String, i64, i64, i64, i64, String)
) -> Object {
    ffi::send_selection_changed(uri, start_line, start_char, end_line, end_char, content).unwrap()
}

fn send_visible_files_changed_wrapper(uris: Vec<String>) -> Object {
    ffi::send_visible_files_changed(uris).unwrap()
}

fn send_user_message_wrapper(message: String) -> Object {
    ffi::send_user_message(message).unwrap()
}

fn send_to_prompt_wrapper(message: String) -> Object {
    ffi::send_to_prompt(message).unwrap()
}

/// Plugin entry point - called when Neovim loads the plugin
///
/// This function is invoked by nvim-oxi and registers all FFI exports
/// that Lua code can call.
/// 
/// The function name determines the exported symbol: amp_extras_core -> luaopen_amp_extras_core
#[nvim_oxi::plugin]
fn amp_extras_core() -> nvim_oxi::Result<Dictionary> {
    // Create FFI exports dictionary with explicit type parameters
    let mut exports = Dictionary::new();
    
    exports.insert("call", Function::<(String, Object), Object>::from_fn(|(command, args): (String, Object)| ffi::call(command, args)));
    exports.insert("autocomplete", Function::<(String, String), Vec<String>>::from_fn(|(kind, prefix): (String, String)| ffi::autocomplete(kind, prefix)));
    exports.insert("server_start", Function::<(), Object>::from_fn(|()| ffi::server_start()));
    exports.insert("server_stop", Function::<(), Object>::from_fn(|()| ffi::server_stop()));
    exports.insert("server_is_running", Function::<(), Object>::from_fn(|()| ffi::server_is_running()));
    exports.insert("setup_notifications", Function::<(), Object>::from_fn(|()| ffi::setup_notifications()));
    exports.insert("send_selection_changed", Function::<(String, i64, i64, i64, i64, String), Object>::from_fn(send_selection_changed_wrapper));
    exports.insert("send_visible_files_changed", Function::<Vec<String>, Object>::from_fn(send_visible_files_changed_wrapper));
    exports.insert("send_user_message", Function::<String, Object>::from_fn(send_user_message_wrapper));
    exports.insert("send_to_prompt", Function::<String, Object>::from_fn(send_to_prompt_wrapper));

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
