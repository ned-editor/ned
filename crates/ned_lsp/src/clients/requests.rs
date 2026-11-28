//! LSP client request dispatch and query helpers.

use std::io;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::clients::client;
use crate::jsonrpc::{notification, request, rpc_error};
use crate::protocol::{hover, location, text_document_position_params};

/// Sends a JSON-RPC request and returns a receiver for the asynchronous response.
///
/// # Errors
///
/// Returns an `io::Error` if serialization or sending fails.
pub fn send_request<P: Serialize>(
    client: &client,
    method: &str,
    params: Option<P>,
) -> io::Result<Receiver<Result<Value, rpc_error>>> {
    let id = client.next_id.fetch_add(1, Ordering::SeqCst);
    let params_val = match params {
        Some(p) => Some(
            serde_json::to_value(p).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        ),
        None => None,
    };

    let req = request::new(id, method, params_val);
    let (tx, rx): (
        Sender<Result<Value, rpc_error>>,
        Receiver<Result<Value, rpc_error>>,
    ) = mpsc::channel();

    if let Ok(mut map) = client.pending_requests.lock() {
        map.insert(id, tx);
    }

    client.transport.send(&req)?;
    Ok(rx)
}

/// Sends a JSON-RPC request and waits up to `timeout` for the response.
///
/// # Errors
///
/// Returns an `io::Error` on transport failure, timeout, or server error.
pub fn send_request_timeout<P: Serialize>(
    client: &client,
    method: &str,
    params: Option<P>,
    timeout: Duration,
) -> io::Result<Value> {
    let rx = send_request(client, method, params)?;
    match rx.recv_timeout(timeout) {
        Ok(Ok(val)) => Ok(val),
        Ok(Err(rpc_err)) => Err(io::Error::other(format!("LSP server error: {rpc_err}"))),
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("LSP request '{method}' timed out after {timeout:?}"),
        )),
    }
}

/// Sends a JSON-RPC notification (no response expected).
///
/// # Errors
///
/// Returns an `io::Error` if serialization or sending fails.
pub fn send_notification<P: Serialize>(
    client: &client,
    method: &str,
    params: Option<P>,
) -> io::Result<()> {
    let params_val = match params {
        Some(p) => Some(
            serde_json::to_value(p).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        ),
        None => None,
    };

    let notif = notification::new(method, params_val);
    client.transport.send(&notif)
}

/// Queries definition location at position (`textDocument/definition`).
///
/// # Errors
///
/// Returns an `io::Error` on request failure or timeout.
pub fn request_definition(
    client: &client,
    uri: &str,
    line: u32,
    character: u32,
    timeout: Duration,
) -> io::Result<Option<Vec<location>>> {
    let params = text_document_position_params::new(uri, line, character);
    let resp = send_request_timeout(client, "textDocument/definition", Some(params), timeout)?;
    if resp.is_null() {
        return Ok(None);
    }

    if let Ok(locs) = serde_json::from_value::<Vec<location>>(resp.clone()) {
        Ok(Some(locs))
    } else if let Ok(loc) = serde_json::from_value::<location>(resp) {
        Ok(Some(vec![loc]))
    } else {
        Ok(None)
    }
}

/// Queries hover documentation at position (`textDocument/hover`).
///
/// # Errors
///
/// Returns an `io::Error` on request failure or timeout.
pub fn request_hover(
    client: &client,
    uri: &str,
    line: u32,
    character: u32,
    timeout: Duration,
) -> io::Result<Option<hover>> {
    let params = text_document_position_params::new(uri, line, character);
    let resp = send_request_timeout(client, "textDocument/hover", Some(params), timeout)?;
    if resp.is_null() {
        return Ok(None);
    }
    let parsed = serde_json::from_value::<hover>(resp)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(Some(parsed))
}
