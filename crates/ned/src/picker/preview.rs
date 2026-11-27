//! Preview pane loading, formatting, and file categorization for picker.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Minimum terminal area width required to display the split preview pane.
pub const min_area_width_for_preview: u16 = 72;

/// Maximum file size eligible for live text preview (10MB).
pub const max_file_size_for_preview: u64 = 10 * 1024 * 1024;

/// Content to display in the live preview pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum preview_content {
    /// Plain text lines from the target file.
    text(Vec<String>),
    /// Binary file not suitable for text preview.
    binary,
    /// Directory listing of entries (entry_name, is_dir).
    directory(Vec<(String, bool)>),
    /// File size exceeds maximum preview limit.
    too_large,
    /// File or directory was not found.
    not_found,
}

impl preview_content {
    /// Returns a placeholder message for non-text preview variants.
    #[must_use]
    pub fn placeholder(&self) -> Option<&'static str> {
        match self {
            Self::text(_) => None,
            Self::binary => Some("<Binary file>"),
            Self::directory(_) => None,
            Self::too_large => Some("<File too large to preview>"),
            Self::not_found => Some("<File not found>"),
        }
    }
}

/// Resolves full path and loads preview content for `item`.
#[must_use]
pub fn load_preview_for_item(root_dir: Option<&Path>, item: &str) -> preview_content {
    let full_path = if let Some(root) = root_dir {
        root.join(item)
    } else {
        PathBuf::from(item)
    };
    load_preview_for_path(&full_path)
}

/// Loads preview content for the given file or directory path.
#[must_use]
pub fn load_preview_for_path(path: &Path) -> preview_content {
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return preview_content::not_found,
    };

    if metadata.is_dir() {
        let mut entries = Vec::new();
        if let Ok(dir_entries) = fs::read_dir(path) {
            for entry in dir_entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                entries.push((name, is_dir));
            }
        }
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        return preview_content::directory(entries);
    }

    if metadata.len() > max_file_size_for_preview {
        return preview_content::too_large;
    }

    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return preview_content::not_found,
    };

    let mut reader = std::io::BufReader::new(file);
    let mut initial_buf = [0u8; 1024];
    let n = match reader.read(&mut initial_buf) {
        Ok(n) => n,
        Err(_) => return preview_content::not_found,
    };

    // Look for null byte in the initial read chunk to identify binary files
    if initial_buf[..n].contains(&0) {
        return preview_content::binary;
    }

    let full_content = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return preview_content::binary,
    };

    let lines: Vec<String> = full_content.lines().take(1000).map(String::from).collect();
    preview_content::text(lines)
}
