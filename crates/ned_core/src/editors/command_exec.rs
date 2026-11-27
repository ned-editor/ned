//! Ex-command execution handling (:write, :quit, :open, :set, etc.).

use crate::command_parser::{editor_command, parse_command};
use crate::editors::editor;

/// Executes an ex-command string parsed from user prompt input.
pub fn execute_command(ed: &mut editor, cmd_str: &str) {
    match parse_command(cmd_str) {
        Ok(editor_command::write(path_opt)) => {
            let doc_id = ed.active_doc_id;
            let res = {
                let doc = ed.active_doc_mut();
                if let Some(path) = path_opt {
                    doc.save_to(path)
                } else {
                    doc.save()
                }
            };
            match res {
                Ok(()) => {
                    ed.status_message = Some("written".to_string());
                    ed.notify_lsp_did_save(doc_id);
                }
                Err(e) => ed.status_message = Some(format!("write error: {e}")),
            }
        }
        Ok(editor_command::quit) => {
            let doc = ed.active_doc();
            if doc.is_modified() {
                ed.status_message = Some("unsaved changes (use :q! to force)".to_string());
            } else {
                ed.should_quit = true;
            }
        }
        Ok(editor_command::force_quit) => {
            ed.should_quit = true;
        }
        Ok(editor_command::write_and_quit(path_opt)) => {
            let doc = ed.active_doc_mut();
            let res = if let Some(path) = path_opt {
                doc.save_to(path)
            } else {
                doc.save()
            };
            if res.is_ok() {
                ed.should_quit = true;
            } else if let Err(e) = res {
                ed.status_message = Some(format!("write error: {e}"));
            }
        }
        Ok(editor_command::open(path)) => {
            if let Err(e) = ed.open_file(&path) {
                ed.status_message = Some(format!("error opening file: {e}"));
            }
        }
        Ok(editor_command::set_option(key, val)) => {
            ed.status_message = Some(format!("set {key} = {val}"));
        }
        Err(e) => {
            ed.status_message = Some(e);
        }
    }
}
