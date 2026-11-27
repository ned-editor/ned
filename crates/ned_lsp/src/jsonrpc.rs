//! JSON-RPC 2.0 specification data types and stdio framing.

use std::fmt;
use std::io::{self, BufRead};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Standard JSON-RPC error codes.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum error_code {
    /// Invalid JSON was received by the server (-32700).
    parse_error,
    /// The JSON sent is not a valid Request object (-32600).
    invalid_request,
    /// The method does not exist or is not available (-32601).
    method_not_found,
    /// Invalid method parameters (-32602).
    invalid_params,
    /// Internal JSON-RPC error (-32603).
    internal_error,
    /// Implementation-defined server error.
    server_error(i64),
}

impl error_code {
    /// Returns the raw integer code.
    #[must_use]
    pub const fn code(&self) -> i64 {
        match *self {
            Self::parse_error => -32700,
            Self::invalid_request => -32600,
            Self::method_not_found => -32601,
            Self::invalid_params => -32602,
            Self::internal_error => -32603,
            Self::server_error(code) => code,
        }
    }
}

impl From<i64> for error_code {
    fn from(code: i64) -> Self {
        match code {
            -32700 => Self::parse_error,
            -32600 => Self::invalid_request,
            -32601 => Self::method_not_found,
            -32602 => Self::invalid_params,
            -32603 => Self::internal_error,
            other => Self::server_error(other),
        }
    }
}

impl<'de> Deserialize<'de> for error_code {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let code: i64 = Deserialize::deserialize(deserializer)?;
        Ok(Self::from(code))
    }
}

impl Serialize for error_code {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i64(self.code())
    }
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct rpc_error {
    /// Error code.
    pub code: error_code,
    /// Short description of the error.
    pub message: String,
    /// Optional additional structured details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl fmt::Display for rpc_error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON-RPC error ({}): {}", self.code.code(), self.message)
    }
}

impl std::error::Error for rpc_error {}

/// Outgoing or incoming JSON-RPC 2.0 request.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct request {
    /// Protocol version, always `"2.0"`.
    pub jsonrpc: String,
    /// Unique request identifier.
    pub id: u64,
    /// Target method name.
    pub method: String,
    /// Optional method parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl request {
    /// Creates a new request with id, method, and parameters.
    #[must_use]
    pub fn new(id: u64, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

/// Outgoing or incoming JSON-RPC 2.0 notification (no response expected).
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct notification {
    /// Protocol version, always `"2.0"`.
    pub jsonrpc: String,
    /// Notification method name.
    pub method: String,
    /// Optional notification parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl notification {
    /// Creates a new notification with method and parameters.
    #[must_use]
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }
}

/// Server response to a prior JSON-RPC 2.0 request.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct response {
    /// Protocol version, always `"2.0"`.
    pub jsonrpc: String,
    /// Matching request identifier.
    pub id: u64,
    /// Result payload if call succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error payload if call failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<rpc_error>,
}

/// Dispatched incoming message from a language server.
#[derive(Debug, PartialEq, Clone)]
pub enum incoming_message {
    /// Response to a client request.
    response(response),
    /// Server notification (e.g. publishDiagnostics).
    notification(notification),
    /// Server-to-client request (e.g. client/registerCapability).
    request(request),
}

/// Encodes any serializable payload into standard LSP framing:
/// `Content-Length: <n>\r\n\r\n<json>`
///
/// # Errors
///
/// Returns an error if serialization to JSON fails.
pub fn encode_payload<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    let json_bytes = serde_json::to_vec(value)?;
    let header = format!("Content-Length: {}\r\n\r\n", json_bytes.len());
    let mut framed = Vec::with_capacity(header.len() + json_bytes.len());
    framed.extend_from_slice(header.as_bytes());
    framed.extend_from_slice(&json_bytes);
    Ok(framed)
}

/// Reads the next JSON-RPC message from a buffered reader implementing LSP headers.
///
/// Returns `Ok(None)` if EOF is reached.
///
/// # Errors
///
/// Returns an `io::Error` if framing is malformed or if underlying read fails.
pub fn read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<incoming_message>> {
    let mut content_length: Option<usize> = None;

    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Ok(None);
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }

        if let Some(stripped) = trimmed.strip_prefix("Content-Length:") {
            let length_str = stripped.trim();
            let parsed: usize = length_str.parse().map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid Content-Length header: {e}"),
                )
            })?;
            content_length = Some(parsed);
        }
    }

    let length = content_length.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "missing Content-Length header in LSP message",
        )
    })?;

    let mut body = vec![0u8; length];
    reader.read_exact(&mut body)?;

    // Parse JSON
    let val: Value = serde_json::from_slice(&body).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to parse LSP JSON payload: {e}"),
        )
    })?;

    // Classify into response, notification, or request
    if val.get("id").is_some() {
        if val.get("method").is_some() {
            let req: request = serde_json::from_value(val)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            Ok(Some(incoming_message::request(req)))
        } else {
            let resp: response = serde_json::from_value(val)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            Ok(Some(incoming_message::response(resp)))
        }
    } else {
        let notif: notification = serde_json::from_value(val)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Some(incoming_message::notification(notif)))
    }
}
