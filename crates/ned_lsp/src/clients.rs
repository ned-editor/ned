//! Generic Language Server Protocol client engine.
//!
//! Provides asynchronous request/response tracking, notification dispatch,
//! document synchronization, and diagnostics polling without any hardcoded
//! language servers or assumptions.

use std::collections::HashMap;
use std::io;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::jsonrpc::rpc_error;
use crate::protocol::{hover, location, publish_diagnostics_params};
use crate::transports::transport;

pub mod diagnostics;
pub mod lifecycle;
pub mod requests;
pub mod sync;

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
    #[inline]
    pub fn send_request<P: Serialize>(
        &self,
        method: &str,
        params: Option<P>,
    ) -> io::Result<Receiver<Result<Value, rpc_error>>> {
        requests::send_request(self, method, params)
    }

    /// Sends a JSON-RPC request and waits up to `timeout` for the response.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` on transport failure, timeout, or server error.
    #[inline]
    pub fn send_request_timeout<P: Serialize>(
        &self,
        method: &str,
        params: Option<P>,
        timeout: Duration,
    ) -> io::Result<Value> {
        requests::send_request_timeout(self, method, params, timeout)
    }

    /// Sends a JSON-RPC notification (no response expected).
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if serialization or sending fails.
    #[inline]
    pub fn send_notification<P: Serialize>(
        &self,
        method: &str,
        params: Option<P>,
    ) -> io::Result<()> {
        requests::send_notification(self, method, params)
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
    #[inline]
    pub fn notify_did_open(
        &self,
        uri: &str,
        language_id: &str,
        version: i32,
        text: &str,
    ) -> io::Result<()> {
        sync::notify_did_open(self, uri, language_id, version, text)
    }

    /// Notifies the language server of document changes (`textDocument/didChange`).
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` on transport failure.
    #[inline]
    pub fn notify_did_change(&self, uri: &str, version: i32, text: &str) -> io::Result<()> {
        sync::notify_did_change(self, uri, version, text)
    }

    /// Notifies the language server that a document was saved (`textDocument/didSave`).
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` on transport failure.
    #[inline]
    pub fn notify_did_save(&self, uri: &str) -> io::Result<()> {
        sync::notify_did_save(self, uri)
    }

    /// Notifies the language server that a document was closed (`textDocument/didClose`).
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` on transport failure.
    #[inline]
    pub fn notify_did_close(&self, uri: &str) -> io::Result<()> {
        sync::notify_did_close(self, uri)
    }

    /// Polls for newly arrived diagnostics from the server without blocking.
    #[inline]
    #[must_use]
    pub fn poll_diagnostics(&self) -> Option<publish_diagnostics_params> {
        diagnostics::poll_diagnostics(self)
    }

    /// Queries definition location at position (`textDocument/definition`).
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` on request failure or timeout.
    #[inline]
    pub fn request_definition(
        &self,
        uri: &str,
        line: u32,
        character: u32,
        timeout: Duration,
    ) -> io::Result<Option<Vec<location>>> {
        requests::request_definition(self, uri, line, character, timeout)
    }

    /// Queries hover documentation at position (`textDocument/hover`).
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` on request failure or timeout.
    #[inline]
    pub fn request_hover(
        &self,
        uri: &str,
        line: u32,
        character: u32,
        timeout: Duration,
    ) -> io::Result<Option<hover>> {
        requests::request_hover(self, uri, line, character, timeout)
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
