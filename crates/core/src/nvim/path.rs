//! Path and URI conversion utilities
//!
//! Provides functions for converting between file paths and URIs,
//! and for getting workspace-relative paths.

use std::path::Path;

use nvim_oxi::api;

use crate::errors::{AmpError, Result};

/// Convert a file path to a file:// URI
///
/// Uses Neovim's `vim.uri_from_fname()` for LSP-compliant URIs with proper
/// percent-encoding of special characters (spaces, Unicode, etc.).
///
/// # Arguments
/// * `path` - File path to convert
///
/// # Returns
/// A file:// URI string with percent-encoded characters
///
/// # Example
/// ```rust,ignore
/// let uri = path::to_uri(Path::new("/tmp/test file.txt"))?;
/// // Returns: "file:///tmp/test%20file.txt"
/// ```
pub fn to_uri(path: &Path) -> Result<String> {
    #[cfg(not(test))]
    {
        use nvim_oxi::mlua::prelude::*;

        let lua = nvim_oxi::mlua::lua();
        let vim = lua
            .globals()
            .get::<LuaTable>("vim")
            .map_err(|e| AmpError::Other(format!("Failed to get vim global: {}", e)))?;

        let uri_from_fname = vim
            .get::<LuaFunction>("uri_from_fname")
            .map_err(|e| AmpError::Other(format!("Failed to get uri_from_fname: {}", e)))?;

        let path_str = path.to_string_lossy();
        let uri: String = uri_from_fname
            .call(path_str.as_ref())
            .map_err(|e| AmpError::Other(format!("Failed to call uri_from_fname: {}", e)))?;

        Ok(uri)
    }

    #[cfg(test)]
    Ok(format!("file://{}", path.display()))
}

/// Convert a file:// URI to a file path
///
/// Uses Neovim's `vim.uri_to_fname()` for proper URI decoding, handling
/// percent-encoded characters and platform-specific paths.
///
/// # Arguments
/// * `uri` - file:// URI string
///
/// # Returns
/// - `Ok(path)` - Successfully parsed path
/// - `Err(_)` - Invalid URI format
///
/// # Example
/// ```rust,ignore
/// let path = path::from_uri("file:///tmp/test%20file.txt")?;
/// assert_eq!(path, PathBuf::from("/tmp/test file.txt"));
/// ```
pub fn from_uri(uri: &str) -> Result<std::path::PathBuf> {
    #[cfg(not(test))]
    {
        use nvim_oxi::mlua::prelude::*;

        let lua = nvim_oxi::mlua::lua();
        let vim = lua
            .globals()
            .get::<LuaTable>("vim")
            .map_err(|e| AmpError::Other(format!("Failed to get vim global: {}", e)))?;

        let uri_to_fname = vim
            .get::<LuaFunction>("uri_to_fname")
            .map_err(|e| AmpError::Other(format!("Failed to get uri_to_fname: {}", e)))?;

        let path: String = uri_to_fname
            .call(uri)
            .map_err(|e| AmpError::Other(format!("Failed to call uri_to_fname: {}", e)))?;

        Ok(std::path::PathBuf::from(path))
    }

    #[cfg(test)]
    uri.strip_prefix("file://")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| AmpError::Other("Invalid URI format".into()))
}

/// Convert an absolute path to a workspace-relative path
///
/// Uses Neovim's `fnamemodify(path, ':.')` to get the path relative to the
/// current working directory. This is used for creating file references like
/// `@src/main.rs` instead of `@/home/user/project/src/main.rs`.
///
/// # Arguments
/// * `path` - Absolute file path to convert
///
/// # Returns
/// Workspace-relative path as a String
///
/// # Errors
/// Returns error if path is invalid or Neovim API call fails
///
/// # Example
/// ```rust,ignore
/// let relative = path::to_relative(Path::new("/home/user/project/src/main.rs"))?;
/// // Returns: "src/main.rs" (if cwd is /home/user/project)
/// ```
pub fn to_relative(path: &Path) -> Result<String> {
    use nvim_oxi::conversion::FromObject;

    let path_str = path
        .to_str()
        .ok_or_else(|| AmpError::Other("Invalid path encoding".into()))?;

    if path_str.is_empty() {
        return Err(AmpError::Other("Empty path provided".into()));
    }

    // Use fnamemodify(path, ':.') to get relative path
    let obj = api::call_function("fnamemodify", (path_str, ":."))
        .map_err(|e| AmpError::Other(format!("Failed to call fnamemodify: {}", e)))?;

    let relative: String = <String as FromObject>::from_object(obj).map_err(|e| {
        AmpError::ConversionError(format!("Failed to convert relative path: {}", e))
    })?;

    // Filter out "v:null" or other invalid values
    if !relative.is_empty() && !relative.starts_with("v:") {
        return Ok(relative);
    }

    // Fallback: return path as-is if it's absolute
    if path.is_absolute() {
        Ok(path_str.to_string())
    } else {
        Err(AmpError::Other(format!(
            "Failed to get relative path for: {}",
            path_str
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_to_uri() {
        let path = Path::new("/tmp/test.txt");
        let uri = to_uri(path);
        assert!(uri.is_ok());
        assert_eq!(uri.unwrap(), "file:///tmp/test.txt");
    }

    #[test]
    fn test_to_uri_with_spaces() {
        let path = Path::new("/tmp/my file.txt");
        let uri = to_uri(path);
        assert!(uri.is_ok());
        // In test mode, simple format (no percent-encoding)
        // In production, vim.uri_from_fname would encode as %20
        assert_eq!(uri.unwrap(), "file:///tmp/my file.txt");
    }

    #[test]
    fn test_from_uri() {
        let uri = "file:///tmp/test.txt";
        let path = from_uri(uri);
        assert!(path.is_ok());
        assert_eq!(path.unwrap(), PathBuf::from("/tmp/test.txt"));
    }

    #[test]
    fn test_from_uri_invalid() {
        let uri = "http://example.com/file.txt";
        let result = from_uri(uri);
        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip() {
        let original = Path::new("/home/user/project/src/main.rs");
        let uri = to_uri(original);
        assert!(uri.is_ok());
        let back = from_uri(&uri.unwrap());
        assert!(back.is_ok());
        assert_eq!(back.unwrap(), original.to_path_buf());
    }

    #[test]
    #[ignore = "Requires Neovim context"]
    fn test_to_relative() {
        // This test requires actual Neovim context
        // Run with: just test-integration
        let path = Path::new("/home/user/project/src/main.rs");
        let result = to_relative(path);
        // Would return "src/main.rs" if cwd is /home/user/project
        assert!(result.is_ok());
    }
}
