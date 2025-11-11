//! FFI (Foreign Function Interface) layer for Lua ↔ Rust communication
//!
//! This module provides the boundary between Lua and Rust, handling:
//! - Command dispatch
//! - Autocomplete
//! - Error conversion to Lua-friendly formats

use nvim_oxi::serde::{Deserializer, Serializer};
use nvim_oxi::{Dictionary, Object};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::commands;
use crate::errors::{AmpError, Result};

/// Main FFI entry point for command execution
///
/// Called from Lua as: `ffi.call(command, args)`
///
/// # Arguments
/// * `command` - Command name in format "category.action" (e.g., "threads.list")
/// * `args` - Command arguments as JSON object
///
/// # Returns
/// Result as JSON object, or error message
pub fn call(command: String, args: Object) -> nvim_oxi::Result<Object> {
    // Convert nvim-oxi Object to serde_json::Value
    let args_value = object_to_value(args)?;

    // Dispatch command
    match dispatch_command(&command, args_value) {
        Ok(result) => value_to_object(result),
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
        Err(err) => {
            // Log error but return empty list (autocomplete should never fail visibly)
            eprintln!("Autocomplete error: {}", err);
            Ok(vec![])
        }
    }
}

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

/// Convert nvim-oxi Object to serde_json::Value
///
/// Uses nvim-oxi's Deserializer to convert Object → serde types
fn object_to_value(obj: Object) -> nvim_oxi::Result<Value> {
    let deserializer = Deserializer::new(obj);
    Value::deserialize(deserializer).map_err(nvim_oxi::Error::Deserialize)
}

/// Convert serde_json::Value to nvim-oxi Object
///
/// Uses nvim-oxi's Serializer to convert serde types → Object
fn value_to_object(value: Value) -> nvim_oxi::Result<Object> {
    value
        .serialize(Serializer::new())
        .map_err(nvim_oxi::Error::Serialize)
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
    use super::*;
    use nvim_oxi::conversion::FromObject;

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
        assert_eq!(err, true, "error field should be true");

        let msg = <String as FromObject>::from_object(dict.get("message").unwrap().clone()).unwrap();
        assert!(
            msg.contains("unknown.command"),
            "message should contain command name"
        );

        let cat = <String as FromObject>::from_object(dict.get("category").unwrap().clone()).unwrap();
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
    // Conversion function tests
    // ========================================

    #[test]
    fn test_value_to_object_string() {
        let value = serde_json::json!("test string");
        let result = value_to_object(value);
        assert!(result.is_ok());
        let obj = result.unwrap();
        let s = <String as FromObject>::from_object(obj).unwrap();
        assert_eq!(s, "test string");
    }

    #[test]
    fn test_value_to_object_number() {
        let value = serde_json::json!(42);
        let result = value_to_object(value);
        assert!(result.is_ok());
        let obj = result.unwrap();
        let n = <i64 as FromObject>::from_object(obj).unwrap();
        assert_eq!(n, 42);
    }

    #[test]
    fn test_value_to_object_boolean() {
        let value = serde_json::json!(true);
        let result = value_to_object(value);
        assert!(result.is_ok());
        let obj = result.unwrap();
        let b = <bool as FromObject>::from_object(obj).unwrap();
        assert_eq!(b, true);
    }

    #[test]
    fn test_value_to_object_null() {
        let value = serde_json::json!(null);
        let result = value_to_object(value);
        assert!(result.is_ok());
        assert!(result.unwrap().is_nil());
    }

    #[test]
    fn test_value_to_object_array() {
        let value = serde_json::json!([1, 2, 3]);
        let result = value_to_object(value);
        assert!(result.is_ok());
        let obj = result.unwrap();
        let array = <Vec<i64> as FromObject>::from_object(obj).unwrap();
        assert_eq!(array, vec![1, 2, 3]);
    }

    #[test]
    fn test_value_to_object_dict() {
        let value = serde_json::json!({"key": "value"});
        let result = value_to_object(value);
        assert!(result.is_ok());
        let obj = result.unwrap();
        let dict = Dictionary::from_object(obj).unwrap();
        let v = <String as FromObject>::from_object(dict.get("key").unwrap().clone()).unwrap();
        assert_eq!(v, "value");
    }

    #[test]
    fn test_value_to_object_nested() {
        let value = serde_json::json!({
            "nested": {
                "array": [1, 2, 3],
                "string": "test"
            }
        });
        let result = value_to_object(value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_object_to_value_string() {
        let obj = Object::from("test string");
        let result = object_to_value(obj);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, serde_json::json!("test string"));
    }

    #[test]
    fn test_object_to_value_number() {
        let obj = Object::from(42);
        let result = object_to_value(obj);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, serde_json::json!(42));
    }

    #[test]
    fn test_object_to_value_boolean() {
        let obj = Object::from(true);
        let result = object_to_value(obj);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, serde_json::json!(true));
    }

    #[test]
    fn test_object_to_value_nil() {
        let obj = Object::nil();
        let result = object_to_value(obj);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, serde_json::json!(null));
    }

    #[test]
    fn test_object_to_value_dict() {
        let mut dict = Dictionary::new();
        dict.insert("key", "value");
        let obj = Object::from(dict);
        let result = object_to_value(obj);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, serde_json::json!({"key": "value"}));
    }

    // ========================================
    // Roundtrip conversion tests
    // ========================================

    #[test]
    fn test_roundtrip_string() {
        let original = serde_json::json!("test");
        let obj = value_to_object(original.clone()).unwrap();
        let back = object_to_value(obj).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_roundtrip_number() {
        let original = serde_json::json!(42);
        let obj = value_to_object(original.clone()).unwrap();
        let back = object_to_value(obj).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_roundtrip_complex() {
        let original = serde_json::json!({
            "string": "value",
            "number": 42,
            "boolean": true,
            "array": [1, 2, 3],
            "nested": {"key": "value"}
        });
        let obj = value_to_object(original.clone()).unwrap();
        let back = object_to_value(obj).unwrap();
        assert_eq!(original, back);
    }
}
