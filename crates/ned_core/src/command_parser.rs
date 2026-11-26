//! Command parser for Helix-style `:` ex-commands.

/// Executable command parsed from editor command prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum editor_command {
    /// Save document (`:w [filename]`).
    write(Option<String>),
    /// Quit editor (`:q`).
    quit,
    /// Force quit without saving (`:q!`).
    force_quit,
    /// Save document and quit (`:wq`, `:x`).
    write_and_quit(Option<String>),
    /// Open file (`:e <path>`, `:open <path>`).
    open(String),
    /// Set editor configuration option (`:set <key> <value>`).
    set_option(String, String),
}

/// Parses a command line string into an [`editor_command`].
///
/// # Errors
///
/// Returns an error message if the command syntax or name is unrecognized.
pub fn parse_command(input: &str) -> Result<editor_command, String> {
    let trimmed = input.trim();
    let command_str = trimmed.strip_prefix(':').unwrap_or(trimmed).trim();

    if command_str.is_empty() {
        return Err("empty command".to_string());
    }

    let mut parts = command_str.split_whitespace();
    let cmd = parts.next().unwrap_or("");
    let arg1 = parts.next();
    let arg2 = parts.next();

    match cmd {
        "w" | "write" => Ok(editor_command::write(arg1.map(ToString::to_string))),
        "q" | "quit" => Ok(editor_command::quit),
        "q!" | "quit!" => Ok(editor_command::force_quit),
        "wq" | "x" => Ok(editor_command::write_and_quit(arg1.map(ToString::to_string))),
        "e" | "open" => {
            if let Some(path) = arg1 {
                Ok(editor_command::open(path.to_string()))
            } else {
                Err("filename required for open command".to_string())
            }
        }
        "set" => {
            if let (Some(k), Some(v)) = (arg1, arg2) {
                Ok(editor_command::set_option(k.to_string(), v.to_string()))
            } else {
                Err("usage: :set <option> <value>".to_string())
            }
        }
        unknown => Err(format!("unknown command: :{unknown}")),
    }
}
