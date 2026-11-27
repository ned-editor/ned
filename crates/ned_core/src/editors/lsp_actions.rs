//! Language Server Protocol (LSP) actions: start, notifications, polling, hover, definition, diagnostics.

use ned_text::selection;

use crate::documents::document_id;
use crate::editors::editor;

/// Starts or connects the language server for `doc_id`.
pub fn start_lsp_for_doc(ed: &mut editor, doc_id: document_id) {
    let (lang_name, srv_cmd, srv_args, doc_path, doc_text, doc_version) = {
        let doc = match ed.documents.get(&doc_id) {
            Some(d) => d,
            None => return,
        };
        let lang = match doc.language() {
            Some(l) => l,
            None => return,
        };
        let lsp_cfg = match &lang.language_server {
            Some(c) => c,
            None => return,
        };
        let path = match doc.path() {
            Some(p) => p.to_path_buf(),
            None => return,
        };
        (
            lang.name.clone(),
            lsp_cfg.command.clone(),
            lsp_cfg.args.clone(),
            path,
            doc.text().to_string(),
            doc.version(),
        )
    };

    if !ed.lsp_clients.contains_key(&lang_name) {
        let root_path = ed
            .config
            .languages
            .find_root_for_path(&doc_path)
            .map(|p| p.to_string_lossy().to_string());
        let srv_cfg = ned_lsp::server_config::new(&lang_name, &srv_cmd, srv_args, root_path);
        if let Ok(c) = ned_lsp::client::start(srv_cfg) {
            if c.initialize(std::time::Duration::from_millis(2000)).is_ok() {
                let _ = c.initialized();
            }
            ed.lsp_clients.insert(lang_name.clone(), c);
        }
    }

    if let Some(client) = ed.lsp_clients.get(&lang_name) {
        let uri = format!("file://{}", doc_path.to_string_lossy());
        let _ = client.notify_did_open(&uri, &lang_name, doc_version, &doc_text);
    }
}

/// Notifies the connected LSP server that `doc_id` contents changed.
pub fn notify_lsp_did_change(ed: &editor, doc_id: document_id) {
    let Some(doc) = ed.documents.get(&doc_id) else {
        return;
    };
    let Some(lang) = doc.language() else {
        return;
    };
    let Some(path) = doc.path() else {
        return;
    };
    if let Some(client) = ed.lsp_clients.get(&lang.name) {
        let uri = format!("file://{}", path.to_string_lossy());
        let _ = client.notify_did_change(&uri, doc.version(), &doc.text().to_string());
    }
}

/// Notifies the connected LSP server that `doc_id` was saved.
pub fn notify_lsp_did_save(ed: &editor, doc_id: document_id) {
    let Some(doc) = ed.documents.get(&doc_id) else {
        return;
    };
    let Some(lang) = doc.language() else {
        return;
    };
    let Some(path) = doc.path() else {
        return;
    };
    if let Some(client) = ed.lsp_clients.get(&lang.name) {
        let uri = format!("file://{}", path.to_string_lossy());
        let _ = client.notify_did_save(&uri);
    }
}

/// Polls diagnostics from connected LSP clients and assigns them to matching documents.
///
/// Returns `true` if any diagnostics were received and applied.
pub fn poll_lsp(ed: &mut editor) -> bool {
    let mut new_diagnostics: Vec<(String, Vec<ned_lsp::diagnostic>)> = Vec::new();

    for client in ed.lsp_clients.values() {
        while let Some(params) = client.poll_diagnostics() {
            new_diagnostics.push((params.uri, params.diagnostics));
        }
    }

    if new_diagnostics.is_empty() {
        return false;
    }

    let mut updated = false;
    for (uri, diags) in new_diagnostics {
        for doc in ed.documents.values_mut() {
            if let Some(path) = doc.path() {
                let doc_uri = format!("file://{}", path.to_string_lossy());
                if doc_uri == uri || uri.ends_with(&path.to_string_lossy().to_string()) {
                    doc.set_diagnostics(diags.clone());
                    updated = true;
                }
            }
        }
    }
    updated
}

/// Jumps cursor to the next or previous diagnostic.
pub fn goto_diagnostic(ed: &mut editor, next: bool) {
    let doc_id = ed.active_doc_id;
    let (target_line, target_col, msg) = {
        let doc = match ed.documents.get(&doc_id) {
            Some(d) => d,
            None => return,
        };
        if doc.diagnostics().is_empty() {
            ed.status_message = Some("No diagnostics for this document".to_string());
            return;
        }
        let cursor_char = doc.selection().primary().cursor(doc.text());
        let cursor_byte = doc.text().char_to_byte(cursor_char);
        let current_line = doc.text().byte_to_line(cursor_byte) as u32;

        let mut sorted = doc.diagnostics().to_vec();
        sorted.sort_by_key(|d| (d.range.start.line, d.range.start.character));

        let candidate = if next {
            sorted
                .iter()
                .find(|d| d.range.start.line > current_line)
                .or_else(|| sorted.first())
        } else {
            sorted
                .iter()
                .rfind(|d| d.range.start.line < current_line)
                .or_else(|| sorted.last())
        };

        match candidate {
            Some(d) => (
                d.range.start.line as usize,
                d.range.start.character as usize,
                d.message.clone(),
            ),
            None => return,
        }
    };

    let doc = ed.active_doc_mut();
    let target_line_byte = doc.text().line_to_byte(target_line).unwrap_or(0);
    let target_line_char = doc.text().byte_to_char(target_line_byte);
    let target_char = (target_line_char + target_col).min(doc.text().char_len());
    doc.set_selection(selection::point(target_char));
    ed.ensure_active_cursor_visible();
    ed.status_message = Some(format!("[LSP] {msg}"));
}

/// Queries hover documentation for the symbol under cursor.
pub fn request_hover(ed: &mut editor) {
    let doc = ed.active_doc();
    let lang_name = match doc.language() {
        Some(l) => l.name.clone(),
        None => {
            ed.status_message = Some("No language server for this buffer".to_string());
            return;
        }
    };
    let path = match doc.path() {
        Some(p) => p.to_path_buf(),
        None => return,
    };

    let cursor_char = doc.selection().primary().cursor(doc.text());
    let cursor_byte = doc.text().char_to_byte(cursor_char);
    let line = doc.text().byte_to_line(cursor_byte);
    let line_start_byte = doc.text().line_to_byte(line).unwrap_or(0);
    let line_start_char = doc.text().byte_to_char(line_start_byte);
    let col = cursor_char.saturating_sub(line_start_char);

    if let Some(client) = ed.lsp_clients.get(&lang_name) {
        let uri = format!("file://{}", path.to_string_lossy());
        match client.request_hover(
            &uri,
            line as u32,
            col as u32,
            std::time::Duration::from_millis(500),
        ) {
            Ok(Some(h)) => {
                let text = match h.contents {
                    serde_json::Value::String(s) => s,
                    serde_json::Value::Object(map) => map
                        .get("value")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    other => other.to_string(),
                };
                let trimmed = text.trim().to_string();
                if trimmed.is_empty() {
                    ed.status_message = Some("No hover information available".to_string());
                } else {
                    ed.hover_popup = Some(trimmed);
                }
            }
            Ok(None) => {
                ed.status_message = Some("No hover information available".to_string());
            }
            Err(e) => {
                ed.status_message = Some(format!("Hover error: {e}"));
            }
        }
    } else {
        ed.status_message = Some("Language server not connected".to_string());
    }
}

/// Queries definition location and jumps to it.
pub fn request_definition(ed: &mut editor) {
    let doc = ed.active_doc();
    let lang_name = match doc.language() {
        Some(l) => l.name.clone(),
        None => return,
    };
    let path = match doc.path() {
        Some(p) => p.to_path_buf(),
        None => return,
    };

    let cursor_char = doc.selection().primary().cursor(doc.text());
    let cursor_byte = doc.text().char_to_byte(cursor_char);
    let line = doc.text().byte_to_line(cursor_byte);
    let line_start_byte = doc.text().line_to_byte(line).unwrap_or(0);
    let line_start_char = doc.text().byte_to_char(line_start_byte);
    let col = cursor_char.saturating_sub(line_start_char);

    let target_location = if let Some(client) = ed.lsp_clients.get(&lang_name) {
        let uri = format!("file://{}", path.to_string_lossy());
        match client.request_definition(
            &uri,
            line as u32,
            col as u32,
            std::time::Duration::from_millis(500),
        ) {
            Ok(Some(locs)) => locs.into_iter().next(),
            _ => None,
        }
    } else {
        None
    };

    if let Some(loc) = target_location {
        let target_path_str = loc.uri.strip_prefix("file://").unwrap_or(&loc.uri);
        let target_line = loc.range.start.line as usize;
        let target_col = loc.range.start.character as usize;

        let is_same_file = doc
            .path()
            .map_or(false, |p| p.to_string_lossy() == target_path_str);
        if !is_same_file {
            let _ = ed.open_file(target_path_str);
        }

        let doc = ed.active_doc_mut();
        let line_byte = doc.text().line_to_byte(target_line).unwrap_or(0);
        let line_char = doc.text().byte_to_char(line_byte);
        let target_char = (line_char + target_col).min(doc.text().char_len());
        doc.set_selection(selection::point(target_char));
        ed.ensure_active_cursor_visible();
        ed.status_message = Some(format!("Jumped to {target_path_str}:{target_line}"));
    } else {
        ed.status_message = Some("Definition not found".to_string());
    }
}
