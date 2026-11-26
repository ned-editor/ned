//! Directory and configuration file path resolution for ned.
//!
//! Exclusively targets `~/.ned/` for global user configurations and `.ned/`
//! for workspace-local project configurations.

use std::env;
use std::path::{Path, PathBuf};

/// Returns the user's global Ned configuration directory (`~/.ned/`).
#[must_use]
pub fn config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(profile) = env::var("USERPROFILE") {
            return PathBuf::from(profile).join(".ned");
        }
        if let Ok(appdata) = env::var("APPDATA") {
            return PathBuf::from(appdata).join(".ned");
        }
        PathBuf::from(r"C:\").join(".ned")
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home).join(".ned");
        }
        PathBuf::from("/").join(".ned")
    }
}

/// Discovers the project-local `.ned/` directory by traversing upwards from the current directory.
///
/// Returns `None` if no `.ned` folder is present in current directory or any ancestor.
#[must_use]
pub fn workspace_ned_dir() -> Option<PathBuf> {
    let current = env::current_dir().ok()?;
    let mut check_dir: &Path = &current;

    loop {
        let ned_path = check_dir.join(".ned");
        if ned_path.is_dir() {
            return Some(ned_path);
        }
        match check_dir.parent() {
            Some(parent) => check_dir = parent,
            None => break,
        }
    }
    None
}

/// Finds the root directory of the active workspace.
///
/// Inspects ancestors for `.ned/` or `.git/`. Defaults to current working directory.
#[must_use]
pub fn find_workspace_root() -> PathBuf {
    if let Ok(current) = env::current_dir() {
        let mut check_dir: &Path = &current;
        loop {
            if check_dir.join(".ned").is_dir() || check_dir.join(".git").exists() {
                return check_dir.to_path_buf();
            }
            match check_dir.parent() {
                Some(parent) => check_dir = parent,
                None => break,
            }
        }
        current
    } else {
        PathBuf::from(".")
    }
}

/// Returns the path to global `~/.ned/config.toml`.
#[must_use]
pub fn global_config_file() -> PathBuf {
    config_dir().join("config.toml")
}

/// Returns the path to workspace-local `.ned/config.toml` if `.ned` directory exists.
#[must_use]
pub fn workspace_config_file() -> Option<PathBuf> {
    workspace_ned_dir().map(|dir| dir.join("config.toml"))
}

/// Returns the path to global `~/.ned/languages.toml`.
#[must_use]
pub fn global_languages_file() -> PathBuf {
    config_dir().join("languages.toml")
}

/// Returns the path to workspace-local `.ned/languages.toml` if `.ned` directory exists.
#[must_use]
pub fn workspace_languages_file() -> Option<PathBuf> {
    workspace_ned_dir().map(|dir| dir.join("languages.toml"))
}

/// Returns theme search directories ordered from highest to lowest priority.
#[must_use]
pub fn theme_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(ws_dir) = workspace_ned_dir() {
        dirs.push(ws_dir.join("themes"));
    }
    dirs.push(config_dir().join("themes"));
    dirs
}

/// Searches theme directories for a theme file matching `<theme_name>.toml`.
#[must_use]
pub fn find_theme_file(theme_name: &str) -> Option<PathBuf> {
    let filename = format!("{theme_name}.toml");
    for dir in theme_dirs() {
        let candidate = dir.join(&filename);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
