//! LSP client diagnostic polling and processing.

use crate::clients::client;
use crate::protocol::publish_diagnostics_params;

/// Polls for newly arrived diagnostics from the server without blocking.
#[must_use]
pub fn poll_diagnostics(client: &client) -> Option<publish_diagnostics_params> {
    client.diagnostics_rx.try_recv().ok()
}
