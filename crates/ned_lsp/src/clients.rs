//! Generic Language Server Protocol client engine.
//!
//! Provides asynchronous request/response tracking, notification dispatch,
//! document synchronization, and diagnostics polling without any hardcoded
//! language servers or assumptions.

use std::collections::HashMap;
use std::io;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::jsonrpc::{notification, request, rpc_error};
use crate::protocol::{
    did_change_text_document_params, did_close_text_document_params, did_open_text_document_params,
    did_save_text_document_params, hover, location, publish_diagnostics_params,
    text_document_content_change_event, text_document_identifier, text_document_item,
    text_document_position_params, versioned_text_document_identifier,
};
use crate::transports::transport;

pub mod lifecycle;
pub use lifecycle::server_config;

/// Generic LSP client driving a single language server instance.
pub struct client {
    pub(crate) config: server_config,
    pub(crate) transport: Arc<transport>,
    pub(crate) next_id: AtomicU64,
    pub(crate) pending_requests: Arc<Mutex<HashMap<u64, Sender<Result<Value, rpc_error>>>>>,
    pub(crate) diagnostics_rx: Receiver<publish_diagnostics_params>,
    pub(crate) is_active: Arc<AtomicBool>,
}

impl client {
    /// Starts a new language server client from the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the subprocess fails to spawn or pipe stdio.
    #[inline]
    pub fn start(config: server_config) -> io::Result<Self> {
        lifecycle::start(config)
    }

    /// Returns the configuration associated with this client.
    #[inline]
    #[must_use]
    pub fn config(&self) -> &server_config {
        &self.config
    }

    /// Checks if the language server process is still alive.
    #[inline]
    #[must_use]
    pub fn is_alive(&self) -> bool {
        lifecycle::is_alive(self)
    }

    /// Sends a JSON-RPC request and returns a receiver for the asynchronous response.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if serialization or sending fails.
    pub fn send_request<P: Serialize>(
        &self,
        method: &str,
        params: Option<P>,
    ) -> io::Result<Receiver<Result<Value, rpc_error>>> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let params_val = match params {
            Some(p) => Some(
                serde_json::to_value(p)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            ),
            None => None,
        };

        let req = request::new(id, method, params_val);
        let (tx, rx) = mpsc::channel();

        if let Ok(mut map) = self.pending_requests.lock() {
            map.insert(id, tx);
        }

        self.transport.send(&req)?;
        Ok(rx)
    }

    /// Sends a JSON-RPC request and waits up to `timeout` for the response.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` on transport failure, timeout, or server error.
    pub fn send_request_timeout<P: Serialize>(
        &self,
        method: &str,
        params: Option<P>,
        timeout: Duration,
    ) -> io::Result<Value> {
        let rx = self.send_request(method, params)?;
        match rx.recv_timeout(timeout) {
            Ok(Ok(val)) => Ok(val),
            Ok(Err(rpc_err)) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("LSP server error: {rpc_err}"),
            )),
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
        &self,
        method: &str,
        params: Option<P>,
    ) -> io::Result<()> {
        let params_val = match params {
            Some(p) => Some(
                serde_json::to_value(p)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            ),
            None => None,
        };

        let notif = notification::new(method, params_val);
        self.transport.send(&notif)
    }

    /// Sends the standard `initialize` request and waits for capabilities.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if server fails to initialize or times out.
    #[inline]
    pub fn initialize(&self, timeout: Duration) -> io::Result<Value> {
        lifecycle::initialize(self, timeout)
    }

    /// Sends the standard `initialized` notification after initialization finishes.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if notification fails to send.
    #[inline]
    pub fn initialized(&self) -> io::Result<()> {
        lifecycle::initialized(self)
    }

    /// Notifies the language server that a document was opened (`textDocument/didOpen`).
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` on transport failure.
    pub fn notify_did_open(
        &self,
        uri: &str,
        language_id: &str,
        version: i32,
        text: &str,
    ) -> io::Result<()> {
        let params = did_open_text_document_params {
            text_document: text_document_item {
                uri: uri.to_string(),
                language_id: language_id.to_string(),
                version,
                text: text.to_string(),
            },
        };
        self.send_notification("textDocument/didOpen", Some(params))
    }

    /// Notifies the language server of document changes (`textDocument/didChange`).
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` on transport failure.
    pub fn notify_did_change(&self, uri: &str, version: i32, text: &str) -> io::Result<()> {
        let params = did_change_text_document_params {
            text_document: versioned_text_document_identifier {
                uri: uri.to_string(),
                version,
            },
            content_changes: vec![text_document_content_change_event {
                range: None,
                text: text.to_string(),
            }],
        };
        self.send_notification("textDocument/didChange", Some(params))
    }

    /// Notifies the language server that a document was saved (`textDocument/didSave`).
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` on transport failure.
    pub fn notify_did_save(&self, uri: &str) -> io::Result<()> {
        let params = did_save_text_document_params {
            text_document: text_document_identifier::new(uri),
            text: None,
        };
        self.send_notification("textDocument/didSave", Some(params))
    }

    /// Notifies the language server that a document was closed (`textDocument/didClose`).
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` on transport failure.
    pub fn notify_did_close(&self, uri: &str) -> io::Result<()> {
        let params = did_close_text_document_params {
            text_document: text_document_identifier::new(uri),
        };
        self.send_notification("textDocument/didClose", Some(params))
    }

    /// Polls for newly arrived diagnostics from the server without blocking.
    #[must_use]
    pub fn poll_diagnostics(&self) -> Option<publish_diagnostics_params> {
        self.diagnostics_rx.try_recv().ok()
    }

    /// Queries definition location at position (`textDocument/definition`).
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` on request failure or timeout.
    pub fn request_definition(
        &self,
        uri: &str,
        line: u32,
        character: u32,
        timeout: Duration,
    ) -> io::Result<Option<Vec<location>>> {
        let params = text_document_position_params::new(uri, line, character);
        let resp = self.send_request_timeout("textDocument/definition", Some(params), timeout)?;
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
        &self,
        uri: &str,
        line: u32,
        character: u32,
        timeout: Duration,
    ) -> io::Result<Option<hover>> {
        let params = text_document_position_params::new(uri, line, character);
        let resp = self.send_request_timeout("textDocument/hover", Some(params), timeout)?;
        if resp.is_null() {
            return Ok(None);
        }
        let parsed = serde_json::from_value::<hover>(resp)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Some(parsed))
    }

    /// Shuts down the language server cleanly.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` on transport failure or timeout.
    #[inline]
    pub fn shutdown(&self, timeout: Duration) -> io::Result<()> {
        lifecycle::shutdown(self, timeout)
    }
}
