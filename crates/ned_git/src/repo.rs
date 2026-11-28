//! git repository interactions and queries.

use std::fs;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// information about an active git repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct repo_info {
    /// absolute path to the repository working directory root.
    pub root: PathBuf,
    /// current branch name or short commit hash.
    pub head: Option<String>,
}

/// representation of the git status of a single file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum file_status {
    /// file matches git index and head.
    unmodified,
    /// file contains uncommitted modifications.
    modified,
    /// file is newly staged or added.
    added,
    /// file has been removed.
    deleted,
    /// file is not tracked by git.
    untracked,
}

/// discovers the git repository root by traversing upward from `start_path`.
#[must_use]
pub fn discover_repo(start_path: &Path) -> Option<PathBuf> {
    let mut current = if start_path.is_file() {
        start_path.parent()?
    } else {
        start_path
    };

    loop {
        let dot_git = current.join(".git");
        if dot_git.exists() {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
}

/// reads the current head branch name or commit id from the repository root.
#[must_use]
pub fn read_head(repo_root: &Path) -> Option<String> {
    let head_path = repo_root.join(".git").join("HEAD");
    let content = fs::read_to_string(head_path).ok()?;
    let trimmed = content.trim();

    if let Some(branch_ref) = trimmed.strip_prefix("ref: refs/heads/") {
        return Some(branch_ref.to_string());
    }

    if trimmed.len() >= 7 {
        return Some(trimmed[..7].to_string());
    }

    Some(trimmed.to_string())
}

/// converts a relative path to standard git path notation with forward slashes.
fn normalize_git_path(relative_path: &Path) -> String {
    let path_str = relative_path.to_string_lossy();
    path_str.replace('\\', "/")
}

/// fetches the committed base content of a file using `git show :<path>`.
///
/// # Errors
///
/// returns `io::Error` if git command fails or file is not in git index.
pub fn fetch_file_base(repo_root: &Path, relative_path: &Path) -> Result<String> {
    let git_path = normalize_git_path(relative_path);
    let target_spec = format!(":{git_path}");

    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["show", &target_spec])
        .output()?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(Error::new(ErrorKind::NotFound, err_msg.trim().to_string()));
    }

    String::from_utf8(output.stdout).map_err(|e| Error::new(ErrorKind::InvalidData, e))
}

/// queries the git status of a specific file.
///
/// # Errors
///
/// returns `io::Error` if git execution fails.
pub fn query_file_status(repo_root: &Path, relative_path: &Path) -> Result<file_status> {
    let git_path = normalize_git_path(relative_path);

    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["status", "--porcelain=v1", "--", &git_path])
        .output()?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(Error::other(err_msg.trim().to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next();

    let Some(line) = first_line else {
        return Ok(file_status::unmodified);
    };

    if line.starts_with("??") {
        return Ok(file_status::untracked);
    }

    let bytes = line.as_bytes();
    let index_stat = bytes.first().copied().unwrap_or(b' ');
    let work_stat = bytes.get(1).copied().unwrap_or(b' ');

    if index_stat == b'A' || work_stat == b'A' {
        Ok(file_status::added)
    } else if index_stat == b'D' || work_stat == b'D' {
        Ok(file_status::deleted)
    } else if index_stat == b'M' || work_stat == b'M' {
        Ok(file_status::modified)
    } else {
        Ok(file_status::unmodified)
    }
}

/// asynchronously fetches the committed base content using yokio's blocking worker pool.
pub fn fetch_file_base_async<f: FnOnce(Result<String>) + 'static>(
    repo_root: PathBuf,
    relative_path: PathBuf,
    cb: f,
) {
    yokio::blocking(move || fetch_file_base(&repo_root, &relative_path), cb);
}
