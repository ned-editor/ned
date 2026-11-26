//! Generic, standalone Language Server Protocol (LSP) client engine for ned.
//!
//! Provides JSON-RPC 2.0 stdio framing, asynchronous subprocess transport,
//! and protocol message types without hardcoded server binaries or language assumptions.

pub mod jsonrpc;

pub use jsonrpc::{
    encode_payload, error_code, incoming_message, notification, read_message, request, response,
    rpc_error,
};
