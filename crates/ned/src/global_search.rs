//! Workspace file traversal and regular expression text search.
//!
//! Provides bounded incremental directory scanning with `.gitignore` awareness,
//! binary file detection, and regex pattern matching powered by `ned_regex`.

use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use ned_git::git_ignore_stack;
use ned_regex::regex;

/// Maximum total search results gathered across workspace files.
pub const MAX_SEARCH_RESULTS: usize = 10_000;
/// Maximum directory traversal depth.
const MAX_DIRECTORY_DEPTH: usize = 16;
/// Number of prefix bytes checked for null bytes to detect binary files.
const BINARY_CHECK_BYTES: usize = 1024;

/// A single search result occurrence inside a workspace file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct search_result {
    /// Relative path from workspace root to the matching file.
    pub path: String,
    /// 1-based line number of the match.
    pub line: usize,
    /// 1-based character column number of the match start.
    pub col: usize,
    /// Number of characters in the matched string.
    pub match_len: usize,
    /// Full line text containing the match.
    pub line_text: String,
}

impl search_result {
    /// Formats the search result into a standard picker display line:
    /// `path:line:col: preview_line`.
    #[must_use]
    pub fn format_display(&self) -> String {
        format!(
            "{}:{}:{}: {}",
            self.path,
            self.line,
            self.col,
            self.line_text.trim_end()
        )
    }
}

/// Scans workspace files under `root` matching `pattern` with smart-case regex.
///
/// Skips `.git`, `target`, `.ned`, hidden files/directories, files excluded by
/// `.gitignore`, and binary files.
///
/// # Errors
///
/// Returns an error string if `pattern` is not a valid regular expression.
pub fn search_workspace(
    root: &Path,
    pattern: &str,
    max_results: usize,
) -> Result<Vec<search_result>, String> {
    if pattern.is_empty() {
        return Ok(Vec::new());
    }

    let re = regex::smart_case(pattern).map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    let mut ignore_stack = git_ignore_stack::new();

    scan_directory(
        root,
        root,
        &re,
        &mut ignore_stack,
        &mut results,
        max_results.min(MAX_SEARCH_RESULTS),
        0,
    );

    Ok(results)
}

/// Recursively traverses directories gathering matching lines until `max_results` is reached.
fn scan_directory(
    root: &Path,
    current: &Path,
    re: &regex,
    ignore_stack: &mut git_ignore_stack,
    results: &mut Vec<search_result>,
    max_results: usize,
    depth: usize,
) {
    if depth > MAX_DIRECTORY_DEPTH || results.len() >= max_results {
        return;
    }

    let pushed = ignore_stack.push_dir(current);

    if let Ok(entries) = std::fs::read_dir(current) {
        for entry in entries.flatten() {
            if results.len() >= max_results {
                break;
            }

            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Skip internal git tracking and common ignored artifact folders
            if name_str == ".git" || name_str == "target" || name_str == ".ned" {
                continue;
            }

            // Skip hidden files and directories
            if name_str.starts_with('.') && name_str != "." && name_str != ".." {
                continue;
            }

            let path = entry.path();
            let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());

            if ignore_stack.is_ignored(&path, is_dir) {
                continue;
            }

            if is_dir {
                scan_directory(
                    root,
                    &path,
                    re,
                    ignore_stack,
                    results,
                    max_results,
                    depth + 1,
                );
            } else if path.is_file() {
                scan_file(root, &path, re, results, max_results);
            }
        }
    }

    if pushed {
        ignore_stack.pop();
    }
}

/// Scans a single text file line by line using a streaming buffer.
fn scan_file(
    root: &Path,
    file_path: &Path,
    re: &regex,
    results: &mut Vec<search_result>,
    max_results: usize,
) {
    if is_binary_file(file_path) {
        return;
    }

    let Ok(file) = File::open(file_path) else {
        return;
    };

    let rel_path = match file_path.strip_prefix(root) {
        Ok(r) => r.to_string_lossy().replace('\\', "/"),
        Err(_) => file_path.to_string_lossy().replace('\\', "/"),
    };

    let mut reader = BufReader::new(file);
    let mut line_buf = String::new();
    let mut line_num = 1;

    while results.len() < max_results {
        line_buf.clear();
        match reader.read_line(&mut line_buf) {
            Ok(0) | Err(_) => break, // EOF or Non-UTF-8 / I/O error
            Ok(_) => {
                for m in re.find_iter(&line_buf) {
                    let col = line_buf[..m.start].chars().count() + 1;
                    let match_len = line_buf[m.start..m.end].chars().count();
                    results.push(search_result {
                        path: rel_path.clone(),
                        line: line_num,
                        col,
                        match_len,
                        line_text: line_buf.clone(),
                    });
                    if results.len() >= max_results {
                        break;
                    }
                }
                line_num += 1;
            }
        }
    }
}

/// Detects binary files by checking for null bytes in the first prefix chunk.
fn is_binary_file(path: &Path) -> bool {
    let Ok(mut file) = File::open(path) else {
        return true;
    };

    let mut buf = [0u8; BINARY_CHECK_BYTES];
    match file.read(&mut buf) {
        Ok(n) => buf[..n].contains(&0),
        Err(_) => true,
    }
}
