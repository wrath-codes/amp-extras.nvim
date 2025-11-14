//! FFI (Foreign Function Interface) layer for Lua ↔ Rust communication
//!
//! This module provides the boundary between Lua and Rust, handling:
//! - Command dispatch
//! - Autocomplete
//! - Error conversion to Lua-friendly formats

use nvim_oxi::{Dictionary, Object};
use serde_json::Value;

use crate::{
    commands,
    conversion::{json_to_object, object_to_json},
    errors::{AmpError, Result},
    server,
};

/// Main FFI entry point for command execution
///
/// Called from Lua as: `ffi.call(command, args)`
///
/// # Arguments
/// * `command` - Command name in format "category.action" (e.g.,
///   "threads.list")
/// * `args` - Command arguments as JSON object
///
/// # Returns
/// Result as JSON object, or error message
pub fn call(command: String, args: Object) -> nvim_oxi::Result<Object> {
    // Convert nvim-oxi Object to serde_json::Value
    let args_value = object_to_json(args)?;

    // Dispatch command
    match dispatch_command(&command, args_value) {
        Ok(result) => json_to_object(result),
        Err(err) => Ok(create_error_object(&err)),
    }
}

/// Autocomplete handler for @ mentions
///
/// Called from Lua as: `ffi.autocomplete(kind, prefix)`
///
/// # Arguments
/// * `kind` - Type of completion ("thread", "prompt", "file")
/// * `prefix` - User-typed prefix to filter by
///
/// # Returns
/// List of completion items
pub fn autocomplete(kind: String, prefix: String) -> nvim_oxi::Result<Vec<String>> {
    match autocomplete_impl(&kind, &prefix) {
        Ok(items) => Ok(items),
        Err(_err) => {
            // Silently return empty list (autocomplete should never fail visibly)
            Ok(vec![])
        },
    }
}

// ============================================================================
// WebSocket Server FFI
// ============================================================================

/// Start the WebSocket server
///
/// Called from Lua as: `ffi.server_start()`
///
/// Returns:
/// ```lua
/// {
///   success = true,
///   port = 12345,
///   token = "abc123...",
///   lockfile = "/path/to/lockfile.json"
/// }
/// ```
/// Or on error:
/// ```lua
/// {
///   error = true,
///   message = "Error description",
///   category = "error_type"
/// }
/// ```
pub fn server_start() -> nvim_oxi::Result<Object> {
    match server::start() {
        Ok((port, token, lockfile_path)) => {
            let result = Dictionary::from_iter([
                ("success", Object::from(true)),
                ("port", Object::from(port as i32)),
                ("token", Object::from(token)),
                (
                    "lockfile",
                    Object::from(lockfile_path.to_string_lossy().to_string()),
                ),
            ]);
            Ok(Object::from(result))
        },
        Err(err) => Ok(create_error_object(&err)),
    }
}

/// Stop the WebSocket server
///
/// Called from Lua as: `ffi.server_stop()`
///
/// Returns:
/// ```lua
/// { success = true }
/// ```
pub fn server_stop() -> nvim_oxi::Result<Object> {
    server::stop();

    let result = Dictionary::from_iter([("success", Object::from(true))]);
    Ok(Object::from(result))
}

/// Check if WebSocket server is running
///
/// Called from Lua as: `ffi.server_is_running()`
///
/// Returns:
/// ```lua
/// { running = true }
/// ```
pub fn server_is_running() -> nvim_oxi::Result<Object> {
    let result = Dictionary::from_iter([("running", Object::from(server::is_running()))]);
    Ok(Object::from(result))
}

/// Setup notification autocommands
///
/// Called from Lua as: `ffi.setup_notifications()`
///
/// Sets up autocommands that trigger WebSocket notifications:
/// - CursorMoved/CursorMovedI → selectionDidChange
/// - BufEnter/WinEnter → visibleFilesDidChange
///
/// Returns:
/// ```lua
/// { success = true }
/// ```
/// Or on error:
/// ```lua
/// {
///   error = true,
///   message = "Error description",
///   category = "error_type"
/// }
/// ```
pub fn setup_notifications() -> nvim_oxi::Result<Object> {
    // Get the Hub from the server (if running)
    match server::get_hub() {
        Some(hub) => match crate::autocmds::setup_notifications(hub) {
            Ok(()) => {
                let result = Dictionary::from_iter([("success", Object::from(true))]);
                Ok(Object::from(result))
            },
            Err(err) => Ok(create_error_object(&err)),
        },
        None => {
            let err = crate::errors::AmpError::Other("WebSocket server not running".into());
            Ok(create_error_object(&err))
        },
    }
}

/// Send selectionDidChange notification manually
///
/// Called from Lua as: `ffi.send_selection_changed(uri, start_line, start_char,
/// end_line, end_char, content)`
///
/// Returns:
/// ```lua
/// { success = true }
/// ```
/// Or on error:
/// ```lua
/// {
///   error = true,
///   message = "Error description"
/// }
/// ```
pub fn send_selection_changed(
    uri: String,
    start_line: i64,
    start_char: i64,
    end_line: i64,
    end_char: i64,
    content: String,
) -> nvim_oxi::Result<Object> {
    match server::get_hub() {
        Some(hub) => {
            match crate::notifications::send_selection_changed(
                &hub,
                &uri,
                start_line as usize,
                start_char as usize,
                end_line as usize,
                end_char as usize,
                &content,
            ) {
                Ok(()) => {
                    let result = Dictionary::from_iter([("success", Object::from(true))]);
                    Ok(Object::from(result))
                },
                Err(err) => Ok(create_error_object(&err)),
            }
        },
        None => {
            let err = crate::errors::AmpError::Other("WebSocket server not running".into());
            Ok(create_error_object(&err))
        },
    }
}

/// Send visibleFilesDidChange notification manually
///
/// Called from Lua as: `ffi.send_visible_files_changed(uris)`
///
/// Returns:
/// ```lua
/// { success = true }
/// ```
/// Or on error:
/// ```lua
/// {
///   error = true,
///   message = "Error description"
/// }
/// ```
pub fn send_visible_files_changed(uris: Vec<String>) -> nvim_oxi::Result<Object> {
    match server::get_hub() {
        Some(hub) => match crate::notifications::send_visible_files_changed(&hub, uris) {
            Ok(()) => {
                let result = Dictionary::from_iter([("success", Object::from(true))]);
                Ok(Object::from(result))
            },
            Err(err) => Ok(create_error_object(&err)),
        },
        None => {
            let err = crate::errors::AmpError::Other("WebSocket server not running".into());
            Ok(create_error_object(&err))
        },
    }
}

/// Send userSentMessage notification
///
/// Called from Lua as: `ffi.send_user_message(message)`
///
/// Sends user-typed message directly to the agent.
/// This immediately submits the message to Amp CLI.
///
/// Returns:
/// ```lua
/// { success = true }
/// ```
/// Or on error:
/// ```lua
/// {
///   error = true,
///   message = "Error description"
/// }
/// ```
pub fn send_user_message(message: String) -> nvim_oxi::Result<Object> {
    match server::get_hub() {
        Some(hub) => match crate::notifications::send_user_sent_message(&hub, &message) {
            Ok(()) => {
                let result = Dictionary::from_iter([("success", Object::from(true))]);
                Ok(Object::from(result))
            },
            Err(err) => Ok(create_error_object(&err)),
        },
        None => {
            let err = crate::errors::AmpError::Other("WebSocket server not running".into());
            Ok(create_error_object(&err))
        },
    }
}

/// Send appendToPrompt notification
///
/// Called from Lua as: `ffi.send_to_prompt(message)`
///
/// Appends text to the IDE prompt field without sending.
/// Allows user to edit before submitting.
///
/// Returns:
/// ```lua
/// { success = true }
/// ```
/// Or on error:
/// ```lua
/// {
///   error = true,
///   message = "Error description"
/// }
/// ```
pub fn send_to_prompt(message: String) -> nvim_oxi::Result<Object> {
    match server::get_hub() {
        Some(hub) => match crate::notifications::send_append_to_prompt(&hub, &message) {
            Ok(()) => {
                let result = Dictionary::from_iter([("success", Object::from(true))]);
                Ok(Object::from(result))
            },
            Err(err) => Ok(create_error_object(&err)),
        },
        None => {
            let err = crate::errors::AmpError::Other("WebSocket server not running".into());
            Ok(create_error_object(&err))
        },
    }
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Internal command dispatcher
///
/// Delegates to the command registry for actual command execution.
fn dispatch_command(command: &str, args: Value) -> Result<Value> {
    commands::dispatch(command, args)
}

/// Internal autocomplete implementation
fn autocomplete_impl(_kind: &str, _prefix: &str) -> Result<Vec<String>> {
    // Placeholder - will be implemented with actual data sources
    Ok(vec![])
}

/// Create a structured error object for Lua
///
/// Returns a Dictionary with fields:
/// - `error`: true (marker that this is an error response)
/// - `message`: user-friendly error message
/// - `category`: error category for logging/handling
fn create_error_object(err: &AmpError) -> Object {
    let error_dict = Dictionary::from_iter([
        ("error", Object::from(true)),
        ("message", Object::from(err.user_message())),
        ("category", Object::from(err.category())),
    ]);
    Object::from(error_dict)
}

#[cfg(test)]
mod tests {
    use nvim_oxi::conversion::FromObject;

    use super::*;

    // ========================================
    // call() function tests
    // ========================================

    #[test]
    fn test_call_unknown_command_returns_error_object() {
        let command = "unknown.command".to_string();
        let args = Object::from(Dictionary::new());

        let result = call(command.clone(), args);
        assert!(result.is_ok(), "call() should return Ok with error object");

        let obj = result.unwrap();
        let dict = Dictionary::from_object(obj.clone()).unwrap();

        // Verify error object structure
        let err = <bool as FromObject>::from_object(dict.get("error").unwrap().clone()).unwrap();
        assert!(err, "error field should be true");

        let msg =
            <String as FromObject>::from_object(dict.get("message").unwrap().clone()).unwrap();
        assert!(
            msg.contains("unknown.command"),
            "message should contain command name"
        );

        let cat =
            <String as FromObject>::from_object(dict.get("category").unwrap().clone()).unwrap();
        assert_eq!(cat, "command", "category should be 'command'");
    }

    #[test]
    fn test_call_with_empty_args() {
        let command = "test.command".to_string();
        let args = Object::from(Dictionary::new());

        let result = call(command, args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_call_with_complex_args() {
        let command = "test.command".to_string();
        let mut args_dict = Dictionary::new();
        args_dict.insert("key1", "value1");
        args_dict.insert("key2", 42);
        args_dict.insert("key3", true);

        let result = call(command, Object::from(args_dict));
        assert!(result.is_ok());
    }

    // ========================================
    // autocomplete() function tests
    // ========================================

    #[test]
    fn test_autocomplete_never_panics() {
        let result = autocomplete("invalid".to_string(), "test".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_autocomplete_returns_empty_list() {
        let result = autocomplete("thread".to_string(), "prefix".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Vec::<String>::new());
    }

    #[test]
    fn test_autocomplete_with_empty_prefix() {
        let result = autocomplete("thread".to_string(), "".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_autocomplete_with_empty_kind() {
        let result = autocomplete("".to_string(), "prefix".to_string());
        assert!(result.is_ok());
    }

    // ========================================
    // dispatch_command() tests
    // ========================================

    #[test]
    fn test_dispatch_unknown_command() {
        let args = serde_json::json!({});
        let result = dispatch_command("unknown.command", args);
        assert!(result.is_err());
        if let Err(AmpError::CommandNotFound(cmd)) = result {
            assert_eq!(cmd, "unknown.command");
        } else {
            panic!("Expected CommandNotFound error");
        }
    }

    #[test]
    fn test_dispatch_with_complex_args() {
        let args = serde_json::json!({
            "string": "value",
            "number": 42,
            "boolean": true,
            "array": [1, 2, 3],
            "object": {"nested": "data"}
        });
        let result = dispatch_command("test.command", args);
        assert!(result.is_err());
    }

    // ========================================
    // autocomplete_impl() tests
    // ========================================

    #[test]
    fn test_autocomplete_impl_returns_empty() {
        let result = autocomplete_impl("thread", "T-");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Vec::<String>::new());
    }

    // ========================================
    // create_error_object() tests
    // ========================================

    #[test]
    fn test_create_error_object_structure() {
        let err = AmpError::CommandNotFound("test.command".to_string());
        let obj = create_error_object(&err);
        let dict = Dictionary::from_object(obj).unwrap();

        let error_flag =
            <bool as FromObject>::from_object(dict.get("error").unwrap().clone()).unwrap();
        assert!(error_flag);

        let msg =
            <String as FromObject>::from_object(dict.get("message").unwrap().clone()).unwrap();
        assert!(msg.contains("test.command"));

        let cat =
            <String as FromObject>::from_object(dict.get("category").unwrap().clone()).unwrap();
        assert_eq!(cat, "command");
    }
}
