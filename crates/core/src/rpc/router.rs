//! JSON-RPC message routing
//!
//! Routes incoming JSON-RPC messages to appropriate handlers:
//! - ide/* methods → ide_ops module
//! - Other methods → commands::dispatch
//!
//! Supports both standard JSON-RPC 2.0 and amp.nvim custom wrapper protocol.

use serde_json::Value;

use super::{ClientRequest, ErrorObject, Notification, Request, Response, ServerResponse};
use crate::{
    commands,
    errors::{AmpError, Result},
    ide_ops,
};

/// Parse and route incoming text message
///
/// Auto-detects protocol format (JSON-RPC 2.0 or amp.nvim) and routes to
/// appropriate handler. Returns:
/// - Some(json_response) for requests (must send response)
/// - None for notifications (no response needed)
pub fn handle_text(text: &str) -> Result<Option<String>> {
    // Try to parse as generic JSON first
    let value: Value = serde_json::from_str(text).map_err(AmpError::SerdeError)?;

    // Check if this is amp.nvim wrapped format (has "clientRequest" field)
    if value.get("clientRequest").is_some() {
        // This is amp.nvim protocol
        let client_req: ClientRequest = serde_json::from_value(value)?;
        let response = handle_amp_request(client_req)?;
        let json = serde_json::to_string(&response)?;
        Ok(Some(json))
    }
    // Check if this is standard JSON-RPC (has "jsonrpc" field)
    else if value.get("jsonrpc").is_some() {
        // Standard JSON-RPC 2.0
        if value.get("id").is_some() {
            // This is a request
            let request: Request = serde_json::from_value(value)?;
            let response = handle_request(request)?;
            let json = serde_json::to_string(&response)?;
            Ok(Some(json))
        } else {
            // This is a notification
            let notification: Notification = serde_json::from_value(value)?;
            handle_notification(notification)?;
            Ok(None)
        }
    } else {
        Err(AmpError::Other(
            "Unknown message format - missing clientRequest or jsonrpc field".into(),
        ))
    }
}

/// Handle amp.nvim wrapped request
fn handle_amp_request(client_req: ClientRequest) -> Result<ServerResponse> {
    let parsed = client_req.parse().map_err(|e| AmpError::Other(e.into()))?;

    let result = route_method(&parsed.method, parsed.params);

    match result {
        Ok(value) => Ok(ServerResponse::success(parsed.id, parsed.method, value)),
        Err(err) => {
            let error = ErrorObject {
                code:    err.to_jsonrpc_code(),
                message: err.to_string(),
                data:    None,
            };
            Ok(ServerResponse::error(parsed.id, error))
        },
    }
}

/// Route a JSON-RPC request to the appropriate handler
///
/// Routes based on method prefix:
/// - ide/* → ide_ops handlers
/// - Other → commands::dispatch
pub fn handle_request(req: Request) -> Result<Response> {
    let result = route_method(&req.method, req.params);

    match result {
        Ok(value) => Ok(Response {
            jsonrpc: "2.0".to_string(),
            id:      req.id,
            result:  Some(value),
            error:   None,
        }),
        Err(err) => Ok(Response {
            jsonrpc: "2.0".to_string(),
            id:      req.id,
            result:  None,
            error:   Some(ErrorObject {
                code:    err.to_jsonrpc_code(),
                message: err.to_string(),
                data:    None,
            }),
        }),
    }
}

/// Handle a JSON-RPC notification
///
/// Routes to handlers but does not return a response
pub fn handle_notification(notif: Notification) -> Result<()> {
    route_method(&notif.method, notif.params)?;
    Ok(())
}

/// Route method to appropriate handler based on method name
///
/// Supports both JSON-RPC format (ide/ping) and amp.nvim format (ping).
///
/// - ping (or ide/ping) → ide_ops::ping
/// - authenticate → ide_ops::authenticate
/// - readFile (or ide/readFile) → ide_ops::read_file
/// - editFile (or ide/editFile) → ide_ops::edit_file
/// - getDiagnostics → ide_ops::get_diagnostics
/// - nvim/notify → ide_ops::nvim_notify
/// - Other → commands::dispatch
fn route_method(method: &str, params: Value) -> Result<Value> {
    match method {
        // IDE protocol operations (amp.nvim format - no prefix)
        "ping" => ide_ops::ping(params),
        "authenticate" => ide_ops::authenticate(params),
        "readFile" => ide_ops::read_file(params),
        "editFile" => ide_ops::edit_file(params),
        "getDiagnostics" => ide_ops::get_diagnostics(params),

        // IDE protocol operations (JSON-RPC format - with ide/ prefix)
        "ide/ping" => ide_ops::ping(params),
        "ide/readFile" => ide_ops::read_file(params),
        "ide/editFile" => ide_ops::edit_file(params),

        // Neovim notifications
        "nvim/notify" => {
            ide_ops::nvim_notify(params)?;
            Ok(Value::Null)
        },

        // Delegate to command registry
        _ => commands::dispatch(method, params),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::rpc::Id;

    // ========================================
    // handle_text() tests
    // ========================================

    #[test]
    fn test_parse_valid_request() {
        let text = r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":{}}"#;
        let result = handle_text(text);

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_some());

        // Parse response to verify it's valid JSON-RPC
        let resp_value: Value = serde_json::from_str(&response.unwrap()).unwrap();
        assert_eq!(resp_value["jsonrpc"], "2.0");
        assert_eq!(resp_value["id"], 1);
        assert!(resp_value.get("result").is_some() || resp_value.get("error").is_some());
    }

    #[test]
    fn test_parse_notification() {
        // Use a method that doesn't panic (ping exists in commands)
        let text = r#"{"jsonrpc":"2.0","method":"ping","params":{}}"#;
        let result = handle_text(text);

        // Should succeed and return None (no response for notifications)
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_parse_invalid_json() {
        let text = "not valid json";
        let result = handle_text(text);

        assert!(result.is_err());
        match result {
            Err(AmpError::SerdeError(_)) => {
                // Expected
            },
            _ => panic!("Expected SerdeError"),
        }
    }

    #[test]
    fn test_parse_request_with_string_id() {
        let text = r#"{"jsonrpc":"2.0","id":"req-123","method":"ping","params":{}}"#;
        let result = handle_text(text);

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_some());

        let resp_value: Value = serde_json::from_str(&response.unwrap()).unwrap();
        assert_eq!(resp_value["id"], "req-123");
    }

    // ========================================
    // handle_request() tests
    // ========================================

    #[test]
    fn test_handle_request_success() {
        let request = Request {
            jsonrpc: "2.0".to_string(),
            id:      Id::Number(1),
            method:  "ping".to_string(),
            params:  json!({}),
        };

        let response = handle_request(request).unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Id::Number(1));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        // Verify ping response contains "pong"
        let result = response.result.unwrap();
        assert_eq!(result["pong"], json!(true));
    }

    #[test]
    fn test_handle_request_error() {
        let request = Request {
            jsonrpc: "2.0".to_string(),
            id:      Id::Number(2),
            method:  "nonexistent.command".to_string(),
            params:  json!({}),
        };

        let response = handle_request(request).unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Id::Number(2));
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, -32601); // Method not found
        assert!(error.message.contains("nonexistent.command"));
    }

    // ========================================
    // handle_notification() tests
    // ========================================

    #[test]
    fn test_handle_notification_success() {
        // Use a method that exists (ping)
        let notification = Notification {
            jsonrpc: "2.0".to_string(),
            method:  "ping".to_string(),
            params:  json!({}),
        };

        // Should succeed without error
        let result = handle_notification(notification);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_notification_error() {
        let notification = Notification {
            jsonrpc: "2.0".to_string(),
            method:  "nonexistent.command".to_string(),
            params:  json!({}),
        };

        // Should return error
        let result = handle_notification(notification);
        assert!(result.is_err());
    }

    // ========================================
    // route_method() tests
    // ========================================

    #[test]
    fn test_route_to_ping() {
        let result = route_method("ping", json!({}));

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["pong"], json!(true));
    }

    #[test]
    fn test_route_ide_ping() {
        let result = route_method("ide/ping", json!({}));

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["pong"], json!(true));
        assert!(value.get("ts").is_some());
    }

    #[test]
    fn test_route_ide_read_file() {
        use std::fs;

        use tempfile::NamedTempFile;

        // Create temp file
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();
        fs::write(path, "test content").unwrap();

        let result = route_method("ide/readFile", json!({"path": path}));

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["content"], json!("test content"));
    }

    #[test]
    fn test_route_ide_edit_file() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let path_str = file_path.to_str().unwrap();

        let result = route_method(
            "ide/editFile",
            json!({
                "path": path_str,
                "content": "hello"
            }),
        );

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["success"], json!(true));
    }

    #[test]
    fn test_route_nvim_notify() {
        let result = route_method("nvim/notify", json!({"message": "test"}));

        // In tests, schedule() is skipped with #[cfg(not(test))], so this succeeds
        assert!(result.is_ok());
    }

    #[test]
    fn test_route_unknown_method() {
        let result = route_method("unknown.method", json!({}));

        assert!(result.is_err());
        match result {
            Err(AmpError::CommandNotFound(cmd)) => {
                assert_eq!(cmd, "unknown.method");
            },
            _ => panic!("Expected CommandNotFound error"),
        }
    }

    // ========================================
    // Error code mapping tests
    // ========================================

    #[test]
    fn test_error_codes_in_response() {
        // Method not found
        let req1 = Request {
            jsonrpc: "2.0".to_string(),
            id:      Id::Number(1),
            method:  "nonexistent".to_string(),
            params:  json!({}),
        };
        let resp1 = handle_request(req1).unwrap();
        assert_eq!(resp1.error.unwrap().code, -32601);

        // Invalid params (if we had validation)
        // This is tested indirectly through command dispatch
    }

    #[test]
    fn test_response_structure() {
        let request = Request {
            jsonrpc: "2.0".to_string(),
            id:      Id::String("test-id".to_string()),
            method:  "ping".to_string(),
            params:  json!({"custom": "data"}),
        };

        let response = handle_request(request).unwrap();

        // Verify JSON-RPC 2.0 compliance
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Id::String("test-id".to_string()));

        // Success response should have result, not error
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        // Error response would have error, not result (tested above)
    }

    // ========================================
    // amp.nvim protocol tests
    // ========================================

    #[test]
    fn test_handle_amp_ping_request() {
        let text = r#"{"clientRequest":{"id":"req-123","ping":{}}}"#;
        let result = handle_text(text).unwrap();

        assert!(result.is_some());
        let response = result.unwrap();

        // Parse response
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert!(value.get("serverResponse").is_some());
        assert_eq!(value["serverResponse"]["id"], "req-123");
        assert!(value["serverResponse"]["ping"]["pong"].as_bool().unwrap());
    }

    #[test]
    fn test_handle_amp_read_file_request() {
        use std::fs;

        use tempfile::NamedTempFile;

        // Create temp file
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();
        fs::write(path, "test content").unwrap();

        let text = format!(
            r#"{{"clientRequest":{{"id":"req-456","readFile":{{"path":"{}"}}}}}}"#,
            path
        );
        let result = handle_text(&text).unwrap();

        assert!(result.is_some());
        let response = result.unwrap();

        // Parse response
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["serverResponse"]["id"], "req-456");
        assert_eq!(
            value["serverResponse"]["readFile"]["content"],
            "test content"
        );
    }

    #[test]
    fn test_handle_amp_error_response() {
        let text = r#"{"clientRequest":{"id":"req-789","readFile":{}}}"#;
        let result = handle_text(text).unwrap();

        assert!(result.is_some());
        let response = result.unwrap();

        // Parse response
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert!(value["serverResponse"]["error"].is_object());
        assert!(value["serverResponse"]["error"]["code"].is_number());
    }

    #[test]
    fn test_detect_json_rpc_vs_amp_protocol() {
        // JSON-RPC request
        let jsonrpc_text = r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":{}}"#;
        let result = handle_text(jsonrpc_text).unwrap();
        assert!(result.is_some());
        let response = result.unwrap();
        assert!(response.contains("jsonrpc"));
        assert!(response.contains("\"2.0\""));

        // amp.nvim request
        let amp_text = r#"{"clientRequest":{"id":"req-1","ping":{}}}"#;
        let result = handle_text(amp_text).unwrap();
        assert!(result.is_some());
        let response = result.unwrap();
        assert!(response.contains("serverResponse"));
        assert!(!response.contains("jsonrpc"));
    }

    #[test]
    fn test_unknown_protocol_format() {
        let text = r#"{"unknownField":"value"}"#;
        let result = handle_text(text);

        assert!(result.is_err());
        match result {
            Err(AmpError::Other(msg)) => {
                assert!(msg.contains("Unknown message format"));
            },
            _ => panic!("Expected Other error"),
        }
    }

    #[test]
    fn test_handle_amp_authenticate_request() {
        let text = r#"{"clientRequest":{"id":"req-auth","authenticate":{}}}"#;
        let result = handle_text(text).unwrap();

        assert!(result.is_some());
        let response = result.unwrap();

        // Parse response
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["serverResponse"]["id"], "req-auth");
        assert_eq!(
            value["serverResponse"]["authenticate"]["authenticated"],
            true
        );
    }

    #[test]
    fn test_handle_amp_get_diagnostics_request() {
        let text =
            r#"{"clientRequest":{"id":"req-diag","getDiagnostics":{"path":"/tmp/test.txt"}}}"#;
        let result = handle_text(text).unwrap();

        assert!(result.is_some());
        let response = result.unwrap();

        // Parse response
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["serverResponse"]["id"], "req-diag");
        assert!(value["serverResponse"]["getDiagnostics"]["entries"].is_array());
    }

    #[test]
    fn test_handle_amp_ping_with_message() {
        let text = r#"{"clientRequest":{"id":"req-msg","ping":{"message":"hello"}}}"#;
        let result = handle_text(text).unwrap();

        assert!(result.is_some());
        let response = result.unwrap();

        // Parse response
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["serverResponse"]["ping"]["message"], "hello");
    }
}
