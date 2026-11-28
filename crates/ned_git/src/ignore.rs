//! gitignore file parsing, pattern matching, and hierarchical ignore tracking.

use std::fs;
use std::path::{Path, PathBuf};

/// A single rule parsed from a `.gitignore` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct git_ignore_rule {
    /// Normalized pattern string.
    pub pattern: String,
    /// Whether this is a negative negation rule (starts with `!`).
    pub is_negative: bool,
    /// Whether this rule only matches directories (trailing `/`).
    pub dir_only: bool,
    /// Whether the rule is path-anchored (contains `/`).
    pub anchored: bool,
}

impl git_ignore_rule {
    /// Parses a single line from a `.gitignore` file.
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }

        let is_negative = trimmed.starts_with('!');
        let pattern_str = if is_negative { &trimmed[1..] } else { trimmed };

        let dir_only = pattern_str.ends_with('/');
        let pattern_trimmed = pattern_str.trim_matches('/');

        let anchored = pattern_str.starts_with('/') || pattern_trimmed.contains('/');

        Some(Self {
            pattern: pattern_trimmed.to_string(),
            is_negative,
            dir_only,
            anchored,
        })
    }

    /// Evaluates if `rel_path` matches this rule.
    #[must_use]
    pub fn matches(&self, rel_path: &str, is_dir: bool) -> bool {
        if self.dir_only && !is_dir {
            return false;
        }

        let norm_path = rel_path.trim_matches('/').replace('\\', "/");

        if self.anchored {
            glob_match(&self.pattern, &norm_path)
                || norm_path == self.pattern
                || norm_path.starts_with(&format!("{}/", self.pattern))
        } else {
            if glob_match(&self.pattern, &norm_path)
                || norm_path == self.pattern
                || norm_path.starts_with(&format!("{}/", self.pattern))
            {
                return true;
            }

            for component in norm_path.split('/') {
                if glob_match(&self.pattern, component) || component == self.pattern {
                    return true;
                }
            }
            false
        }
    }
}

/// Matches `text` against a pattern supporting `*`, `**`, and `?`.
#[must_use]
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p_chars: Vec<char> = pattern.chars().collect();
    let t_chars: Vec<char> = text.chars().collect();
    glob_match_slices(&p_chars, &t_chars)
}

/// Recursive slice matcher for glob patterns.
fn glob_match_slices(pattern: &[char], text: &[char]) -> bool {
    match (pattern.first(), text.first()) {
        (None, None) => true,
        (None, Some(_)) => false,
        (Some('*'), _) => {
            if pattern.get(1) == Some(&'*') {
                let rest_p = &pattern[2..];
                let trimmed_p = if rest_p.first() == Some(&'/') {
                    &rest_p[1..]
                } else {
                    rest_p
                };
                if glob_match_slices(trimmed_p, text) {
                    return true;
                }
                if !text.is_empty() && glob_match_slices(pattern, &text[1..]) {
                    return true;
                }
                return false;
            }
            if glob_match_slices(&pattern[1..], text) {
                return true;
            }
            if let Some(&t_first) = text.first() {
                if t_first != '/' && glob_match_slices(pattern, &text[1..]) {
                    return true;
                }
            }
            false
        }
        (Some('?'), Some(&t_first)) => {
            if t_first == '/' {
                false
            } else {
                glob_match_slices(&pattern[1..], &text[1..])
            }
        }
        (Some(&p_first), Some(&t_first)) => {
            if p_first == t_first {
                glob_match_slices(&pattern[1..], &text[1..])
            } else {
                false
            }
        }
        (Some(_), None) => pattern.iter().all(|&c| c == '*'),
    }
}

/// Loads `.gitignore` rules from `dir` if a `.gitignore` file exists.
#[must_use]
pub fn load_gitignore(dir: &Path) -> Vec<git_ignore_rule> {
    let gitignore_path = dir.join(".gitignore");
    if let Ok(content) = fs::read_to_string(gitignore_path) {
        content.lines().filter_map(git_ignore_rule::parse).collect()
    } else {
        Vec::new()
    }
}

/// Hierarchical stack of gitignore rules tracking nested directories.
#[derive(Debug, Clone, Default)]
pub struct git_ignore_stack {
    scopes: Vec<(PathBuf, Vec<git_ignore_rule>)>,
}

impl git_ignore_stack {
    /// Creates a new empty ignore stack.
    #[must_use]
    pub fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    /// Pushes local `.gitignore` rules for `dir` if present. Returns true if rules were added.
    pub fn push_dir(&mut self, dir: &Path) -> bool {
        let rules = load_gitignore(dir);
        if rules.is_empty() {
            false
        } else {
            self.scopes.push((dir.to_path_buf(), rules));
            true
        }
    }

    /// Pops the top directory rules from the stack.
    pub fn pop(&mut self) {
        self.scopes.pop();
    }

    /// Checks if a file or directory at `path` is ignored by any active rules.
    #[must_use]
    pub fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        let mut ignored = false;
        for (scope_dir, rules) in self.scopes.iter().rev() {
            let Ok(rel) = path.strip_prefix(scope_dir) else {
                continue;
            };
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            for rule in rules {
                if rule.matches(&rel_str, is_dir) {
                    ignored = !rule.is_negative;
                }
            }
        }
        ignored
    }
}
