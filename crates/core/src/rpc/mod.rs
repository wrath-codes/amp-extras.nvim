//! JSON-RPC 2.0 types and message handling
//!
//! This module supports both standard JSON-RPC 2.0 and the amp.nvim
//! custom protocol format which wraps messages in clientRequest/serverResponse.

pub mod router;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub id: Id,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id: Id,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorObject>,
}

/// JSON-RPC 2.0 Notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC 2.0 Error Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorObject {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// JSON-RPC 2.0 ID (String or Number)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum Id {
    String(String),
    Number(i64),
}

// ============================================================================
// Amp.nvim Protocol Types (Custom Wrapper Format)
// ============================================================================

/// Client request wrapper used by amp.nvim protocol
///
/// Format: { "clientRequest": { "id": "...", "methodName": { params } } }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientRequest {
    pub client_request: ClientRequestInner,
}

/// Inner structure of client request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRequestInner {
    pub id: String,
    #[serde(flatten)]
    pub method_data: HashMap<String, Value>,
}

/// Server response wrapper used by amp.nvim protocol
///
/// Format: { "serverResponse": { "id": "...", "methodName": { result } } }
/// or     { "serverResponse": { "id": "...", "error": { code, message } } }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerResponse {
    pub server_response: ServerResponseInner,
}

/// Inner structure of server response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerResponseInner {
    pub id: String,
    #[serde(flatten)]
    pub response_data: HashMap<String, Value>,
}

/// Server notification wrapper used by amp.nvim protocol
///
/// Format: { "serverNotification": { "notificationName": { data } } }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerNotification {
    pub server_notification: HashMap<String, Value>,
}

/// Parsed request with method name and parameters
#[derive(Debug, Clone)]
pub struct ParsedRequest {
    pub id: String,
    pub method: String,
    pub params: Value,
}

impl ClientRequest {
    /// Parse the client request into method name and parameters
    pub fn parse(&self) -> Result<ParsedRequest, &'static str> {
        if self.client_request.method_data.len() != 1 {
            return Err("ClientRequest must have exactly one method");
        }
        
        let (method, params) = self.client_request.method_data.iter().next()
            .ok_or("No method in ClientRequest")?;
        
        Ok(ParsedRequest {
            id: self.client_request.id.clone(),
            method: method.clone(),
            params: params.clone(),
        })
    }
}

impl ServerResponse {
    /// Create a success response for amp.nvim protocol
    pub fn success(id: String, method: String, result: Value) -> Self {
        let mut response_data = HashMap::new();
        response_data.insert(method, result);
        
        ServerResponse {
            server_response: ServerResponseInner {
                id,
                response_data,
            },
        }
    }
    
    /// Create an error response for amp.nvim protocol
    pub fn error(id: String, error: ErrorObject) -> Self {
        let mut response_data = HashMap::new();
        response_data.insert("error".to_string(), serde_json::to_value(error).unwrap());
        
        ServerResponse {
            server_response: ServerResponseInner {
                id,
                response_data,
            },
        }
    }
}

impl ServerNotification {
    /// Create a server notification for amp.nvim protocol
    pub fn new(notification_name: String, data: Value) -> Self {
        let mut notification_data = HashMap::new();
        notification_data.insert(notification_name, data);
        
        ServerNotification {
            server_notification: notification_data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_client_request_parse() {
        let json_str = r#"{"clientRequest":{"id":"req-123","ping":{"message":"hello"}}}"#;
        let req: ClientRequest = serde_json::from_str(json_str).unwrap();
        
        let parsed = req.parse().unwrap();
        assert_eq!(parsed.id, "req-123");
        assert_eq!(parsed.method, "ping");
        assert_eq!(parsed.params["message"], "hello");
    }

    #[test]
    fn test_client_request_read_file() {
        let json_str = r#"{"clientRequest":{"id":"req-456","readFile":{"path":"/tmp/test.txt"}}}"#;
        let req: ClientRequest = serde_json::from_str(json_str).unwrap();
        
        let parsed = req.parse().unwrap();
        assert_eq!(parsed.id, "req-456");
        assert_eq!(parsed.method, "readFile");
        assert_eq!(parsed.params["path"], "/tmp/test.txt");
    }

    #[test]
    fn test_server_response_success() {
        let resp = ServerResponse::success(
            "req-123".to_string(),
            "ping".to_string(),
            json!({"message": "pong"}),
        );
        
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("serverResponse"));
        assert!(json.contains("req-123"));
        assert!(json.contains("ping"));
        assert!(json.contains("pong"));
    }

    #[test]
    fn test_server_response_error() {
        let error = ErrorObject {
            code: -32602,
            message: "Invalid params".to_string(),
            data: Some(json!("Missing path parameter")),
        };
        
        let resp = ServerResponse::error("req-456".to_string(), error);
        let json = serde_json::to_string(&resp).unwrap();
        
        assert!(json.contains("serverResponse"));
        assert!(json.contains("req-456"));
        assert!(json.contains("error"));
        assert!(json.contains("-32602"));
    }

    #[test]
    fn test_server_notification() {
        let notif = ServerNotification::new(
            "selectionDidChange".to_string(),
            json!({"uri": "file:///test.txt"}),
        );
        
        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains("serverNotification"));
        assert!(json.contains("selectionDidChange"));
        assert!(json.contains("file:///test.txt"));
    }

    #[test]
    fn test_roundtrip_client_request() {
        let original = r#"{"clientRequest":{"id":"test","authenticate":{}}}"#;
        let req: ClientRequest = serde_json::from_str(original).unwrap();
        let serialized = serde_json::to_string(&req).unwrap();
        let req2: ClientRequest = serde_json::from_str(&serialized).unwrap();
        
        let parsed = req2.parse().unwrap();
        assert_eq!(parsed.method, "authenticate");
    }
}
