//! Command documentation registry and completion calculation matching Helix typable commands.

use super::state::prompt_state;

/// Documentation and signature metadata for a typable command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct command_doc {
    /// Command name or alias.
    pub name: &'static str,
    /// Detailed description of command behavior and options.
    pub doc: &'static str,
}

/// Complete registry of Ned typable commands and aliases with full documentation, sorted alphabetically.
pub const COMMANDS: &[command_doc] = &[
    command_doc {
        name: "!",
        doc: "Alias for :run-shell-command. Run a shell command.",
    },
    command_doc {
        name: "bc",
        doc: "Alias for :buffer-close. Close the current buffer.",
    },
    command_doc {
        name: "bc!",
        doc: "Alias for :buffer-close!. Close the current buffer forcefully, ignoring unsaved changes.",
    },
    command_doc {
        name: "bca",
        doc: "Alias for :buffer-close-all. Close all buffers without quitting.",
    },
    command_doc {
        name: "bca!",
        doc: "Alias for :buffer-close-all!. Force close all buffers ignoring unsaved changes without quitting.",
    },
    command_doc {
        name: "bclose",
        doc: "Alias for :buffer-close. Close the current buffer.",
    },
    command_doc {
        name: "bclose!",
        doc: "Alias for :buffer-close!. Close the current buffer forcefully, ignoring unsaved changes.",
    },
    command_doc {
        name: "bcloseall",
        doc: "Alias for :buffer-close-all. Close all buffers without quitting.",
    },
    command_doc {
        name: "bcloseall!",
        doc: "Alias for :buffer-close-all!. Force close all buffers ignoring unsaved changes without quitting.",
    },
    command_doc {
        name: "bcloseother",
        doc: "Alias for :buffer-close-others. Close all buffers but the currently focused one.",
    },
    command_doc {
        name: "bcloseother!",
        doc: "Alias for :buffer-close-others!. Force close all buffers but the currently focused one.",
    },
    command_doc {
        name: "bco",
        doc: "Alias for :buffer-close-others. Close all buffers but the currently focused one.",
    },
    command_doc {
        name: "bco!",
        doc: "Alias for :buffer-close-others!. Force close all buffers but the currently focused one.",
    },
    command_doc {
        name: "bn",
        doc: "Alias for :buffer-next. Goto next buffer.",
    },
    command_doc {
        name: "bnext",
        doc: "Alias for :buffer-next. Goto next buffer.",
    },
    command_doc {
        name: "bp",
        doc: "Alias for :buffer-previous. Goto previous buffer.",
    },
    command_doc {
        name: "bprev",
        doc: "Alias for :buffer-previous. Goto previous buffer.",
    },
    command_doc {
        name: "buffer-close",
        doc: "Close the current buffer.\nAliases: bc, bclose",
    },
    command_doc {
        name: "buffer-close!",
        doc: "Close the current buffer forcefully, ignoring unsaved changes.\nAliases: bc!, bclose!",
    },
    command_doc {
        name: "buffer-close-all",
        doc: "Close all buffers without quitting.\nAliases: bca, bcloseall",
    },
    command_doc {
        name: "buffer-close-all!",
        doc: "Force close all buffers ignoring unsaved changes without quitting.\nAliases: bca!, bcloseall!",
    },
    command_doc {
        name: "buffer-close-others",
        doc: "Close all buffers but the currently focused one.\nAliases: bco, bcloseother",
    },
    command_doc {
        name: "buffer-close-others!",
        doc: "Force close all buffers but the currently focused one.\nAliases: bco!, bcloseother!",
    },
    command_doc {
        name: "buffer-next",
        doc: "Goto next buffer.\nAliases: bn, bnext",
    },
    command_doc {
        name: "buffer-previous",
        doc: "Goto previous buffer.\nAliases: bp, bprev",
    },
    command_doc {
        name: "cd",
        doc: "Alias for :change-current-directory. Change the current working directory.",
    },
    command_doc {
        name: "change-current-directory",
        doc: "Change the current working directory.\nAliases: cd",
    },
    command_doc {
        name: "char",
        doc: "Alias for :character-info. Get info about the character under the primary cursor.",
    },
    command_doc {
        name: "character-info",
        doc: "Get info about the character under the primary cursor.\nAliases: char",
    },
    command_doc {
        name: "clear-register",
        doc: "Clear given register. If no argument is provided, clear all registers.",
    },
    command_doc {
        name: "clipboard-paste-after",
        doc: "Paste system clipboard after selections.",
    },
    command_doc {
        name: "clipboard-paste-before",
        doc: "Paste system clipboard before selections.",
    },
    command_doc {
        name: "clipboard-paste-replace",
        doc: "Replace selections with content of system clipboard.",
    },
    command_doc {
        name: "clipboard-yank",
        doc: "Yank main selection into system clipboard.",
    },
    command_doc {
        name: "clipboard-yank-join",
        doc: "Yank joined selections into system clipboard. A separator can be provided as first argument. Default value is newline.",
    },
    command_doc {
        name: "config-open",
        doc: "Open the user config.toml file.",
    },
    command_doc {
        name: "config-open-workspace",
        doc: "Open the workspace config.toml file.",
    },
    command_doc {
        name: "config-reload",
        doc: "Refresh user config.",
    },
    command_doc {
        name: "cq",
        doc: "Alias for :cquit. Quit with exit code (default 1). Accepts an optional integer exit code (:cq 2).",
    },
    command_doc {
        name: "cq!",
        doc: "Alias for :cquit!. Force quit with exit code (default 1) ignoring unsaved changes.",
    },
    command_doc {
        name: "cquit",
        doc: "Quit with exit code (default 1). Accepts an optional integer exit code (:cq 2).\nAliases: cq",
    },
    command_doc {
        name: "cquit!",
        doc: "Force quit with exit code (default 1) ignoring unsaved changes.\nAliases: cq!",
    },
    command_doc {
        name: "e",
        doc: "Alias for :open. Open a file from disk into the current view.",
    },
    command_doc {
        name: "ear",
        doc: "Alias for :earlier. Jump back to an earlier point in edit history.",
    },
    command_doc {
        name: "earlier",
        doc: "Jump back to an earlier point in edit history. Accepts a number of steps.\nAliases: ear",
    },
    command_doc {
        name: "echo",
        doc: "Prints the given arguments to the statusline.",
    },
    command_doc {
        name: "edit",
        doc: "Alias for :open. Open a file from disk into the current view.",
    },
    command_doc {
        name: "fmt",
        doc: "Alias for :format. Format the file using an external formatter.",
    },
    command_doc {
        name: "format",
        doc: "Format the file using an external formatter.\nAliases: fmt",
    },
    command_doc {
        name: "g",
        doc: "Alias for :goto. Goto line number.",
    },
    command_doc {
        name: "get",
        doc: "Alias for :get-option. Get the current value of a config option.",
    },
    command_doc {
        name: "get-option",
        doc: "Get the current value of a config option.\nAliases: get",
    },
    command_doc {
        name: "goto",
        doc: "Goto line number.\nAliases: g",
    },
    command_doc {
        name: "indent-style",
        doc: "Set the indentation style for editing ('t' for tabs or 1-16 for number of spaces).",
    },
    command_doc {
        name: "lat",
        doc: "Alias for :later. Jump to a later point in edit history.",
    },
    command_doc {
        name: "later",
        doc: "Jump to a later point in edit history. Accepts a number of steps.\nAliases: lat",
    },
    command_doc {
        name: "line-ending",
        doc: "Set the document's default line ending. Options: crlf, lf.",
    },
    command_doc {
        name: "log-open",
        doc: "Open the log file.",
    },
    command_doc {
        name: "n",
        doc: "Alias for :new. Create a new scratch buffer.",
    },
    command_doc {
        name: "new",
        doc: "Create a new scratch buffer.\nAliases: n",
    },
    command_doc {
        name: "noop",
        doc: "Does nothing.",
    },
    command_doc {
        name: "o",
        doc: "Alias for :open. Open a file from disk into the current view.",
    },
    command_doc {
        name: "open",
        doc: "Open a file from disk into the current view.\nAliases: e, edit, o",
    },
    command_doc {
        name: "primary-clipboard-paste-after",
        doc: "Paste primary clipboard after selections.",
    },
    command_doc {
        name: "primary-clipboard-paste-before",
        doc: "Paste primary clipboard before selections.",
    },
    command_doc {
        name: "primary-clipboard-paste-replace",
        doc: "Replace selections with content of primary clipboard.",
    },
    command_doc {
        name: "primary-clipboard-yank",
        doc: "Yank main selection into primary clipboard.",
    },
    command_doc {
        name: "primary-clipboard-yank-join",
        doc: "Yank joined selections into primary clipboard. A separator can be provided as first argument. Default value is newline.",
    },
    command_doc {
        name: "pwd",
        doc: "Alias for :show-directory. Show the current working directory.",
    },
    command_doc {
        name: "q",
        doc: "Alias for :quit. Close the current view.",
    },
    command_doc {
        name: "q!",
        doc: "Alias for :quit!. Force close the current view, ignoring unsaved changes.",
    },
    command_doc {
        name: "qa",
        doc: "Alias for :quit-all. Close all views.",
    },
    command_doc {
        name: "qa!",
        doc: "Alias for :quit-all!. Force close all views ignoring unsaved changes.",
    },
    command_doc {
        name: "quit",
        doc: "Close the current view.\nAliases: q",
    },
    command_doc {
        name: "quit!",
        doc: "Force close the current view, ignoring unsaved changes.\nAliases: q!",
    },
    command_doc {
        name: "quit-all",
        doc: "Close all views.\nAliases: qa",
    },
    command_doc {
        name: "quit-all!",
        doc: "Force close all views ignoring unsaved changes.\nAliases: qa!",
    },
    command_doc {
        name: "redraw",
        doc: "Clear and re-render the whole UI.",
    },
    command_doc {
        name: "reload",
        doc: "Discard changes and reload from the source file.\nAliases: rl",
    },
    command_doc {
        name: "reload-all",
        doc: "Discard changes and reload all documents from the source files.\nAliases: rla",
    },
    command_doc {
        name: "rl",
        doc: "Alias for :reload. Discard changes and reload from the source file.",
    },
    command_doc {
        name: "rla",
        doc: "Alias for :reload-all. Discard changes and reload all documents from the source files.",
    },
    command_doc {
        name: "run-shell-command",
        doc: "Run a shell command.\nAliases: !, sh",
    },
    command_doc {
        name: "set",
        doc: "Alias for :set-option. Set a config option at runtime.",
    },
    command_doc {
        name: "set-option",
        doc: "Set a config option at runtime.\nAliases: set",
    },
    command_doc {
        name: "sh",
        doc: "Alias for :run-shell-command. Run a shell command.",
    },
    command_doc {
        name: "show-clipboard-provider",
        doc: "Show clipboard provider name in status bar.",
    },
    command_doc {
        name: "show-directory",
        doc: "Show the current working directory.\nAliases: pwd",
    },
    command_doc {
        name: "theme",
        doc: "Change the editor theme (show current theme if no name specified).",
    },
    command_doc {
        name: "toggle",
        doc: "Alias for :toggle-option. Toggle a config option at runtime.",
    },
    command_doc {
        name: "toggle-option",
        doc: "Toggle a config option at runtime.\nAliases: toggle",
    },
    command_doc {
        name: "u",
        doc: "Alias for :update. Write changes only if the file has been modified.",
    },
    command_doc {
        name: "update",
        doc: "Write changes only if the file has been modified.\nAliases: u",
    },
    command_doc {
        name: "w",
        doc: "Alias for :write. Write changes to disk.",
    },
    command_doc {
        name: "w!",
        doc: "Alias for :write!. Force write changes to disk creating necessary subdirectories.",
    },
    command_doc {
        name: "wa",
        doc: "Alias for :write-all. Write changes from all buffers to disk.",
    },
    command_doc {
        name: "wa!",
        doc: "Alias for :write-all!. Forcefully write changes from all buffers to disk creating necessary subdirectories.",
    },
    command_doc {
        name: "wbc",
        doc: "Alias for :write-buffer-close. Write changes to disk and close the buffer.",
    },
    command_doc {
        name: "wbc!",
        doc: "Alias for :write-buffer-close!. Force write changes to disk creating necessary subdirectories and close the buffer.",
    },
    command_doc {
        name: "write",
        doc: "Write changes to disk. Accepts an optional path (:write some/path.txt).\nAliases: w",
    },
    command_doc {
        name: "write!",
        doc: "Force write changes to disk creating necessary subdirectories. Accepts an optional path (:write! some/path.txt).\nAliases: w!",
    },
    command_doc {
        name: "write-all",
        doc: "Write changes from all buffers to disk.\nAliases: wa",
    },
    command_doc {
        name: "write-all!",
        doc: "Forcefully write changes from all buffers to disk creating necessary subdirectories.\nAliases: wa!",
    },
    command_doc {
        name: "write-buffer-close",
        doc: "Write changes to disk and close the buffer. Accepts an optional path (:write-buffer-close some/path.txt).\nAliases: wbc",
    },
    command_doc {
        name: "write-buffer-close!",
        doc: "Force write changes to disk creating necessary subdirectories and close the buffer.\nAliases: wbc!",
    },
    command_doc {
        name: "write-quit",
        doc: "Write changes to disk and close the current view. Accepts an optional path (:wq some/path.txt).\nAliases: wq, x",
    },
    command_doc {
        name: "write-quit!",
        doc: "Write changes to disk and close the current view forcefully. Accepts an optional path (:wq! some/path.txt).\nAliases: wq!, x!",
    },
    command_doc {
        name: "write-quit-all",
        doc: "Write changes from all buffers to disk and close all views.\nAliases: wqa, xa",
    },
    command_doc {
        name: "write-quit-all!",
        doc: "Write changes from all buffers to disk and close all views forcefully.\nAliases: wqa!, xa!",
    },
    command_doc {
        name: "wq",
        doc: "Alias for :write-quit. Write changes to disk and close the current view.",
    },
    command_doc {
        name: "wq!",
        doc: "Alias for :write-quit!. Write changes to disk and close the current view forcefully.",
    },
    command_doc {
        name: "wqa",
        doc: "Alias for :write-quit-all. Write changes from all buffers to disk and close all views.",
    },
    command_doc {
        name: "wqa!",
        doc: "Alias for :write-quit-all!. Write changes from all buffers to disk and close all views forcefully.",
    },
    command_doc {
        name: "x",
        doc: "Alias for :write-quit. Write changes to disk and close the current view.",
    },
    command_doc {
        name: "x!",
        doc: "Alias for :write-quit!. Write changes to disk and close the current view forcefully.",
    },
    command_doc {
        name: "xa",
        doc: "Alias for :write-quit-all. Write changes from all buffers to disk and close all views.",
    },
    command_doc {
        name: "xa!",
        doc: "Alias for :write-quit-all!. Write changes from all buffers to disk and close all views forcefully.",
    },
];

/// Direction to cycle through prompt completions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum completion_direction {
    /// Move to next item.
    forward,
    /// Move to previous item.
    backward,
}

/// Recalculates matching completions based on the currently typed command prefix.
pub fn recalculate_completion(prompt: &mut prompt_state) {
    prompt.selection = None;
    if prompt.prefix != ":" {
        prompt.completion = Vec::new();
        return;
    }

    let trimmed = prompt.input.trim_start();
    let cmd_token = trimmed.split_whitespace().next().unwrap_or("");

    let mut matches: Vec<&'static command_doc> = COMMANDS
        .iter()
        .filter(|c| c.name.starts_with(cmd_token))
        .collect();

    // Sort alphabetically matching Helix TYPABLE_COMMAND_LIST
    matches.sort_by_key(|c| c.name);
    prompt.completion = matches;
}

/// Cycles the active completion candidate selection forward or backward.
pub fn change_completion_selection(prompt: &mut prompt_state, direction: completion_direction) {
    if prompt.completion.is_empty() {
        return;
    }

    let len = prompt.completion.len();
    let next_idx = match direction {
        completion_direction::forward => match prompt.selection {
            Some(i) => (i + 1) % len,
            None => 0,
        },
        completion_direction::backward => match prompt.selection {
            Some(i) => {
                if i == 0 {
                    len - 1
                } else {
                    i - 1
                }
            }
            None => len - 1,
        },
    };

    prompt.selection = Some(next_idx);
    let selected_item = prompt.completion[next_idx];

    // Replace command token with selected item
    let trimmed = prompt.input.trim_start();
    let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
    if parts.len() > 1 {
        prompt.input = format!("{} {}", selected_item.name, parts[1]);
    } else {
        prompt.input = selected_item.name.to_string();
    }
    prompt.cursor = prompt.input.chars().count();
}

/// Clears the active completion selection.
pub fn exit_selection(prompt: &mut prompt_state) {
    prompt.selection = None;
}

/// Returns the currently active documentation (from selected completion candidate or typed command).
#[must_use]
pub fn active_doc(prompt: &prompt_state) -> Option<(&'static str, &'static str)> {
    if let Some(idx) = prompt.selection {
        if let Some(cmd) = prompt.completion.get(idx) {
            return Some((cmd.name, cmd.doc));
        }
    }
    let token = prompt
        .input
        .split_whitespace()
        .next()
        .unwrap_or("");
    if token.is_empty() {
        return None;
    }
    COMMANDS
        .iter()
        .find(|c| c.name == token)
        .map(|c| (c.name, c.doc))
}
