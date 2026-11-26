//! Generic, standalone Language Server Protocol (LSP) client engine for ned.
//!
//! Provides JSON-RPC 2.0 stdio framing, asynchronous subprocess transport,
//! and protocol message types without hardcoded server binaries or language assumptions.

pub mod jsonrpc;
pub mod protocol;

pub use jsonrpc::{
    encode_payload, error_code, incoming_message, notification, read_message, request, response,
    rpc_error,
};
pub use protocol::{
    client_info, completion_item, diagnostic, diagnostic_severity, did_change_text_document_params,
    did_close_text_document_params, did_open_text_document_params, did_save_text_document_params,
    hover, initialize_params, location, position, publish_diagnostics_params, range,
    text_document_content_change_event, text_document_identifier, text_document_item,
    text_document_position_params, versioned_text_document_identifier,
};
