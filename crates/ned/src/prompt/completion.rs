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

/// Complete registry of Helix typable commands and aliases with full documentation.
pub const COMMANDS: &[command_doc] = &[
    command_doc {
        name: "!",
        doc: "Alias for :run-shell-command. Run a shell command",
    },
    command_doc {
        name: "append-output",
        doc: "Run shell command, appending output after each selection.",
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
        name: "bnext",
        doc: "Alias for :buffer-next. Goto next buffer.",
    },
    command_doc {
        name: "bn",
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
        doc: "Change the current working directory.",
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
        doc: "Force quit with exit code (default 1) ignoring unsaved changes. Accepts an optional integer exit code (:cq! 2).\nAliases: cq!",
    },
    command_doc {
        name: "diffg",
        doc: "Alias for :reset-diff-change. Reset the diff change at the cursor position.",
    },
    command_doc {
        name: "diffget",
        doc: "Alias for :reset-diff-change. Reset the diff change at the cursor position.",
    },
    command_doc {
        name: "e",
        doc: "Alias for :open. Open a file from disk into the current view.",
    },
    command_doc {
        name: "ear",
        doc: "Alias for :earlier. Jump back to an earlier point in edit history. Accepts a number of steps or a time span.",
    },
    command_doc {
        name: "earlier",
        doc: "Jump back to an earlier point in edit history. Accepts a number of steps or a time span.\nAliases: ear",
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
        name: "exit",
        doc: "Write changes to disk if the buffer is modified and then quit. Accepts an optional path (:exit some/path.txt).\nAliases: x, xit",
    },
    command_doc {
        name: "exit!",
        doc: "Force write changes to disk, creating necessary subdirectories, if the buffer is modified and then quit. Accepts an optional path (:exit! some/path.txt).\nAliases: x!, xit!",
    },
    command_doc {
        name: "fmt",
        doc: "Alias for :format. Format the file using an external formatter or language server.",
    },
    command_doc {
        name: "format",
        doc: "Format the file using an external formatter or language server.\nAliases: fmt",
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
        name: "hnew",
        doc: "Alias for :hsplit-new. Open a scratch buffer in a horizontal split.",
    },
    command_doc {
        name: "hs",
        doc: "Alias for :hsplit. Open the file in a horizontal split.",
    },
    command_doc {
        name: "hsplit",
        doc: "Open the file in a horizontal split.\nAliases: hs, sp",
    },
    command_doc {
        name: "hsplit-new",
        doc: "Open a scratch buffer in a horizontal split.\nAliases: hnew",
    },
    command_doc {
        name: "indent-style",
        doc: "Set the indentation style for editing. ('t' for tabs or 1-16 for number of spaces.)",
    },
    command_doc {
        name: "insert-output",
        doc: "Run shell command, inserting output before each selection.",
    },
    command_doc {
        name: "lang",
        doc: "Alias for :set-language. Set the language of current buffer (show current language if no value specified).",
    },
    command_doc {
        name: "lat",
        doc: "Alias for :later. Jump to a later point in edit history. Accepts a number of steps or a time span.",
    },
    command_doc {
        name: "later",
        doc: "Jump to a later point in edit history. Accepts a number of steps or a time span.\nAliases: lat",
    },
    command_doc {
        name: "log-open",
        doc: "Open the helix log file.",
    },
    command_doc {
        name: "lsp-restart",
        doc: "Restarts the given language servers, or all language servers that are used by the current file if no arguments are supplied",
    },
    command_doc {
        name: "lsp-stop",
        doc: "Stops the given language servers, or all language servers that are used by the current file if no arguments are supplied",
    },
    command_doc {
        name: "lsp-workspace-command",
        doc: "Open workspace command picker",
    },
    command_doc {
        name: "move",
        doc: "Move the current buffer and its corresponding file to a different path\nAliases: mv",
    },
    command_doc {
        name: "move!",
        doc: "Move the current buffer and its corresponding file to a different path creating necessary subdirectories\nAliases: mv!",
    },
    command_doc {
        name: "mv",
        doc: "Alias for :move. Move the current buffer and its corresponding file to a different path",
    },
    command_doc {
        name: "mv!",
        doc: "Alias for :move!. Move the current buffer and its corresponding file to a different path creating necessary subdirectories",
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
        doc: "Open a file from disk into the current view.\nAliases: o, edit, e",
    },
    command_doc {
        name: "pipe",
        doc: "Pipe each selection to the shell command.\nAliases: |",
    },
    command_doc {
        name: "pipe-to",
        doc: "Pipe each selection to the shell command, ignoring output.",
    },
    command_doc {
        name: "primary-clipboard-paste-after",
        doc: "Paste system primary clipboard after selections.",
    },
    command_doc {
        name: "primary-clipboard-paste-before",
        doc: "Paste system primary clipboard before selections.",
    },
    command_doc {
        name: "primary-clipboard-yank",
        doc: "Yank main selection into system primary clipboard.",
    },
    command_doc {
        name: "primary-clipboard-yank-join",
        doc: "Yank joined selections into system primary clipboard. A separator can be provided as first argument. Default value is newline.",
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
        name: "r",
        doc: "Alias for :read. Load a file into buffer",
    },
    command_doc {
        name: "read",
        doc: "Load a file into buffer\nAliases: r",
    },
    command_doc {
        name: "redraw",
        doc: "Clear and re-render the whole UI",
    },
    command_doc {
        name: "reflow",
        doc: "Hard-wrap the current selection of lines to a given width.",
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
        name: "reset-diff-change",
        doc: "Reset the diff change at the cursor position.\nAliases: diffget, diffg",
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
        doc: "Run a shell command\nAliases: sh, !",
    },
    command_doc {
        name: "set",
        doc: "Alias for :set-option. Set a config option at runtime.",
    },
    command_doc {
        name: "set-language",
        doc: "Set the language of current buffer (show current language if no value specified).\nAliases: lang",
    },
    command_doc {
        name: "set-option",
        doc: "Set a config option at runtime.\nAliases: set",
    },
    command_doc {
        name: "set-register",
        doc: "Set contents of the given register.",
    },
    command_doc {
        name: "sh",
        doc: "Alias for :run-shell-command. Run a shell command",
    },
    command_doc {
        name: "sort",
        doc: "Sort ranges in selection.",
    },
    command_doc {
        name: "sp",
        doc: "Alias for :hsplit. Open the file in a horizontal split.",
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
        name: "vnew",
        doc: "Alias for :vsplit-new. Open a scratch buffer in a vertical split.",
    },
    command_doc {
        name: "vs",
        doc: "Alias for :vsplit. Open the file in a vertical split.",
    },
    command_doc {
        name: "vsplit",
        doc: "Open the file in a vertical split.\nAliases: vs",
    },
    command_doc {
        name: "vsplit-new",
        doc: "Open a scratch buffer in a vertical split.\nAliases: vnew",
    },
    command_doc {
        name: "w",
        doc: "Alias for :write. Write changes to disk. Accepts an optional path (:write some/path.txt)",
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
        doc: "Alias for :write-buffer-close. Write changes to disk and closes the buffer.",
    },
    command_doc {
        name: "wbc!",
        doc: "Alias for :write-buffer-close!. Force write changes to disk creating necessary subdirectories and closes the buffer.",
    },
    command_doc {
        name: "workspace-exclude",
        doc: "Mark the current workspace as never-prompt. Never prompts for trust again.",
    },
    command_doc {
        name: "workspace-trust",
        doc: "Allow language servers and local config for the current workspace.",
    },
    command_doc {
        name: "workspace-untrust",
        doc: "Revoke the current workspace's trust grant or exclusion.",
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
        doc: "Alias for :write-quit-all!. Forcefully write changes from all buffers to disk, creating necessary subdirectories, and close all views (ignoring unsaved changes).",
    },
    command_doc {
        name: "write",
        doc: "Write changes to disk. Accepts an optional path (:write some/path.txt)\nAliases: w",
    },
    command_doc {
        name: "write!",
        doc: "Force write changes to disk creating necessary subdirectories. Accepts an optional path (:write! some/path.txt)\nAliases: w!",
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
        doc: "Write changes to disk and closes the buffer. Accepts an optional path (:write-buffer-close some/path.txt)\nAliases: wbc",
    },
    command_doc {
        name: "write-buffer-close!",
        doc: "Force write changes to disk creating necessary subdirectories and closes the buffer. Accepts an optional path (:write-buffer-close! some/path.txt)\nAliases: wbc!",
    },
    command_doc {
        name: "write-quit",
        doc: "Write changes to disk and close the current view. Accepts an optional path (:wq some/path.txt)\nAliases: wq",
    },
    command_doc {
        name: "write-quit!",
        doc: "Write changes to disk and close the current view forcefully. Accepts an optional path (:wq! some/path.txt)\nAliases: wq!",
    },
    command_doc {
        name: "write-quit-all",
        doc: "Write changes from all buffers to disk and close all views.\nAliases: wqa, xa",
    },
    command_doc {
        name: "write-quit-all!",
        doc: "Forcefully write changes from all buffers to disk, creating necessary subdirectories, and close all views (ignoring unsaved changes).\nAliases: wqa!, xa!",
    },
    command_doc {
        name: "x",
        doc: "Alias for :exit. Write changes to disk if the buffer is modified and then quit.",
    },
    command_doc {
        name: "x!",
        doc: "Alias for :exit!. Force write changes to disk, creating necessary subdirectories, if the buffer is modified and then quit.",
    },
    command_doc {
        name: "xa",
        doc: "Alias for :write-quit-all. Write changes from all buffers to disk and close all views.",
    },
    command_doc {
        name: "xa!",
        doc: "Alias for :write-quit-all!. Forcefully write changes from all buffers to disk, creating necessary subdirectories, and close all views (ignoring unsaved changes).",
    },
    command_doc {
        name: "xit",
        doc: "Alias for :exit. Write changes to disk if the buffer is modified and then quit.",
    },
    command_doc {
        name: "xit!",
        doc: "Alias for :exit!. Force write changes to disk, creating necessary subdirectories, if the buffer is modified and then quit.",
    },
    command_doc {
        name: "yank-diagnostic",
        doc: "Yank diagnostic(s) under primary cursor to register, or clipboard by default",
    },
    command_doc {
        name: "yank-join",
        doc: "Yank joined selections. A separator can be provided as first argument. Default value is newline.",
    },
    command_doc {
        name: "|",
        doc: "Alias for :pipe. Pipe each selection to the shell command.",
    },
];

/// Direction for cycling completion candidate selections.
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
    let token = prompt.input.trim_start().split_whitespace().next().unwrap_or("");
    if token.is_empty() {
        return None;
    }
    COMMANDS
        .iter()
        .find(|c| c.name == token)
        .map(|c| (c.name, c.doc))
}
