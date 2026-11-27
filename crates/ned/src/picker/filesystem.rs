//! Directory scanning and gitignore-aware candidate file collection for picker.

use std::fs;
use std::path::Path;

/// Recursively traverses directory to collect candidate file paths relative to `root`,
/// strictly respecting active `.gitignore` rules via `ignore_stack`.
pub fn collect_files(
    root: &Path,
    current: &Path,
    ignore_stack: &mut ned_git::git_ignore_stack,
    out: &mut Vec<String>,
    depth: usize,
) {
    if depth > 12 {
        return;
    }

    let pushed = ignore_stack.push_dir(current);

    if let Ok(entries) = fs::read_dir(current) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Always ignore internal git tracking metadata
            if name_str == ".git" {
                continue;
            }

            let path = entry.path();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

            if ignore_stack.is_ignored(&path, is_dir) {
                continue;
            }

            if is_dir {
                collect_files(root, &path, ignore_stack, out, depth + 1);
            } else if path.is_file() {
                if let Ok(rel) = path.strip_prefix(root) {
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    }

    if pushed {
        ignore_stack.pop();
    }
}
