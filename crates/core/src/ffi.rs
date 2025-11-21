//! FFI (Foreign Function Interface) layer for Lua ↔ Rust communication
//!
//! This module provides the boundary between Lua and Rust, handling:
//! - Command dispatch
//! - Autocomplete
//! - Error conversion to Lua-friendly formats

use std::sync::OnceLock;

use nvim_oxi::{serde::Deserializer, Dictionary, Object};
use serde::Deserialize;
use serde_json::Value;

use crate::{commands, errors::{AmpError, Result}, db::Db, runtime};

/// Plugin configuration
#[derive(Debug, Clone, Deserialize)]
struct Config {
    // Add configuration fields here if needed in the future
    // Previously had auto_start for server
}

impl Default for Config {
    fn default() -> Self {
        Self {}
    }
}

/// Global config storage
static CONFIG: OnceLock<Config> = OnceLock::new();

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
    // Convert nvim-oxi Object to serde_json::Value using serde
    let args_value: Value = Value::deserialize(Deserializer::new(args))
        .map_err(nvim_oxi::Error::Deserialize)?;

    // Dispatch command
    match dispatch_command(&command, args_value) {
        Ok(result) => {
            // Convert serde_json::Value back to nvim-oxi Object
            use nvim_oxi::serde::Serializer;
            use serde::Serialize;
            result.serialize(Serializer::new())
                .map_err(nvim_oxi::Error::Serialize)
        },
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
// Plugin Setup
// ============================================================================

/// Setup the plugin with configuration
///
/// Called from Lua as: `ffi.setup({})`
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
pub fn setup(config_obj: Object) -> nvim_oxi::Result<Object> {
    // Deserialize config from Lua
    let config: Config = Config::deserialize(Deserializer::new(config_obj))
        .unwrap_or_default();

    // Store config (first call wins)
    let _ = CONFIG.set(config);

    // Initialize Database
    // Use XDG_CONFIG_HOME or ~/.config style path
    // On macOS, dirs::config_dir defaults to Application Support, but we prefer ~/.config
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .ok() // Convert Result to Option
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let db_path = config_dir.join("amp-extras/prompts.db");
    
    let db_path_str = db_path.to_str().unwrap_or("prompts.db");

    if let Err(e) = runtime::block_on(Db::init(db_path_str)) {
         return Ok(create_error_object(&e));
    }

    let result = Dictionary::from_iter([("success", Object::from(true))]);
    Ok(Object::from(result))
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
