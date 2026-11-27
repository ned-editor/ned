//! Language server process lifecycle, initialization handshake, and shutdown.

use std::collections::HashMap;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::Value;

use crate::clients::client;
use crate::jsonrpc::{incoming_message, rpc_error};
use crate::protocol::{client_info, initialize_params, publish_diagnostics_params};
use crate::transports::transport;

/// Configuration defining how to spawn a language server.
///
/// Contains zero hardcoded defaults; all commands and arguments
/// are provided dynamically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct server_config {
    /// Human-readable label for this server instance.
    pub name: String,
    /// Executable binary path or command name.
    pub command: String,
    /// Command-line arguments.
    pub args: Vec<String>,
    /// Optional root directory path of the workspace.
    pub root_path: Option<String>,
}

impl server_config {
    /// Creates a new server configuration.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
        root_path: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            args,
            root_path,
        }
    }
}

/// Spawns a new language server client subprocess and starts its reader thread.
pub fn start(config: server_config) -> io::Result<client> {
    let (trans, incoming_rx) =
        transport::spawn(&config.command, &config.args, config.root_path.as_deref())?;
    let transport = Arc::new(trans);

    let pending_requests: Arc<Mutex<HashMap<u64, Sender<Result<Value, rpc_error>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let (diag_tx, diag_rx) = mpsc::channel();
    let is_active = Arc::new(AtomicBool::new(true));

    let pending_map = Arc::clone(&pending_requests);
    let active_flag = Arc::clone(&is_active);

    thread::Builder::new()
        .name(format!("lsp-dispatcher-{}", config.name))
        .spawn(move || {
            while active_flag.load(Ordering::Relaxed) {
                let msg = match incoming_rx.recv() {
                    Ok(m) => m,
                    Err(_) => break,
                };

                match msg {
                    incoming_message::response(resp) => {
                        if let Ok(mut map) = pending_map.lock() {
                            if let Some(tx) = map.remove(&resp.id) {
                                if let Some(err) = resp.error {
                                    let _ = tx.send(Err(err));
                                } else {
                                    let _ = tx.send(Ok(resp.result.unwrap_or(Value::Null)));
                                }
                            }
                        }
                    }
                    incoming_message::notification(notif) => {
                        if notif.method == "textDocument/publishDiagnostics" {
                            if let Some(params) = notif.params {
                                if let Ok(parsed) =
                                    serde_json::from_value::<publish_diagnostics_params>(params)
                                {
                                    let _ = diag_tx.send(parsed);
                                }
                            }
                        }
                    }
                    incoming_message::request(_) => {
                        // Server-to-client request; ignored or acknowledged in core engine
                    }
                }
            }
            active_flag.store(false, Ordering::Relaxed);
        })?;

    Ok(client {
        config,
        transport,
        next_id: AtomicU64::new(1),
        pending_requests,
        diagnostics_rx: diag_rx,
        is_active,
    })
}

/// Checks if the language server process is still alive.
#[inline]
#[must_use]
pub fn is_alive(cl: &client) -> bool {
    cl.is_active.load(Ordering::Relaxed) && cl.transport.is_alive()
}

/// Sends the standard `initialize` request and waits for capabilities.
pub fn initialize(cl: &client, timeout: Duration) -> io::Result<Value> {
    let root_uri = cl.config.root_path.as_ref().map(|p| format!("file://{p}"));

    let params = initialize_params {
        process_id: Some(std::process::id()),
        root_uri,
        capabilities: serde_json::json!({
            "textDocument": {
                "synchronization": {
                    "dynamicRegistration": false,
                    "willSave": false,
                    "willSaveWaitUntil": false,
                    "didSave": true,
                },
                "hover": {
                    "dynamicRegistration": false,
                    "contentFormat": ["markdown", "plaintext"],
                },
                "definition": {
                    "dynamicRegistration": false,
                },
            }
        }),
        client_info: Some(client_info {
            name: "ned".to_string(),
            version: Some("0.1.0".to_string()),
        }),
    };

    cl.send_request_timeout("initialize", Some(params), timeout)
}

/// Sends the standard `initialized` notification after initialization finishes.
#[inline]
pub fn initialized(cl: &client) -> io::Result<()> {
    cl.send_notification::<Value>("initialized", Some(serde_json::json!({})))
}

/// Shuts down the language server cleanly.
pub fn shutdown(cl: &client, timeout: Duration) -> io::Result<()> {
    let _ = cl.send_request_timeout::<Value>("shutdown", None, timeout);
    let _ = cl.send_notification::<Value>("exit", None);
    cl.is_active.store(false, Ordering::Relaxed);
    let _ = cl.transport.terminate();
    Ok(())
}
