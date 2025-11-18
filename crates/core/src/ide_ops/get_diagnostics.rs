//! LSP diagnostics operation

use serde::Deserialize;
use serde_json::{json, Value};

use crate::errors::{AmpError, Result};

/// Parameters for getDiagnostics
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GetDiagnosticsParams {
    path: Option<String>,
}

/// Diagnostic entry from Neovim's vim.diagnostic.get()
///
/// Fields are 0-based as returned by Neovim
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct NvimDiagnostic {
    pub(crate) lnum:     u32,
    pub(crate) col:      u32,
    pub(crate) end_lnum: Option<u32>,
    pub(crate) end_col:  Option<u32>,
    pub(crate) severity: Option<u8>,
    pub(crate) message:  String,
}

/// Handle getDiagnostics request
///
/// Returns diagnostics for a file or directory path.
/// Integrates with Neovim's diagnostic system.
///
/// Request:
/// ```json
/// { "path": "/path/to/file.txt" }  // Optional - file or directory prefix
/// ```
///
/// Response:
/// ```json
/// {
///   "entries": [
///     {
///       "uri": "file:///path/to/file.rs",
///       "diagnostics": [...]
///     }
///   ]
/// }
/// ```
///
/// # Errors
/// - InvalidArgs: Invalid parameters
pub fn get_diagnostics(params: Value) -> Result<Value> {
    let _params: GetDiagnosticsParams =
        serde_json::from_value(params).map_err(|e| AmpError::InvalidArgs {
            command: "getDiagnostics".to_string(),
            reason:  e.to_string(),
        })?;

    // Only get diagnostics if Neovim is initialized
    if super::nvim_available() {
        return get_diagnostics_impl(_params.path.as_deref());
    }

    // Fallback: return empty diagnostics
    Ok(json!({
        "entries": []
    }))
}

/// Implementation of getDiagnostics
fn get_diagnostics_impl(path_filter: Option<&str>) -> Result<Value> {
    use std::collections::HashMap;

    use nvim_oxi::{api, conversion::FromObject};

    // Collect diagnostics grouped by file URI
    let mut entries_map: HashMap<String, Vec<Value>> = HashMap::new();

    // Normalize the filter path (handles /tmp -> /private/tmp on macOS)
    let filter_raw = path_filter.map(|s| s.to_string());
    let filter_canon = filter_raw
        .as_deref()
        .and_then(|p| std::fs::canonicalize(p).ok());

    // Iterate through all buffers
    for buf in api::list_bufs() {
        // Skip unloaded buffers
        if !buf.is_loaded() {
            continue;
        }

        // Get buffer path
        let Ok(buf_path) = buf.get_name() else {
            continue;
        };

        // Only consider absolute paths
        if !buf_path.is_absolute() {
            continue;
        }

        let path_str = buf_path.to_string_lossy().to_string();

        // Apply path filter (prefix matching for directories)
        // Check both raw and canonical paths to handle symlinks (e.g., /tmp ->
        // /private/tmp)
        if filter_raw.is_some() || filter_canon.is_some() {
            let mut matches = false;

            // Try raw path match first (fastest)
            if let Some(ref filter) = filter_raw {
                if path_str.starts_with(filter) {
                    matches = true;
                }
            }

            // Try canonical path match if raw didn't match
            if !matches {
                if let Some(ref filter_canon_path) = filter_canon {
                    // Canonicalize buffer path for comparison
                    if let Ok(canon_buf) = std::fs::canonicalize(&buf_path) {
                        let canon_buf_str = canon_buf.to_string_lossy();
                        let canon_filter_str = filter_canon_path.to_string_lossy();
                        if canon_buf_str.starts_with(canon_filter_str.as_ref()) {
                            matches = true;
                        }
                    }
                }
            }

            if !matches {
                continue;
            }
        }

        // Get diagnostics as Object (no JSON round-trip)
        // Use buffer handle directly instead of path lookup
        let bufnr = buf.handle();
        let lua_expr = "vim.diagnostic.get(_A)";
        let result: std::result::Result<nvim_oxi::Object, _> =
            api::call_function("luaeval", (lua_expr, bufnr));

        let Ok(diag_obj) = result else {
            continue;
        };

        // Deserialize via serde (NvimDiagnostic doesn't implement FromObject)
        use nvim_oxi::serde::Deserializer;
        use serde::Deserialize;
        let diags: Vec<NvimDiagnostic> = match Vec::<NvimDiagnostic>::deserialize(Deserializer::new(diag_obj)) {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Skip if no diagnostics for this buffer
        if diags.is_empty() {
            continue;
        }

        // Get LSP-compliant URI with percent-encoding
        let uri_obj =
            match api::call_function("luaeval", ("vim.uri_from_fname(_A)", path_str.as_str())) {
                Ok(obj) => obj,
                Err(_) => {
                    // Fallback to simple format if vim.uri_from_fname fails
                    nvim_oxi::Object::from(nvim_oxi::string!("file://{}", path_str))
                },
            };

        let uri: String = match <String as FromObject>::from_object(uri_obj) {
            Ok(u) => u,
            Err(_) => nvim_oxi::string!("file://{}", path_str).to_string(), // Fallback
        };

        let diagnostics: Vec<Value> = diags
            .into_iter()
            .map(|diag| {
                let line_content = super::get_line_content(&buf_path, diag.lnum);
                let start_line = diag.lnum;
                let start_char = diag.col;
                let end_line = diag.end_lnum.unwrap_or(diag.lnum);
                let end_char = diag.end_col.unwrap_or(diag.col);

                // Calculate character offsets (simple approach)
                let start_offset = start_char;
                let end_offset = end_char;

                json!({
                    "range": {
                        "startLine": start_line,
                        "startCharacter": start_char,
                        "endLine": end_line,
                        "endCharacter": end_char
                    },
                    "severity": super::map_severity(diag.severity),
                    "description": diag.message,
                    "lineContent": line_content,
                    "startOffset": start_offset,
                    "endOffset": end_offset
                })
            })
            .collect();

        entries_map.insert(uri, diagnostics);
    }

    // Convert to entries array
    let entries: Vec<Value> = entries_map
        .into_iter()
        .map(|(uri, diagnostics)| {
            json!({
                "uri": uri,
                "diagnostics": diagnostics
            })
        })
        .collect();

    Ok(json!({
        "entries": entries
    }))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_get_diagnostics_empty() {
        let result = get_diagnostics(json!({"path": "/tmp/test.txt"})).unwrap();

        assert!(result["entries"].is_array());
        assert_eq!(result["entries"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_get_diagnostics_no_params() {
        // Should still return empty for now
        let result = get_diagnostics(json!({})).unwrap();

        assert!(result["entries"].is_array());
    }
}
