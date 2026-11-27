//! Language Server Protocol document synchronization (`textDocument/did*`) operations.

use std::io;

use crate::clients::client;
use crate::protocol::{
    did_change_text_document_params, did_close_text_document_params, did_open_text_document_params,
    did_save_text_document_params, text_document_content_change_event, text_document_identifier,
    text_document_item, versioned_text_document_identifier,
};

/// Notifies the language server that a document was opened (`textDocument/didOpen`).
pub fn notify_did_open(
    cl: &client,
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
    cl.send_notification("textDocument/didOpen", Some(params))
}

/// Notifies the language server of document changes (`textDocument/didChange`).
pub fn notify_did_change(cl: &client, uri: &str, version: i32, text: &str) -> io::Result<()> {
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
    cl.send_notification("textDocument/didChange", Some(params))
}

/// Notifies the language server that a document was saved (`textDocument/didSave`).
pub fn notify_did_save(cl: &client, uri: &str) -> io::Result<()> {
    let params = did_save_text_document_params {
        text_document: text_document_identifier::new(uri),
        text: None,
    };
    cl.send_notification("textDocument/didSave", Some(params))
}

/// Notifies the language server that a document was closed (`textDocument/didClose`).
pub fn notify_did_close(cl: &client, uri: &str) -> io::Result<()> {
    let params = did_close_text_document_params {
        text_document: text_document_identifier::new(uri),
    };
    cl.send_notification("textDocument/didClose", Some(params))
}
