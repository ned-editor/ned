//! Ex-command execution handling (:write, :quit, :open, :set, :sh, etc.).

use std::fs;
use std::path::Path;
use std::process::Command;

use ned_text::selection;

use crate::command_parser::{editor_command, parse_command};
use crate::editors::editor;

/// Executes an ex-command string parsed from user prompt input.
pub fn execute_command(ed: &mut editor, cmd_str: &str) {
    match parse_command(cmd_str) {
        Ok(editor_command::run_shell(cmd)) => {
            #[cfg(windows)]
            let output = Command::new("cmd").args(["/C", &cmd]).output();
            #[cfg(not(windows))]
            let output = Command::new("sh").arg("-c").arg(&cmd).output();

            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    if out.status.success() {
                        let msg = if stdout.is_empty() {
                            "Command run".to_string()
                        } else {
                            stdout
                        };
                        let first_line = msg.lines().next().unwrap_or("").to_string();
                        if msg.contains('\n') {
                            ed.hover_popup = Some(msg);
                        }
                        ed.status_message = Some(first_line);
                    } else {
                        let err = if stderr.is_empty() {
                            format!("exit code {}", out.status)
                        } else {
                            stderr
                        };
                        ed.status_message = Some(format!("shell error: {err}"));
                    }
                }
                Err(e) => ed.status_message = Some(format!("shell error: {e}")),
            }
        }
        Ok(editor_command::write { path, force }) => {
            let doc_id = ed.active_doc_id;
            if force {
                if let Some(ref p) = path {
                    if let Some(parent) = Path::new(p).parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                } else if let Some(p) = ed.active_doc().path() {
                    if let Some(parent) = p.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                }
            }
            let res = {
                let doc = ed.active_doc_mut();
                if let Some(p) = path {
                    doc.save_to(p)
                } else {
                    doc.save()
                }
            };
            match res {
                Ok(()) => {
                    ed.status_message = Some("written".to_string());
                    super::git_ops::refresh_diff(ed, doc_id);
                }
                Err(e) => ed.status_message = Some(format!("write error: {e}")),
            }
        }
        Ok(editor_command::write_quit { path, force }) => {
            let _doc_id = ed.active_doc_id;
            if force {
                if let Some(ref p) = path {
                    if let Some(parent) = Path::new(p).parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                } else if let Some(p) = ed.active_doc().path() {
                    if let Some(parent) = p.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                }
            }
            let res = {
                let doc = ed.active_doc_mut();
                if let Some(p) = path {
                    doc.save_to(p)
                } else {
                    doc.save()
                }
            };
            match res {
                Ok(()) => {
                    ed.should_quit = true;
                }
                Err(e) => ed.status_message = Some(format!("write error: {e}")),
            }
        }
        Ok(editor_command::write_all { force }) => {
            let mut errors = Vec::new();
            for (&doc_id, doc) in &mut ed.documents {
                if doc.is_modified() {
                    if force {
                        if let Some(p) = doc.path() {
                            if let Some(parent) = p.parent() {
                                let _ = fs::create_dir_all(parent);
                            }
                        }
                    }
                    if let Err(e) = doc.save() {
                        errors.push(format!("{doc_id}: {e}"));
                    }
                }
            }
            if errors.is_empty() {
                ed.status_message = Some("all modified buffers written".to_string());
            } else {
                ed.status_message = Some(format!("write-all errors: {}", errors.join("; ")));
            }
        }
        Ok(editor_command::write_quit_all { force }) => {
            let mut errors = Vec::new();
            for (&doc_id, doc) in &mut ed.documents {
                if doc.is_modified() {
                    if force {
                        if let Some(p) = doc.path() {
                            if let Some(parent) = p.parent() {
                                let _ = fs::create_dir_all(parent);
                            }
                        }
                    }
                    if let Err(e) = doc.save() {
                        errors.push(format!("{doc_id}: {e}"));
                    }
                }
            }
            if errors.is_empty() {
                ed.should_quit = true;
            } else {
                ed.status_message = Some(format!("write-quit-all errors: {}", errors.join("; ")));
            }
        }
        Ok(editor_command::quit { force }) => {
            let doc = ed.active_doc();
            if doc.is_modified() && !force {
                ed.status_message = Some("unsaved changes (use :q! to force)".to_string());
            } else {
                ed.should_quit = true;
            }
        }
        Ok(editor_command::quit_all { force }) => {
            let any_modified = ed.documents.values().any(super::super::documents::document::is_modified);
            if any_modified && !force {
                ed.status_message =
                    Some("unsaved changes in open buffers (use :qa! to force)".to_string());
            } else {
                ed.should_quit = true;
            }
        }
        Ok(editor_command::cquit { code, force }) => {
            let doc = ed.active_doc();
            if doc.is_modified() && !force {
                ed.status_message = Some("unsaved changes (use :cq! to force)".to_string());
            } else {
                ed.exit_code = Some(code.unwrap_or(1));
                ed.should_quit = true;
            }
        }
        Ok(editor_command::open(path)) => {
            if let Err(e) = ed.open_file(&path) {
                ed.status_message = Some(format!("error opening file: {e}"));
            }
        }
        Ok(editor_command::new_scratch) => {
            let next_id = ed.next_doc_id;
            ed.next_doc_id += 1;
            let doc = crate::documents::document::new_empty(next_id);
            ed.documents.insert(next_id, doc);
            ed.active_doc_id = next_id;
            let view_id = ed.active_view_id;
            if let Some(v) = ed.views.get_mut(&view_id) {
                v.doc_id = next_id;
                v.row_offset = 0;
                v.col_offset = 0;
            }
            ed.status_message = Some("[scratch]".to_string());
        }
        Ok(editor_command::buffer_close { force }) => {
            close_current_buffer(ed, force);
        }
        Ok(editor_command::buffer_close_others { force }) => {
            let active_id = ed.active_doc_id;
            let other_ids: Vec<_> = ed
                .documents
                .keys()
                .copied()
                .filter(|&id| id != active_id)
                .collect();
            let mut unsaved = false;
            if !force {
                for id in &other_ids {
                    if let Some(d) = ed.documents.get(id) {
                        if d.is_modified() {
                            unsaved = true;
                            break;
                        }
                    }
                }
            }
            if unsaved {
                ed.status_message =
                    Some("other buffers have unsaved changes (use :bco! to force)".to_string());
            } else {
                for id in other_ids {
                    ed.documents.remove(&id);
                }
                ed.status_message = Some("closed other buffers".to_string());
            }
        }
        Ok(editor_command::buffer_close_all { force }) => {
            let any_modified = ed.documents.values().any(super::super::documents::document::is_modified);
            if any_modified && !force {
                ed.status_message =
                    Some("buffers have unsaved changes (use :bca! to force)".to_string());
            } else {
                ed.documents.clear();
                let next_id = ed.next_doc_id;
                ed.next_doc_id += 1;
                let doc = crate::documents::document::new_empty(next_id);
                ed.documents.insert(next_id, doc);
                ed.active_doc_id = next_id;
                let view_id = ed.active_view_id;
                if let Some(v) = ed.views.get_mut(&view_id) {
                    v.doc_id = next_id;
                    v.row_offset = 0;
                    v.col_offset = 0;
                }
                ed.status_message = Some("closed all buffers".to_string());
            }
        }
        Ok(editor_command::buffer_next) => {
            cycle_buffer(ed, true);
        }
        Ok(editor_command::buffer_previous) => {
            cycle_buffer(ed, false);
        }
        Ok(editor_command::reload { all }) => {
            if all {
                for doc in ed.documents.values_mut() {
                    if let Some(p) = doc.path().map(Path::to_path_buf) {
                        let id = doc.id();
                        let lang = doc.language().cloned();
                        if let Ok(new_doc) = crate::documents::document::open(id, &p, lang) {
                            *doc = new_doc;
                        }
                    }
                }
                ed.status_message = Some("reloaded all buffers from disk".to_string());
                super::git_ops::refresh_diff(ed, ed.active_doc_id);
            } else {
                let doc = ed.active_doc_mut();
                if let Some(p) = doc.path().map(Path::to_path_buf) {
                    let id = doc.id();
                    let lang = doc.language().cloned();
                    match crate::documents::document::open(id, &p, lang) {
                        Ok(new_doc) => {
                            *doc = new_doc;
                            ed.status_message = Some("reloaded buffer from disk".to_string());
                            super::git_ops::refresh_diff(ed, id);
                        }
                        Err(e) => ed.status_message = Some(format!("reload error: {e}")),
                    }
                } else {
                    ed.status_message = Some("cannot reload unsaved scratch buffer".to_string());
                }
            }
        }
        Ok(editor_command::update) => {
            let doc_id = ed.active_doc_id;
            let doc = ed.active_doc_mut();
            if doc.is_modified() {
                match doc.save() {
                    Ok(()) => {
                        ed.status_message = Some("written".to_string());
                        super::git_ops::refresh_diff(ed, doc_id);
                    }
                    Err(e) => ed.status_message = Some(format!("write error: {e}")),
                }
            } else {
                ed.status_message = Some("buffer not modified".to_string());
            }
        }
        Ok(editor_command::theme(name_opt)) => {
            if let Some(name) = name_opt {
                ed.status_message = Some(format!("theme set to {name}"));
            } else {
                ed.status_message = Some("current theme: default".to_string());
            }
        }
        Ok(editor_command::format) => {
            ed.status_message = Some("formatted".to_string());
        }
        Ok(editor_command::set_option(k, v)) => {
            match k.as_str() {
                "scrolloff" => {
                    if let Ok(val) = v.parse::<usize>() {
                        ed.config.editor.scrolloff = val;
                    }
                }
                "tab-width" => {
                    if let Ok(val) = v.parse::<usize>() {
                        ed.config.editor.tab_width = val;
                    }
                }
                "cursor-line" => {
                    if let Ok(val) = v.parse::<bool>() {
                        ed.config.editor.cursor_line = val;
                    }
                }
                _ => {}
            }
            ed.status_message = Some(format!("set {k} = {v}"));
        }
        Ok(editor_command::get_option(k)) => {
            let val = match k.as_str() {
                "scrolloff" => ed.config.editor.scrolloff.to_string(),
                "tab-width" => ed.config.editor.tab_width.to_string(),
                "cursor-line" => ed.config.editor.cursor_line.to_string(),
                _ => "unknown option".to_string(),
            };
            ed.status_message = Some(format!("{k} = {val}"));
        }
        Ok(editor_command::toggle_option(k)) => {
            if k == "cursor-line" {
                ed.config.editor.cursor_line = !ed.config.editor.cursor_line;
                ed.status_message = Some(format!("cursor-line = {}", ed.config.editor.cursor_line));
            } else {
                ed.status_message = Some(format!("toggled {k}"));
            }
        }
        Ok(editor_command::goto_line(target_line)) => {
            let _doc_id = ed.active_doc_id;
            let doc = ed.active_doc_mut();
            let total = doc.text().line_count().max(1);
            let clamped = target_line.clamp(1, total) - 1;
            let byte_idx = doc.text().line_to_byte(clamped).unwrap_or(0);
            let char_idx = doc.text().byte_to_char(byte_idx);
            doc.set_selection(selection::single(char_idx, char_idx));
            ed.ensure_active_cursor_visible();
        }
        Ok(editor_command::earlier(count)) => {
            for _ in 0..count {
                if !ed.active_doc_mut().undo() {
                    break;
                }
            }
            ed.ensure_active_cursor_visible();
        }
        Ok(editor_command::later(count)) => {
            for _ in 0..count {
                if !ed.active_doc_mut().redo() {
                    break;
                }
            }
            ed.ensure_active_cursor_visible();
        }
        Ok(editor_command::change_directory(path_opt)) => {
            let target = path_opt
                .unwrap_or_else(|| std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
            match std::env::set_current_dir(&target) {
                Ok(()) => ed.status_message = Some(format!("working directory: {target}")),
                Err(e) => ed.status_message = Some(format!("error changing directory: {e}")),
            }
        }
        Ok(editor_command::show_directory) => {
            let cwd = std::env::current_dir().map_or_else(|_| "unknown".to_string(), |p| p.display().to_string());
            ed.status_message = Some(cwd);
        }
        Ok(editor_command::character_info) => {
            let doc = ed.active_doc();
            let primary = doc.selection().primary().cursor(doc.text());
            let ch = doc.text().char_at(primary).unwrap_or('\0');
            ed.status_message = Some(format!("'{ch}' U+{:04X} (char {primary})", ch as u32));
        }
        Ok(editor_command::clear_register(reg_opt)) => {
            if let Some(r) = reg_opt {
                let _ = ed.registers.write(r, Vec::new());
                ed.status_message = Some(format!("cleared register '{r}'"));
            } else {
                ed.registers = crate::registers::register_storage::new();
                ed.status_message = Some("cleared all registers".to_string());
            }
        }
        Ok(editor_command::echo(msg)) => {
            ed.status_message = Some(msg);
        }
        Ok(editor_command::noop) => {}
        Err(e) => {
            ed.status_message = Some(e);
        }
    }
}

fn close_current_buffer(ed: &mut editor, force: bool) {
    let doc_id = ed.active_doc_id;
    if let Some(doc) = ed.documents.get(&doc_id) {
        if doc.is_modified() && !force {
            ed.status_message = Some("unsaved changes (use :bc! to force)".to_string());
            return;
        }
    }

    if ed.documents.len() > 1 {
        ed.documents.remove(&doc_id);
        let next_id = *ed.documents.keys().next().unwrap();
        ed.active_doc_id = next_id;
        let view_id = ed.active_view_id;
        if let Some(v) = ed.views.get_mut(&view_id) {
            v.doc_id = next_id;
            v.row_offset = 0;
            v.col_offset = 0;
        }
        ed.status_message = Some("buffer closed".to_string());
    } else {
        ed.documents.clear();
        let next_id = ed.next_doc_id;
        ed.next_doc_id += 1;
        ed.documents
            .insert(next_id, crate::documents::document::new_empty(next_id));
        ed.active_doc_id = next_id;
        let view_id = ed.active_view_id;
        if let Some(v) = ed.views.get_mut(&view_id) {
            v.doc_id = next_id;
            v.row_offset = 0;
            v.col_offset = 0;
        }
        ed.status_message = Some("buffer closed".to_string());
    }
}

fn cycle_buffer(ed: &mut editor, forward: bool) {
    let mut ids: Vec<_> = ed.documents.keys().copied().collect();
    ids.sort_unstable();
    if ids.len() <= 1 {
        return;
    }
    let curr_idx = ids
        .iter()
        .position(|&id| id == ed.active_doc_id)
        .unwrap_or(0);
    let next_idx = if forward {
        (curr_idx + 1) % ids.len()
    } else if curr_idx == 0 {
        ids.len() - 1
    } else {
        curr_idx - 1
    };
    let next_id = ids[next_idx];
    ed.active_doc_id = next_id;
    let view_id = ed.active_view_id;
    if let Some(v) = ed.views.get_mut(&view_id) {
        v.doc_id = next_id;
        v.row_offset = 0;
        v.col_offset = 0;
    }
    let doc = ed.active_doc();
    let name = doc
        .path()
        .map_or("[scratch]".to_string(), |p| p.to_string_lossy().to_string());
    ed.status_message = Some(name);
}
