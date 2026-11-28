//! Command-line argument parsing, health diagnostics, and asset fetching for the Ned editor binary.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use ned_config::dirs::{
    config_dir, find_workspace_root, global_config_file, workspace_config_file,
};

/// Embedded default dark theme definition for zero-dependency runtime asset installation.
const BUNDLED_NED_DARK: &str = include_str!("../../../runtime/themes/ned-dark.toml");

/// Embedded default light theme definition for zero-dependency runtime asset installation.
const BUNDLED_NED_LIGHT: &str = include_str!("../../../runtime/themes/ned-light.toml");

/// Embedded default Rust grammar definition for zero-dependency runtime asset installation.
const BUNDLED_RUST_GRAMMAR: &str = include_str!("../../../runtime/grammars/rust.json");

/// Embedded default JSON grammar definition for zero-dependency runtime asset installation.
const BUNDLED_JSON_GRAMMAR: &str = include_str!("../../../runtime/grammars/json.json");

/// Target file to open into an editor buffer, with an optional one-based initial cursor line.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct file_target {
    /// File path on disk.
    pub path: PathBuf,
    /// 1-based initial cursor line if specified via `+line` or `file:line`.
    pub line: Option<usize>,
}

impl file_target {
    /// Constructs a new `file_target`.
    #[must_use]
    pub fn new(path: PathBuf, line: Option<usize>) -> Self {
        Self { path, line }
    }
}

impl AsRef<Path> for file_target {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl std::ops::Deref for file_target {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

/// Command line parsing errors.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum cli_error {
    /// An unknown or unexpected argument flag was supplied.
    unknown_option(String),
    /// An option flag was supplied without its required parameter value.
    missing_value(String),
}

impl std::fmt::Display for cli_error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::unknown_option(opt) => write!(f, "error: unexpected argument '{opt}' found"),
            Self::missing_value(opt) => write!(f, "error: option '{opt}' requires a value"),
        }
    }
}

impl std::error::Error for cli_error {}

/// Parsed command line options and execution parameters for Ned.
///
/// The boolean flags are independent CLI toggles; grouping them into enums would add
/// indirection without improving clarity, so the excessive-bools lint is allowed here.
#[allow(non_camel_case_types)]
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct cli_args {
    /// Print help information and exit.
    pub help: bool,
    /// Print version information and exit.
    pub version: bool,
    /// Print health diagnostics and system capabilities, then exit.
    pub health: bool,
    /// Fetch and install theme runtime assets into user config, then exit.
    pub fetch_themes: bool,
    /// Fetch and install grammar runtime assets into user config, then exit.
    pub fetch_grammars: bool,
    /// Bypass confirmation and force overwrite of existing files during fetch.
    pub force: bool,
    /// Skip loading workspace and global configuration files.
    pub no_config: bool,
    /// Path to an explicit custom configuration file.
    pub config: Option<PathBuf>,
    /// Path to an explicit custom runtime directory.
    pub runtime: Option<PathBuf>,
    /// Explicit theme override for the session.
    pub theme: Option<String>,
    /// List of file targets to open into editor buffers.
    pub files: Vec<file_target>,
}

impl cli_args {
    /// Parses command-line arguments (excluding program name).
    ///
    /// # Errors
    ///
    /// Returns `cli_error::unknown_option` if an unrecognized option flag is encountered before `--`.
    /// Returns `cli_error::missing_value` if an option requiring a parameter is missing its value.
    ///
    /// # Panics
    ///
    /// Never panics: the `files.last().unwrap()` call is guarded by a preceding
    /// `!parsed.files.is_empty()` check.
    pub fn parse<S: AsRef<str>, I: IntoIterator<Item = S>>(args: I) -> Result<Self, cli_error> {
        let mut parsed = Self::default();
        let mut iter = args.into_iter();
        let mut pending_line: Option<usize> = None;

        while let Some(arg) = iter.next() {
            let s = arg.as_ref();

            if s == "--" {
                // All remaining tokens are treated as file targets, even if starting with '-'
                for rem in iter {
                    let r = rem.as_ref();
                    let (path, line) = parse_file_and_line(r);
                    let resolved_line = line.or_else(|| pending_line.take());
                    parsed.files.push(file_target::new(path, resolved_line));
                }
                break;
            }

            if s == "-h" || s == "--help" {
                parsed.help = true;
            } else if s == "-v" || s == "-V" || s == "--version" {
                parsed.version = true;
            } else if s == "--health" {
                parsed.health = true;
            } else if s == "--fetch-themes" {
                parsed.fetch_themes = true;
            } else if s == "--fetch-grammars" {
                parsed.fetch_grammars = true;
            } else if s == "-f" || s == "--force" {
                parsed.force = true;
            } else if s == "--no-config" {
                parsed.no_config = true;
            } else if s == "-c" || s == "--config" {
                if let Some(cfg_path) = iter.next() {
                    parsed.config = Some(PathBuf::from(cfg_path.as_ref()));
                } else {
                    return Err(cli_error::missing_value("--config".to_string()));
                }
            } else if let Some(stripped) = s.strip_prefix("--config=") {
                parsed.config = Some(PathBuf::from(stripped));
            } else if s == "-r" || s == "--runtime" {
                if let Some(rt_path) = iter.next() {
                    parsed.runtime = Some(PathBuf::from(rt_path.as_ref()));
                } else {
                    return Err(cli_error::missing_value("--runtime".to_string()));
                }
            } else if let Some(stripped) = s.strip_prefix("--runtime=") {
                parsed.runtime = Some(PathBuf::from(stripped));
            } else if s == "-t" || s == "--theme" {
                if let Some(th_name) = iter.next() {
                    parsed.theme = Some(th_name.as_ref().to_string());
                } else {
                    return Err(cli_error::missing_value("--theme".to_string()));
                }
            } else if let Some(stripped) = s.strip_prefix("--theme=") {
                parsed.theme = Some(stripped.to_string());
            } else if let Some(stripped) = s.strip_prefix('+') {
                if let Ok(line_num) = stripped.parse::<usize>() {
                    if !parsed.files.is_empty() && parsed.files.last().unwrap().line.is_none() {
                        let last_idx = parsed.files.len() - 1;
                        parsed.files[last_idx].line = Some(line_num);
                    } else {
                        pending_line = Some(line_num);
                    }
                } else {
                    return Err(cli_error::unknown_option(s.to_string()));
                }
            } else if s.starts_with('-') && s != "-" {
                return Err(cli_error::unknown_option(s.to_string()));
            } else {
                let (path, line) = parse_file_and_line(s);
                let resolved_line = line.or_else(|| pending_line.take());
                parsed.files.push(file_target::new(path, resolved_line));
            }
        }

        Ok(parsed)
    }

    /// Returns standard help documentation text.
    #[must_use]
    pub fn help_text() -> &'static str {
        concat!(
            "ned: fast, disciplined modal terminal text editor\n\n",
            "Usage: ned [OPTIONS] [FILES]...\n\n",
            "Arguments:\n",
            "  [FILES]...               Files to open in editor buffers (+<line> or <file>:<line>)\n\n",
            "Options:\n",
            "  -c, --config <FILE>      Specify path to config file\n",
            "  -r, --runtime <DIR>      Specify path to runtime directory\n",
            "  -t, --theme <NAME>       Specify theme override for the session\n",
            "      --no-config          Skip loading workspace and global config files\n",
            "      --health             Print health diagnostics and system capabilities\n",
            "      --fetch-themes       Fetch and install official theme files\n",
            "      --fetch-grammars     Fetch and install syntax grammar files\n",
            "  -f, --force              Force overwrite existing files during fetch\n",
            "  -h, --help               Print help information\n",
            "  -V, -v, --version        Print version information\n",
            "      --                   Stop parsing command line options\n"
        )
    }

    /// Returns version text.
    #[must_use]
    pub fn version_text() -> String {
        format!("ned {}", env!("CARGO_PKG_VERSION"))
    }

    /// Returns paths to all files specified on the command line.
    #[must_use]
    pub fn file_paths(&self) -> Vec<PathBuf> {
        self.files.iter().map(|f| f.path.clone()).collect()
    }

    /// Executes the `--health` command, outputting configuration and environment diagnostics.
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if stdout writing fails.
    pub fn run_health(&self) -> io::Result<()> {
        let mut out = io::stdout().lock();

        writeln!(out, "Ned Health & System Diagnostics")?;
        writeln!(out, "================================")?;
        writeln!(out)?;

        // 1. Configuration diagnostics
        writeln!(out, "Configuration:")?;
        if let Some(ref explicit_cfg) = self.config {
            let status = if explicit_cfg.is_file() {
                "found"
            } else {
                "missing"
            };
            writeln!(
                out,
                "  Explicit config:  {} (status: {})",
                explicit_cfg.display(),
                status
            )?;
        } else {
            let global_cfg = global_config_file();
            let global_status = if global_cfg.is_file() {
                "found"
            } else {
                "missing"
            };
            writeln!(
                out,
                "  Global config:    {} (status: {})",
                global_cfg.display(),
                global_status
            )?;

            match workspace_config_file() {
                Some(ws_cfg) => {
                    let ws_status = if ws_cfg.is_file() { "found" } else { "missing" };
                    writeln!(
                        out,
                        "  Workspace config: {} (status: {})",
                        ws_cfg.display(),
                        ws_status
                    )?;
                }
                None => {
                    writeln!(out, "  Workspace config: none detected")?;
                }
            }
        }

        if self.no_config {
            writeln!(out, "  Config loading:   bypassed (--no-config active)")?;
        }

        if let Some(ref theme_override) = self.theme {
            writeln!(out, "  Theme override:   {theme_override}")?;
        }
        writeln!(out)?;

        // 2. Runtime directory diagnostics
        writeln!(out, "Runtime:")?;
        let active_runtime = if let Some(ref rt) = self.runtime {
            rt.clone()
        } else {
            let ws_rt = find_workspace_root().join("runtime");
            if ws_rt.is_dir() {
                ws_rt
            } else {
                config_dir().join("runtime")
            }
        };

        let rt_exists = active_runtime.is_dir();
        let (themes_count, grammars_count) = count_runtime_assets(&active_runtime);
        let total_assets = themes_count + grammars_count;

        writeln!(
            out,
            "  Runtime path:     {} (status: {})",
            active_runtime.display(),
            if rt_exists { "found" } else { "missing" }
        )?;
        writeln!(
            out,
            "  Asset count:      {total_assets} total ({themes_count} themes, {grammars_count} grammars)"
        )?;
        writeln!(out)?;

        // 3. Clipboard diagnostics
        writeln!(out, "System Capabilities:")?;
        let clip_provider = ned_core::clipboard::provider_name();
        writeln!(out, "  Clipboard:        {clip_provider}")?;

        // 4. Git diagnostics
        let git_version = match Command::new("git").arg("--version").output() {
            Ok(cmd_out) if cmd_out.status.success() => {
                String::from_utf8_lossy(&cmd_out.stdout).trim().to_string()
            }
            _ => "not found / unavailable".to_string(),
        };
        writeln!(out, "  Git executable:   {git_version}")?;

        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let git_status = match ned_git::discover_repo(&cwd) {
            Some(root) => {
                let head = ned_git::read_head(&root).unwrap_or_else(|| "detached".to_string());
                format!("active repository at {} (head: {head})", root.display())
            }
            None => "not inside a git repository".to_string(),
        };
        writeln!(out, "  Git repository:   {git_status}")?;

        // 5. Terminal true-color and color capabilities
        let colorterm = env::var("COLORTERM").unwrap_or_default();
        let term = env::var("TERM").unwrap_or_default();

        let true_color_status = match colorterm.as_str() {
            "truecolor" | "24bit" => "supported (24-bit RGB, COLORTERM=truecolor)",
            _ => {
                if term.contains("direct") || term.contains("truecolor") {
                    "supported (24-bit RGB, indicated by TERM)"
                } else {
                    "unsupported / unindicated (COLORTERM unset)"
                }
            }
        };
        writeln!(out, "  Terminal true-color: {true_color_status}")?;

        let color_status = if term.is_empty() || term == "dumb" {
            "monochrome / disabled (TERM=dumb or unset)"
        } else if term.contains("256color") {
            "256-color palette supported (TERM=256color)"
        } else {
            "standard ANSI palette supported"
        };
        writeln!(out, "  Terminal colors:  {color_status}")?;

        out.flush()
    }

    /// Fetches and installs official themes into `~/.ned/runtime/themes/`.
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if creating directories or writing theme files fails.
    pub fn run_fetch_themes(&self) -> io::Result<()> {
        let dest_dir = if let Some(ref rt) = self.runtime {
            rt.join("themes")
        } else {
            config_dir().join("runtime").join("themes")
        };

        if !check_curl_available() {
            println!("curl is not installed or not found in PATH.");
            println!("Manual installation instructions:");
            println!("  1. Install curl using your system package manager.");
            println!("  2. Or manually copy theme TOML files into:");
            println!("     {}", dest_dir.display());
            return Ok(());
        }

        fs::create_dir_all(&dest_dir)?;

        println!(
            "Fetching official theme assets into {}...",
            dest_dir.display()
        );

        let mut assets: Vec<(String, String)> = Vec::new();

        // 1. Discover repo themes if running in or near a source tree
        let repo_themes = find_workspace_root().join("runtime").join("themes");
        if repo_themes.is_dir() {
            if let Ok(entries) = fs::read_dir(&repo_themes) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().is_some_and(|ext| ext == "toml") {
                        if let (Some(name), Ok(content)) =
                            (path.file_name(), fs::read_to_string(&path))
                        {
                            assets.push((name.to_string_lossy().to_string(), content));
                        }
                    }
                }
            }
        }

        // 2. Discover fork themes if available
        let fork_themes = find_workspace_root()
            .join("fork")
            .join("runtime")
            .join("themes");
        if fork_themes.is_dir() {
            if let Ok(entries) = fs::read_dir(&fork_themes) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().is_some_and(|ext| ext == "toml") {
                        if let (Some(name), Ok(content)) =
                            (path.file_name(), fs::read_to_string(&path))
                        {
                            let name_str = name.to_string_lossy().to_string();
                            if !assets.iter().any(|(n, _)| *n == name_str) {
                                assets.push((name_str, content));
                            }
                        }
                    }
                }
            }
        }

        // 3. Ensure core bundled defaults are always available
        if !assets.iter().any(|(n, _)| n == "ned-dark.toml") {
            assets.push(("ned-dark.toml".to_string(), BUNDLED_NED_DARK.to_string()));
        }
        if !assets.iter().any(|(n, _)| n == "ned-light.toml") {
            assets.push(("ned-light.toml".to_string(), BUNDLED_NED_LIGHT.to_string()));
        }

        let mut installed = 0;
        let mut skipped = 0;

        for (name, content) in &assets {
            let target_path = dest_dir.join(name);
            if target_path.exists() && !self.force {
                println!("  skipped {name} (already exists, use --force to overwrite)");
                skipped += 1;
                continue;
            }

            atomic_write_file(&target_path, content)?;
            println!("  installed {name}");
            installed += 1;
        }

        println!("Themes installation complete: {installed} installed, {skipped} skipped.");
        Ok(())
    }

    /// Fetches and installs official syntax grammars into `~/.ned/runtime/grammars/`.
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if creating directories or writing grammar files fails.
    pub fn run_fetch_grammars(&self) -> io::Result<()> {
        let dest_dir = if let Some(ref rt) = self.runtime {
            rt.join("grammars")
        } else {
            config_dir().join("runtime").join("grammars")
        };

        if !check_curl_available() {
            println!("curl is not installed or not found in PATH.");
            println!("Manual installation instructions:");
            println!("  1. Install curl using your system package manager.");
            println!("  2. Or manually copy grammar JSON files into:");
            println!("     {}", dest_dir.display());
            return Ok(());
        }

        fs::create_dir_all(&dest_dir)?;

        println!(
            "Fetching official grammar assets into {}...",
            dest_dir.display()
        );

        let mut assets: Vec<(String, String)> = Vec::new();

        // 1. Discover repo grammars if running in source checkout
        let repo_grammars = find_workspace_root().join("runtime").join("grammars");
        if repo_grammars.is_dir() {
            if let Ok(entries) = fs::read_dir(&repo_grammars) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                        if let (Some(name), Ok(content)) =
                            (path.file_name(), fs::read_to_string(&path))
                        {
                            assets.push((name.to_string_lossy().to_string(), content));
                        }
                    }
                }
            }
        }

        // 2. Ensure core bundled defaults are always available
        if !assets.iter().any(|(n, _)| n == "rust.json") {
            assets.push(("rust.json".to_string(), BUNDLED_RUST_GRAMMAR.to_string()));
        }
        if !assets.iter().any(|(n, _)| n == "json.json") {
            assets.push(("json.json".to_string(), BUNDLED_JSON_GRAMMAR.to_string()));
        }

        let mut installed = 0;
        let mut skipped = 0;

        for (name, content) in &assets {
            let target_path = dest_dir.join(name);
            if target_path.exists() && !self.force {
                println!("  skipped {name} (already exists, use --force to overwrite)");
                skipped += 1;
                continue;
            }

            atomic_write_file(&target_path, content)?;
            println!("  installed {name}");
            installed += 1;
        }

        println!("Grammars installation complete: {installed} installed, {skipped} skipped.");
        Ok(())
    }
}

/// Parses a token into a path and an optional trailing `:line` number.
fn parse_file_and_line(s: &str) -> (PathBuf, Option<usize>) {
    if let Some((path_part, line_part)) = s.rsplit_once(':') {
        // Exclude single-letter Windows drive roots like C:
        let is_drive_letter =
            path_part.len() == 1 && path_part.chars().next().unwrap().is_ascii_alphabetic();
        if !is_drive_letter && !path_part.is_empty() {
            if let Ok(line_num) = line_part.parse::<usize>() {
                return (PathBuf::from(path_part), Some(line_num));
            }
        }
    }

    (PathBuf::from(s), None)
}

/// Probes the system for an executable `curl` binary.
///
/// The `map`+`unwrap_or` combination is clearer here than `map_or` with a closure,
/// so the `map_unwrap_or` lint is allowed.
#[allow(clippy::map_unwrap_or)]
fn check_curl_available() -> bool {
    Command::new("curl")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Counts theme and grammar files located inside the given runtime directory.
fn count_runtime_assets(dir: &Path) -> (usize, usize) {
    let mut themes = 0;
    let mut grammars = 0;

    let themes_dir = dir.join("themes");
    if let Ok(entries) = fs::read_dir(themes_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|e| e == "toml") {
                themes += 1;
            }
        }
    }

    let grammars_dir = dir.join("grammars");
    if let Ok(entries) = fs::read_dir(grammars_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|e| e == "json") {
                grammars += 1;
            }
        }
    }

    (themes, grammars)
}

/// Atomically writes content to a target file via a sibling temporary file.
fn atomic_write_file(target: &Path, content: &str) -> io::Result<()> {
    let parent = target
        .parent()
        .ok_or_else(|| io::Error::other("target has no parent directory"))?;
    let file_stem = target
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    let temp_name = format!(".tmp_{}_{file_stem}", std::process::id());
    let temp_path = parent.join(temp_name);

    {
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
    }

    fs::rename(temp_path, target)
}
