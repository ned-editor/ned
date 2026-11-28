//! Command parser for Helix-style `:` ex-commands.

/// Executable command parsed from editor command prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum editor_command {
    /// Save document (`:w [path]`).
    write {
        /// Optional destination path.
        path: Option<String>,
        /// Force write flag.
        force: bool,
    },
    /// Save document and quit (`:wq`, `:x`).
    write_quit {
        /// Optional destination path.
        path: Option<String>,
        /// Force write flag.
        force: bool,
    },
    /// Save all documents (`:wa`).
    write_all {
        /// Force write flag.
        force: bool,
    },
    /// Save all documents and quit (`:wqa`, `:xa`).
    write_quit_all {
        /// Force write flag.
        force: bool,
    },
    /// Quit editor (`:q`).
    quit {
        /// Force quit flag.
        force: bool,
    },
    /// Quit all buffers (`:qa`).
    quit_all {
        /// Force quit flag.
        force: bool,
    },
    /// Quit with exit code (`:cq`).
    cquit {
        /// Optional exit code.
        code: Option<i32>,
        /// Force quit flag.
        force: bool,
    },
    /// Open file into buffer (`:open <path>`, `:e <path>`).
    open(String),
    /// Create new scratch buffer (`:new`, `:n`).
    new_scratch,
    /// Close current buffer (`:bc`, `:bclose`).
    buffer_close {
        /// Force close flag.
        force: bool,
    },
    /// Close all buffers except current (`:bco`).
    buffer_close_others {
        /// Force close flag.
        force: bool,
    },
    /// Close all buffers (`:bca`).
    buffer_close_all {
        /// Force close flag.
        force: bool,
    },
    /// Switch to next buffer (`:bn`).
    buffer_next,
    /// Switch to previous buffer (`:bp`).
    buffer_previous,
    /// Reload buffer from disk (`:rl`).
    reload {
        /// Reload all open buffers.
        all: bool,
    },
    /// Write only if modified (`:u`).
    update,
    /// Run quick external shell command (`:sh <cmd>`, `:! <cmd>`, `:run-shell-command <cmd>`).
    run_shell(String),
    /// Switch theme (`:theme [name]`).
    theme(Option<String>),
    /// Format current file (`:format`, `:fmt`).
    format,
    /// Set config option (`:set <key> <value>`).
    set_option(String, String),
    /// Get config option (`:get <key>`).
    get_option(String),
    /// Toggle config option (`:toggle <key>`).
    toggle_option(String),
    /// Jump to line (`:goto <line>`).
    goto_line(usize),
    /// Undo steps (`:earlier [count]`).
    earlier(usize),
    /// Redo steps (`:later [count]`).
    later(usize),
    /// Change working directory (`:cd [path]`).
    change_directory(Option<String>),
    /// Show working directory (`:pwd`).
    show_directory,
    /// Show character info (`:char`).
    character_info,
    /// Clear register (`:clear-register [reg]`).
    clear_register(Option<char>),
    /// Echo status message (`:echo <msg>`).
    echo(String),
    /// No operation (`:noop`).
    noop,
}

/// Parses a command line string into an [`editor_command`].
///
/// # Errors
///
/// Returns an error if the command is empty or unrecognized.
pub fn parse_command(input: &str) -> Result<editor_command, String> {
    let trimmed = input.trim();
    let command_str = trimmed.strip_prefix(':').unwrap_or(trimmed).trim();

    if command_str.is_empty() {
        return Err("empty command".to_string());
    }

    let (cmd, rest) = match command_str.find(char::is_whitespace) {
        Some(idx) => (&command_str[..idx], command_str[idx..].trim_start()),
        None => (command_str, ""),
    };

    // Quick shell execution takes the remainder as-is
    match cmd {
        "sh" | "!" | "run-shell-command" => {
            if rest.is_empty() {
                return Err("usage: :sh <command>".to_string());
            }
            return Ok(editor_command::run_shell(rest.to_string()));
        }
        _ => {}
    }

    let args = parse_args(rest);

    match cmd {
        "w" | "write" => Ok(editor_command::write {
            path: args.first().cloned(),
            force: false,
        }),
        "w!" | "write!" => Ok(editor_command::write {
            path: args.first().cloned(),
            force: true,
        }),
        "wq" | "write-quit" | "x" => Ok(editor_command::write_quit {
            path: args.first().cloned(),
            force: false,
        }),
        "wq!" | "write-quit!" | "x!" => Ok(editor_command::write_quit {
            path: args.first().cloned(),
            force: true,
        }),
        "wa" | "write-all" => Ok(editor_command::write_all { force: false }),
        "wa!" | "write-all!" => Ok(editor_command::write_all { force: true }),
        "wqa" | "write-quit-all" | "xa" => Ok(editor_command::write_quit_all { force: false }),
        "wqa!" | "write-quit-all!" | "xa!" => Ok(editor_command::write_quit_all { force: true }),
        "q" | "quit" => Ok(editor_command::quit { force: false }),
        "q!" | "quit!" => Ok(editor_command::quit { force: true }),
        "qa" | "quit-all" => Ok(editor_command::quit_all { force: false }),
        "qa!" | "quit-all!" => Ok(editor_command::quit_all { force: true }),
        "cq" | "cquit" => {
            let code = args.first().and_then(|s| s.parse().ok());
            Ok(editor_command::cquit { code, force: false })
        }
        "cq!" | "cquit!" => {
            let code = args.first().and_then(|s| s.parse().ok());
            Ok(editor_command::cquit { code, force: true })
        }
        "e" | "edit" | "o" | "open" => {
            if let Some(path) = args.first() {
                Ok(editor_command::open(path.clone()))
            } else {
                Err("filename required for open command".to_string())
            }
        }
        "n" | "new" => Ok(editor_command::new_scratch),
        "bc" | "bclose" | "buffer-close" => Ok(editor_command::buffer_close { force: false }),
        "bc!" | "bclose!" | "buffer-close!" => Ok(editor_command::buffer_close { force: true }),
        "bco" | "bcloseother" | "buffer-close-others" => {
            Ok(editor_command::buffer_close_others { force: false })
        }
        "bco!" | "bcloseother!" | "buffer-close-others!" => {
            Ok(editor_command::buffer_close_others { force: true })
        }
        "bca" | "bcloseall" | "buffer-close-all" => {
            Ok(editor_command::buffer_close_all { force: false })
        }
        "bca!" | "bcloseall!" | "buffer-close-all!" => {
            Ok(editor_command::buffer_close_all { force: true })
        }
        "bn" | "bnext" | "buffer-next" => Ok(editor_command::buffer_next),
        "bp" | "bprev" | "buffer-previous" => Ok(editor_command::buffer_previous),
        "rl" | "reload" => Ok(editor_command::reload { all: false }),
        "rla" | "reload-all" => Ok(editor_command::reload { all: true }),
        "u" | "update" => Ok(editor_command::update),
        "theme" => Ok(editor_command::theme(args.first().cloned())),
        "format" | "fmt" => Ok(editor_command::format),
        "set" | "set-option" => {
            if args.len() >= 2 {
                Ok(editor_command::set_option(args[0].clone(), args[1].clone()))
            } else {
                Err("usage: :set <option> <value>".to_string())
            }
        }
        "get" | "get-option" => {
            if let Some(k) = args.first() {
                Ok(editor_command::get_option(k.clone()))
            } else {
                Err("usage: :get <option>".to_string())
            }
        }
        "toggle" | "toggle-option" => {
            if let Some(k) = args.first() {
                Ok(editor_command::toggle_option(k.clone()))
            } else {
                Err("usage: :toggle <option>".to_string())
            }
        }
        "g" | "goto" => {
            if let Some(line) = args.first().and_then(|s| s.parse().ok()) {
                Ok(editor_command::goto_line(line))
            } else {
                Err("usage: :goto <line>".to_string())
            }
        }
        "ear" | "earlier" => {
            let count = args.first().and_then(|s| s.parse().ok()).unwrap_or(1);
            Ok(editor_command::earlier(count))
        }
        "lat" | "later" => {
            let count = args.first().and_then(|s| s.parse().ok()).unwrap_or(1);
            Ok(editor_command::later(count))
        }
        "cd" | "change-current-directory" => {
            Ok(editor_command::change_directory(args.first().cloned()))
        }
        "pwd" | "show-directory" => Ok(editor_command::show_directory),
        "char" | "character-info" => Ok(editor_command::character_info),
        "clear-register" => {
            let ch = args.first().and_then(|s| s.chars().next());
            Ok(editor_command::clear_register(ch))
        }
        "echo" => Ok(editor_command::echo(args.join(" "))),
        "noop" => Ok(editor_command::noop),
        unknown => Err(format!("unknown command: :{unknown}")),
    }
}

fn parse_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote = None;

    for ch in input.chars() {
        if let Some(q) = in_quote {
            if ch == q {
                in_quote = None;
            } else {
                current.push(ch);
            }
        } else if ch == '"' || ch == '\'' {
            in_quote = Some(ch);
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                args.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}
