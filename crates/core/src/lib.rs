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
pub mod db;
pub mod errors;
pub mod ffi;
pub mod ide_ops;
pub mod lockfile;
pub mod notifications;
pub mod nvim;
pub mod rpc;
pub mod server;
pub mod util;

use nvim_oxi::{
    api,
    api::{
        opts::CreateCommandOpts,
        types::{CommandArgs, CommandRange},
    },
    Dictionary, Function, Object,
};

// Wrapper functions for complex signatures to help type inference
fn send_selection_changed_wrapper(
    (uri, start_line, start_char, end_line, end_char, content): (
        String,
        i64,
        i64,
        i64,
        i64,
        String,
    ),
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

/// Register Neovim user commands
fn register_commands() -> nvim_oxi::Result<()> {
    use serde_json::json;

    // Send file reference
    let opts = CreateCommandOpts::builder()
        .desc("Send file reference to Amp prompt (@file.rs)")
        .build();
    api::create_user_command(
        "AmpSendFileRef",
        |_| -> nvim_oxi::Result<()> {
            match commands::dispatch("send_file_ref", json!({})) {
                Ok(_) => Ok(()),
                Err(e) => {
                    api::err_writeln(&format!("AmpSendFileRef error: {}", e));
                    Ok(())
                },
            }
        },
        &opts,
    )?;

    // Send line reference
    let opts = CreateCommandOpts::builder()
        .desc("Send current line reference to Amp prompt (@file.rs#L10)")
        .build();
    api::create_user_command(
        "AmpSendLineRef",
        |_| -> nvim_oxi::Result<()> {
            match commands::dispatch("send_line_ref", json!({})) {
                Ok(_) => Ok(()),
                Err(e) => {
                    api::err_writeln(&format!("AmpSendLineRef error: {}", e));
                    Ok(())
                },
            }
        },
        &opts,
    )?;

    // Send entire buffer
    let opts = CreateCommandOpts::builder()
        .desc("Send entire buffer content to Amp prompt")
        .build();
    api::create_user_command(
        "AmpSendBuffer",
        |_| -> nvim_oxi::Result<()> {
            match commands::dispatch("send_buffer", json!({})) {
                Ok(_) => Ok(()),
                Err(e) => {
                    api::err_writeln(&format!("AmpSendBuffer error: {}", e));
                    Ok(())
                },
            }
        },
        &opts,
    )?;

    // Send selection (range command)
    let opts = CreateCommandOpts::builder()
        .desc("Send selected text to Amp prompt")
        .range(CommandRange::CurrentLine)
        .build();
    api::create_user_command(
        "AmpSendSelection",
        |args: CommandArgs| -> nvim_oxi::Result<()> {
            let start_line = args.line1;
            let end_line = args.line2;

            match commands::dispatch(
                "send_selection",
                json!({
                    "start_line": start_line,
                    "end_line": end_line
                }),
            ) {
                Ok(_) => Ok(()),
                Err(e) => {
                    api::err_writeln(&format!("AmpSendSelection error: {}", e));
                    Ok(())
                },
            }
        },
        &opts,
    )?;

    // Send selection reference (range command)
    let opts = CreateCommandOpts::builder()
        .desc("Send file reference with line range to Amp prompt (@file.rs#L10-L20)")
        .range(CommandRange::CurrentLine)
        .build();
    api::create_user_command(
        "AmpSendSelectionRef",
        |args: CommandArgs| -> nvim_oxi::Result<()> {
            let start_line = args.line1;
            let end_line = args.line2;

            match commands::dispatch(
                "send_selection_ref",
                json!({
                    "start_line": start_line,
                    "end_line": end_line
                }),
            ) {
                Ok(_) => Ok(()),
                Err(e) => {
                    api::err_writeln(&format!("AmpSendSelectionRef error: {}", e));
                    Ok(())
                },
            }
        },
        &opts,
    )?;

    // Server commands
    let opts = CreateCommandOpts::builder()
        .desc("Start the Amp WebSocket server")
        .build();
    api::create_user_command(
        "AmpServerStart",
        |_| -> nvim_oxi::Result<()> {
            let _ = ffi::server_start();
            // Notification already sent by ffi layer
            Ok(())
        },
        &opts,
    )?;

    let opts = CreateCommandOpts::builder()
        .desc("Stop the Amp WebSocket server")
        .build();
    api::create_user_command(
        "AmpServerStop",
        |_| -> nvim_oxi::Result<()> {
            let _ = ffi::server_stop();
            Ok(())
        },
        &opts,
    )?;

    let opts = CreateCommandOpts::builder()
        .desc("Check Amp WebSocket server status")
        .build();
    api::create_user_command(
        "AmpServerStatus",
        |_| -> nvim_oxi::Result<()> {
            let _ = ffi::server_is_running();
            Ok(())
        },
        &opts,
    )?;

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
        "server_start",
        Function::<(), Object>::from_fn(|()| ffi::server_start()),
    );
    exports.insert(
        "server_stop",
        Function::<(), Object>::from_fn(|()| ffi::server_stop()),
    );
    exports.insert(
        "server_is_running",
        Function::<(), Object>::from_fn(|()| ffi::server_is_running()),
    );
    exports.insert(
        "setup_notifications",
        Function::<(), Object>::from_fn(|()| ffi::setup_notifications()),
    );
    exports.insert(
        "send_selection_changed",
        Function::<(String, i64, i64, i64, i64, String), Object>::from_fn(
            send_selection_changed_wrapper,
        ),
    );
    exports.insert(
        "send_visible_files_changed",
        Function::<Vec<String>, Object>::from_fn(send_visible_files_changed_wrapper),
    );
    exports.insert(
        "send_user_message",
        Function::<String, Object>::from_fn(send_user_message_wrapper),
    );
    exports.insert(
        "send_to_prompt",
        Function::<String, Object>::from_fn(send_to_prompt_wrapper),
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
