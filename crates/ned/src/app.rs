//! Application lifecycle, Yokio event loop, and terminal event dispatch.

use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

use ::ned_config::{cursor_shape, mode, ned_config};
use ned_core::editor;
use ned_tui::input::events::{
    Event, KeyCode, KeyEvent, KeyModifier, MouseButton, MouseEvent, MouseEventKind,
};
use ned_tui::input::InputWatcher;
use ned_tui::output::command::{
    CaptureMouse, EnterRawMode, TerminalCommander, UseAlternateScreen, WrapLines,
};
use ned_tui::Control;
use ned_tui::Terminal;
use ned_tui::Termit;

use crate::input_mapping::translate_key_event;
use crate::picker::{picker_action, picker_state};
use crate::prompt::{prompt_action, prompt_state};
use crate::ui::{cursor_render_info, render_frame};

/// Internal mutable state shared between event loop and input handlers.
struct app_state {
    /// Core editor engine instance.
    ed: editor,
    /// Active interactive command or search prompt.
    prompt: Option<prompt_state>,
    /// Active interactive modal picker.
    picker: Option<picker_state>,
    /// Terminal facilitator managing double-buffered diffing.
    termit: Termit<'static, ()>,
    /// Thread handle for stopping the Yokio runtime.
    handle: yokio::Handle,
    /// RAII token keeping the Yokio loop alive while waiting for input.
    keep_alive: Option<yokio::KeepAlive>,
    /// Active asynchronous input watcher.
    input_watcher: Option<InputWatcher>,
}

impl app_state {
    /// Paints the editor frame and flushes updates to the terminal.
    fn render(&mut self) -> io::Result<()> {
        self.ed.poll_lsp();
        let c_info = render_frame(
            self.termit.screen_mut(),
            &mut self.ed,
            self.prompt.as_ref(),
            self.picker.as_ref(),
        );
        self.termit.print(None)?;
        sync_cursor(c_info)
    }
}

/// Synchronizes the hardware terminal cursor position, shape, and visibility.
fn sync_cursor(info: cursor_render_info) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if info.visible {
        let shape_code = match info.shape {
            cursor_shape::block => 2,
            cursor_shape::underline => 4,
            cursor_shape::bar => 6,
        };
        write!(
            out,
            "\x1b[{};{}H\x1b[{} q\x1b[?25h",
            info.row + 1,
            info.col + 1,
            shape_code
        )?;
    } else {
        write!(out, "\x1b[?25l")?;
    }
    out.flush()
}

/// Runs the Ned editor application to completion.
///
/// # Errors
///
/// Returns an `io::Error` if terminal initialization, Yokio setup, or file I/O fails.
pub fn run<P: AsRef<std::path::Path>>(paths: &[P]) -> io::Result<()> {
    let mut terminal = Terminal::try_system_default()?;
    let config = ned_config::load();
    let mouse_enabled = config.editor.mouse;

    terminal.send(EnterRawMode)?;
    terminal.send(UseAlternateScreen(true))?;
    terminal.send(WrapLines(false))?;
    if mouse_enabled {
        let _ = terminal.send(CaptureMouse(true));
    }

    let size = terminal.control.get_size().unwrap_or_default();
    let width = size.col;
    let height = size.row;

    let termit: Termit<'static, ()> = Termit::from(terminal).true_color(true);

    let mut ed = editor::new(config, width, height);

    for path in paths {
        let p = path.as_ref();
        if let Err(e) = ed.open_file(p) {
            ed.status_message = Some(format!("error opening {}: {e}", p.display()));
        }
    }

    let rt = yokio::runtime::Runtime::new().map_err(|e| io::Error::other(e.to_string()))?;
    let _guard = rt.enter();
    let handle = rt.handle();
    let keep_alive = handle.keep_alive();

    let state = Rc::new(RefCell::new(app_state {
        ed,
        prompt: None,
        picker: None,
        termit,
        handle: handle.clone(),
        keep_alive: Some(keep_alive),
        input_watcher: None,
    }));

    // Initial paint and screen flush
    state.borrow_mut().render()?;

    // Set up Yokio terminal input watcher
    let state_clone = Rc::clone(&state);
    let watcher = {
        let app = state.borrow();
        app.termit.watch_input(move |event_res| {
            let Ok(event) = event_res else {
                return;
            };

            let mut app = state_clone.borrow_mut();
            match event {
                Event::Resize(new_size) => {
                    app.termit.screen_mut().resize(new_size);
                }
                Event::Key(key_ev) => {
                    handle_key_event(&mut app, &key_ev);
                }
                Event::Mouse(mouse_ev) => {
                    handle_mouse_event(&mut app, &mouse_ev);
                }
                Event::InputClosed => {
                    app.keep_alive.take();
                    app.handle.stop();
                    return;
                }
                _ => {}
            }

            if app.ed.should_quit {
                app.keep_alive.take();
                app.handle.stop();
                return;
            }

            let _ = app.render();
        })?
    };

    state.borrow_mut().input_watcher = Some(watcher);

    // Poll LSP diagnostics asynchronously so UI updates immediately when servers respond
    let lsp_state = Rc::clone(&state);
    let _lsp_ticker = yokio::every(std::time::Duration::from_millis(100), move || {
        if let Ok(mut app) = lsp_state.try_borrow_mut() {
            if app.ed.should_quit {
                return;
            }
            if app.ed.poll_lsp() {
                let _ = app.render();
            }
        }
    });

    // Drive event loop until quit
    rt.run().map_err(|e| io::Error::other(e.to_string()))?;

    Ok(())
}

/// Processes a single keyboard event and updates editor or prompt state.
fn handle_key_event(app: &mut app_state, key_ev: &KeyEvent) {
    // 1. Active modal picker intercept
    if let Some(mut p) = app.picker.take() {
        match p.handle_key(key_ev) {
            picker_action::none => {
                app.picker = Some(p);
            }
            picker_action::cancel => {
                app.picker = None;
            }
            picker_action::select(item) => {
                app.picker = None;
                if p.title == "Diagnostics" {
                    if let Some((loc, msg)) = item.split_once("] ") {
                        if let Some(coords) = loc.split_whitespace().next() {
                            if let Some((l_str, c_str)) = coords.split_once(':') {
                                if let (Ok(l), Ok(c)) = (l_str.parse::<usize>(), c_str.parse::<usize>()) {
                                    let doc = app.ed.active_doc_mut();
                                    let line_idx = l.saturating_sub(1);
                                    let col_idx = c.saturating_sub(1);
                                    let line_byte = doc.text().line_to_byte(line_idx).unwrap_or(0);
                                    let line_char = doc.text().byte_to_char(line_byte);
                                    let target_char = (line_char + col_idx).min(doc.text().char_len());
                                    doc.set_selection(ned_text::selection::point(target_char));
                                    app.ed.ensure_active_cursor_visible();
                                }
                            }
                        }
                        app.ed.status_message = Some(format!("[LSP] {msg}"));
                    }
                } else if let Err(e) = app.ed.open_file(&item) {
                    app.ed.status_message = Some(format!("error opening {item}: {e}"));
                }
            }
        }
        return;
    }

    // 2. Active command/search prompt intercept
    if let Some(mut p) = app.prompt.take() {
        match p.handle_key(key_ev) {
            prompt_action::none => {
                app.prompt = Some(p);
            }
            prompt_action::cancel => {
                app.prompt = None;
            }
            prompt_action::submit(cmd) => {
                app.prompt = None;
                let trimmed = cmd.trim();
                if trimmed == "o" || trimmed == "open" {
                    let cwd = std::env::current_dir()
                        .unwrap_or_else(|_| std::path::PathBuf::from("."));
                    app.picker = Some(picker_state::new_file_picker(&cwd));
                } else {
                    app.ed.execute_command(&cmd);
                }
            }
        }
        return;
    }

    // 3. Normal, Insert, or Select mode handling
    if app.ed.mode != mode::insert
        && key_ev.keycode == KeyCode::Char(':')
        && !key_ev.modifiers.contains(&KeyModifier::Control)
        && !key_ev.modifiers.contains(&KeyModifier::Alt)
    {
        app.prompt = Some(prompt_state::new(":"));
        return;
    }

    if let Some(modal_ev) = translate_key_event(key_ev) {
        app.ed.handle_key(modal_ev);

        // Check if an action triggered a special UI overlay
        if let Some(msg) = app.ed.status_message.take() {
            if msg == ":" {
                app.prompt = Some(prompt_state::new(":"));
            } else if msg == "picker:file" {
                let cwd =
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                app.picker = Some(picker_state::new_file_picker(&cwd));
            } else if msg == "picker:buffer" {
                let mut buffers = Vec::new();
                for doc in app.ed.documents.values() {
                    let name = doc
                        .path()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "[scratch]".to_string());
                    buffers.push(name);
                }
                app.picker = Some(picker_state::new_buffer_picker(buffers));
            } else if msg == "picker:diagnostics" {
                let diags = app.ed.active_doc().diagnostics();
                if diags.is_empty() {
                    app.ed.status_message = Some("No diagnostics for this document".to_string());
                } else {
                    let mut items = Vec::with_capacity(diags.len());
                    for d in diags {
                        let sev = match d.severity {
                            Some(ned_lsp::diagnostic_severity::error) => "ERROR",
                            Some(ned_lsp::diagnostic_severity::warning) => "WARN",
                            Some(ned_lsp::diagnostic_severity::information) => "INFO",
                            Some(ned_lsp::diagnostic_severity::hint) | None => "HINT",
                        };
                        let first_line = d.message.lines().next().unwrap_or("");
                        items.push(format!(
                            "{}:{} [{sev}] {first_line}",
                            d.range.start.line + 1,
                            d.range.start.character + 1
                        ));
                    }
                    app.picker = Some(picker_state::new_diagnostics_picker(items));
                }
            } else {
                app.ed.status_message = Some(msg);
            }
        }
    }
}

/// Processes a single mouse event and navigates the cursor or scrolls viewport.
fn handle_mouse_event(app: &mut app_state, mouse_ev: &MouseEvent) {
    if !app.ed.config.editor.mouse {
        return;
    }
    if app.prompt.is_some() || app.picker.is_some() {
        return;
    }

    let size = app.termit.screen_mut().size();
    let width = size.col;
    let height = size.row;
    if width < 5 || height < 2 {
        return;
    }

    let editor_height = height.saturating_sub(2);
    let doc_id = app.ed.active_doc_id;
    let total_lines = app
        .ed
        .documents
        .get(&doc_id)
        .map_or(1, |d| d.text().line_count().max(1));
    let gutter_width = crate::ui::calculate_gutter_width(total_lines);

    match mouse_ev.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            app.ed
                .handle_mouse_click(mouse_ev.column, mouse_ev.row, gutter_width, editor_height, false);
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            app.ed
                .handle_mouse_click(mouse_ev.column, mouse_ev.row, gutter_width, editor_height, true);
        }
        MouseEventKind::ScrollUp => {
            let lines = app.ed.config.editor.scroll_lines as isize;
            app.ed.scroll_view_by(-lines);
        }
        MouseEventKind::ScrollDown => {
            let lines = app.ed.config.editor.scroll_lines as isize;
            app.ed.scroll_view_by(lines);
        }
        _ => {}
    }
}
