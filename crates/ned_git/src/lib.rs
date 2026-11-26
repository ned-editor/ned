//! ned_git: git integration, repository discovery, and line diffing for ned editor.

#![warn(missing_docs)]

pub mod diff;
pub mod repo;

pub use diff::{compute_diff, compute_diff_async, diff_tag, hunk};
pub use repo::{
    discover_repo, fetch_file_base, fetch_file_base_async, file_status, query_file_status,
    read_head, repo_info,
};
