//! Conversion utilities for nvim_oxi::Object ↔ Rust types
//!
//! Provides centralized, type-safe conversion logic using nvim-oxi's serde
//! integration. Used throughout the codebase for:
//! - FFI boundary conversions (Lua ↔ Rust)
//! - Neovim API responses (luaeval, vim.fn.*, etc.)
//! - Command dispatch and result handling

use nvim_oxi::{
    serde::{Deserializer, Serializer},
    Object,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// Generic Typed Conversions
// ============================================================================

/// Convert nvim-oxi Object to any deserializable Rust type
///
/// Uses nvim-oxi's serde integration to deserialize directly from Object.
/// This is the preferred way to convert Neovim API results into typed Rust
/// data.
///
/// # Examples
/// ```rust,ignore
/// let s: String = from_object(obj)?;
/// let n: i64 = from_object(obj)?;
/// let diagnostics: Vec<Diagnostic> = from_object(obj)?;
/// ```
///
/// # Errors
/// Returns `nvim_oxi::Error::Deserialize` if the Object structure doesn't match
/// type T
pub fn from_object<T: DeserializeOwned>(obj: Object) -> nvim_oxi::Result<T> {
    T::deserialize(Deserializer::new(obj)).map_err(nvim_oxi::Error::Deserialize)
}

/// Convert any serializable Rust type to nvim-oxi Object
///
/// Uses nvim-oxi's serde integration to serialize to Object.
/// Useful for preparing values to pass to Neovim API functions.
///
/// # Examples
/// ```rust,ignore
/// let obj = to_object(&"hello")?;
/// let obj = to_object(&42)?;
/// let obj = to_object(&vec![1, 2, 3])?;
/// ```
///
/// # Errors
/// Returns `nvim_oxi::Error::Serialize` if the value cannot be serialized
pub fn to_object<T: Serialize>(value: &T) -> nvim_oxi::Result<Object> {
    value
        .serialize(Serializer::new())
        .map_err(nvim_oxi::Error::Serialize)
}

// ============================================================================
// JSON-Specific Conversions
// ============================================================================

/// Convert nvim-oxi Object to serde_json::Value
///
/// This is the bridge between Neovim's Object representation and JSON.
/// Used primarily at FFI boundaries where commands expect JSON.
///
/// # Examples
/// ```rust,ignore
/// let json_value = object_to_json(lua_args)?;
/// // json_value can now be passed to command dispatch
/// ```
///
/// # Errors
/// Returns `nvim_oxi::Error::Deserialize` if Object cannot be represented as
/// JSON
pub fn object_to_json(obj: Object) -> nvim_oxi::Result<Value> {
    Value::deserialize(Deserializer::new(obj)).map_err(nvim_oxi::Error::Deserialize)
}

/// Convert serde_json::Value to nvim-oxi Object
///
/// This is the bridge from JSON back to Neovim's Object representation.
/// Used primarily at FFI boundaries to return command results to Lua.
///
/// # Examples
/// ```rust,ignore
/// let result = json!({ "success": true });
/// let obj = json_to_object(result)?;
/// // obj can now be returned to Lua
/// ```
///
/// # Errors
/// Returns `nvim_oxi::Error::Serialize` if Value cannot be converted to Object
pub fn json_to_object(value: Value) -> nvim_oxi::Result<Object> {
    value
        .serialize(Serializer::new())
        .map_err(nvim_oxi::Error::Serialize)
}

#[cfg(test)]
mod tests {
    use nvim_oxi::{conversion::FromObject, Dictionary};
    use serde_json::json;

    use super::*;

    // ========================================
    // Generic typed conversion tests
    // ========================================

    #[test]
    fn test_from_object_string() {
        let obj = Object::from("test string");
        let result: Result<String, _> = from_object(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test string");
    }

    #[test]
    fn test_from_object_number() {
        let obj = Object::from(42);
        let result: Result<i64, _> = from_object(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_from_object_bool() {
        let obj = Object::from(true);
        let result: Result<bool, _> = from_object(obj);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_from_object_vec() {
        // Create Object from Vec using to_object
        let obj = to_object(&vec![1i64, 2, 3]).unwrap();
        let result: Result<Vec<i64>, _> = from_object(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn test_to_object_string() {
        let result = to_object(&"hello");
        assert!(result.is_ok());
        let obj = result.unwrap();
        let s = <String as FromObject>::from_object(obj).unwrap();
        assert_eq!(s, "hello");
    }

    #[test]
    fn test_to_object_number() {
        let result = to_object(&123i64);
        assert!(result.is_ok());
        let obj = result.unwrap();
        let n = <i64 as FromObject>::from_object(obj).unwrap();
        assert_eq!(n, 123);
    }

    #[test]
    fn test_to_object_vec() {
        let result = to_object(&vec![1i64, 2, 3]);
        assert!(result.is_ok());
        let obj = result.unwrap();
        let v = <Vec<i64> as FromObject>::from_object(obj).unwrap();
        assert_eq!(v, vec![1, 2, 3]);
    }

    // ========================================
    // JSON conversion tests
    // ========================================

    #[test]
    fn test_json_to_object_string() {
        let value = json!("test string");
        let result = json_to_object(value);
        assert!(result.is_ok());
        let obj = result.unwrap();
        let s = <String as FromObject>::from_object(obj).unwrap();
        assert_eq!(s, "test string");
    }

    #[test]
    fn test_json_to_object_number() {
        let value = json!(42);
        let result = json_to_object(value);
        assert!(result.is_ok());
        let obj = result.unwrap();
        let n = <i64 as FromObject>::from_object(obj).unwrap();
        assert_eq!(n, 42);
    }

    #[test]
    fn test_json_to_object_boolean() {
        let value = json!(true);
        let result = json_to_object(value);
        assert!(result.is_ok());
        let obj = result.unwrap();
        let b = <bool as FromObject>::from_object(obj).unwrap();
        assert!(b);
    }

    #[test]
    fn test_json_to_object_null() {
        let value = json!(null);
        let result = json_to_object(value);
        assert!(result.is_ok());
        assert!(result.unwrap().is_nil());
    }

    #[test]
    fn test_json_to_object_array() {
        let value = json!([1, 2, 3]);
        let result = json_to_object(value);
        assert!(result.is_ok());
        let obj = result.unwrap();
        let array = <Vec<i64> as FromObject>::from_object(obj).unwrap();
        assert_eq!(array, vec![1, 2, 3]);
    }

    #[test]
    fn test_json_to_object_dict() {
        let value = json!({"key": "value"});
        let result = json_to_object(value);
        assert!(result.is_ok());
        let obj = result.unwrap();
        let dict = Dictionary::from_object(obj).unwrap();
        let v = <String as FromObject>::from_object(dict.get("key").unwrap().clone()).unwrap();
        assert_eq!(v, "value");
    }

    #[test]
    fn test_json_to_object_nested() {
        let value = json!({
            "nested": {
                "array": [1, 2, 3],
                "string": "test"
            }
        });
        let result = json_to_object(value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_object_to_json_string() {
        let obj = Object::from("test string");
        let result = object_to_json(obj);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, json!("test string"));
    }

    #[test]
    fn test_object_to_json_number() {
        let obj = Object::from(42);
        let result = object_to_json(obj);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, json!(42));
    }

    #[test]
    fn test_object_to_json_boolean() {
        let obj = Object::from(true);
        let result = object_to_json(obj);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, json!(true));
    }

    #[test]
    fn test_object_to_json_nil() {
        let obj = Object::nil();
        let result = object_to_json(obj);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, json!(null));
    }

    #[test]
    fn test_object_to_json_dict() {
        let mut dict = Dictionary::new();
        dict.insert("key", "value");
        let obj = Object::from(dict);
        let result = object_to_json(obj);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, json!({"key": "value"}));
    }

    // ========================================
    // Roundtrip conversion tests
    // ========================================

    #[test]
    fn test_roundtrip_json_string() {
        let original = json!("test");
        let obj = json_to_object(original.clone()).unwrap();
        let back = object_to_json(obj).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_roundtrip_json_number() {
        let original = json!(42);
        let obj = json_to_object(original.clone()).unwrap();
        let back = object_to_json(obj).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_roundtrip_json_complex() {
        let original = json!({
            "string": "value",
            "number": 42,
            "boolean": true,
            "array": [1, 2, 3],
            "nested": {"key": "value"}
        });
        let obj = json_to_object(original.clone()).unwrap();
        let back = object_to_json(obj).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_roundtrip_typed_string() {
        let original = "hello world";
        let obj = to_object(&original).unwrap();
        let back: String = from_object(obj).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_roundtrip_typed_vec() {
        let original = vec![1i64, 2, 3, 4, 5];
        let obj = to_object(&original).unwrap();
        let back: Vec<i64> = from_object(obj).unwrap();
        assert_eq!(original, back);
    }
}
