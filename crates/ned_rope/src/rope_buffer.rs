//! high-level rope text buffer providing efficient edits and coordinate conversions.

use std::fmt;
use std::ops::Range;

use crate::tree_metric::metric;
use crate::tree_node::{max_children, max_leaf_bytes, node};

/// a balanced, chunked b-tree rope for text editing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct rope {
    root: node,
}

impl Default for rope {
    fn default() -> Self {
        Self::new()
    }
}

impl rope {
    /// creates an empty rope buffer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            root: node::empty_leaf(),
        }
    }

    /// creates a rope from an existing string.
    #[must_use]
    pub fn from_str(text: &str) -> Self {
        if text.len() <= max_leaf_bytes {
            return Self {
                root: node::new_leaf(text.to_string()),
            };
        }

        let mut leaves = Vec::new();
        let mut start = 0;
        let bytes = text.as_bytes();
        let total = bytes.len();

        while start < total {
            let target_end = (start + max_leaf_bytes).min(total);
            let mut split = target_end;
            while split < total && !text.is_char_boundary(split) {
                split += 1;
            }
            let chunk = &text[start..split];
            leaves.push(node::new_leaf(chunk.to_string()));
            start = split;
        }

        let mut current_level = leaves;
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in current_level.chunks_mut(max_children) {
                let mut int_node = node::internal {
                    children: chunk.to_vec(),
                    metric: metric::default(),
                };
                int_node.update_metric();
                next_level.push(int_node);
            }
            current_level = next_level;
        }

        Self {
            root: current_level.pop().unwrap_or_else(node::empty_leaf),
        }
    }

    /// returns the total length in bytes.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.root.metric().byte_len
    }

    /// returns the total count of unicode scalar characters.
    #[must_use]
    pub fn char_len(&self) -> usize {
        self.root.metric().char_len
    }

    /// returns true if the buffer has zero bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.byte_len() == 0
    }

    /// returns total line count (empty buffer has 1 empty line).
    #[must_use]
    pub fn line_count(&self) -> usize {
        if self.is_empty() {
            1
        } else {
            self.root.metric().newlines + 1
        }
    }

    /// inserts a string slice at the given byte offset.
    pub fn insert_str(&mut self, byte_offset: usize, text: &str) {
        if text.is_empty() {
            return;
        }
        let offset = byte_offset.min(self.byte_len());
        if let Some(right_sibling) = insert_into_node(&mut self.root, offset, text) {
            let old_root = std::mem::replace(&mut self.root, node::empty_leaf());
            let mut new_root = node::internal {
                children: vec![old_root, right_sibling],
                metric: metric::default(),
            };
            new_root.update_metric();
            self.root = new_root;
        }
    }

    /// inserts a single unicode character at the given byte offset.
    pub fn insert_char(&mut self, byte_offset: usize, ch: char) {
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        self.insert_str(byte_offset, s);
    }

    /// deletes a range of bytes from the rope.
    pub fn delete_bytes(&mut self, range: Range<usize>) {
        if range.start >= range.end || range.start >= self.byte_len() {
            return;
        }
        let bounded_end = range.end.min(self.byte_len());
        delete_from_node(&mut self.root, range.start, bounded_end);

        // collapse root if it has become an internal node with a single child
        if let node::internal { children, .. } = &mut self.root {
            if children.len() == 1 {
                let only_child = children.pop().unwrap();
                self.root = only_child;
            }
        }
    }

    /// returns the starting byte offset of a 0-based line index.
    #[must_use]
    pub fn line_to_byte(&self, line_idx: usize) -> Option<usize> {
        self.root.line_to_byte(line_idx)
    }

    /// returns the 0-based line index containing a byte offset.
    #[must_use]
    pub fn byte_to_line(&self, byte_offset: usize) -> usize {
        self.root.byte_to_line(byte_offset)
    }

    /// converts a character index to its byte offset.
    #[must_use]
    pub fn char_to_byte(&self, char_idx: usize) -> usize {
        self.root.char_to_byte(char_idx)
    }

    /// converts a byte offset to its character index.
    #[must_use]
    pub fn byte_to_char(&self, byte_offset: usize) -> usize {
        self.root.byte_to_char(byte_offset)
    }

    /// extracts the content of a single 0-based line including trailing newline if present.
    #[must_use]
    pub fn line(&self, line_idx: usize) -> Option<String> {
        let start = self.line_to_byte(line_idx)?;
        let end = match self.line_to_byte(line_idx + 1) {
            Some(next_start) => next_start,
            None => self.byte_len(),
        };
        Some(self.slice_to_string(start..end))
    }

    /// extracts a byte range slice into an owned String.
    #[must_use]
    pub fn slice_to_string(&self, range: Range<usize>) -> String {
        let mut buf = String::with_capacity(range.end.saturating_sub(range.start));
        self.root.slice_bytes(range.start, range.end, &mut buf);
        buf
    }

    /// converts the entire rope into an owned String.
    #[must_use]
    pub fn to_string_lossy(&self) -> String {
        let mut buf = String::with_capacity(self.byte_len());
        self.root.collect_string(&mut buf);
        buf
    }

    /// returns the character at the specified 0-based character index.
    #[must_use]
    pub fn char_at(&self, char_idx: usize) -> Option<char> {
        if char_idx >= self.char_len() {
            return None;
        }
        let start_byte = self.char_to_byte(char_idx);
        let end_byte = self.char_to_byte(char_idx + 1);
        let s = self.slice_to_string(start_byte..end_byte);
        s.chars().next()
    }

    /// extracts a character range into an owned String.
    #[must_use]
    pub fn slice_chars(&self, range: Range<usize>) -> String {
        let start_byte = self.char_to_byte(range.start);
        let end_byte = self.char_to_byte(range.end);
        self.slice_to_string(start_byte..end_byte)
    }
}

impl fmt::Display for rope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.to_string_lossy();
        write!(f, "{s}")
    }
}

/// recursively inserts text into a b-tree node, returning a split sibling if capacity exceeded.
fn insert_into_node(target: &mut node, byte_offset: usize, text: &str) -> Option<node> {
    match target {
        node::leaf {
            text: leaf_text, ..
        } => {
            let offset = byte_offset.min(leaf_text.len());
            leaf_text.insert_str(offset, text);
            target.update_metric();
            target.split_overflow()
        }
        node::internal { children, .. } => {
            let mut current_offset = 0;
            let mut target_idx = children.len().saturating_sub(1);

            for (idx, child) in children.iter().enumerate() {
                let child_len = child.metric().byte_len;
                if byte_offset <= current_offset + child_len {
                    target_idx = idx;
                    break;
                }
                current_offset += child_len;
            }

            let sub_offset = byte_offset.saturating_sub(current_offset);
            let maybe_split = insert_into_node(&mut children[target_idx], sub_offset, text);
            if let Some(new_child) = maybe_split {
                children.insert(target_idx + 1, new_child);
            }
            target.update_metric();
            target.split_overflow()
        }
    }
}

/// recursively deletes bytes in a specified range from a b-tree node.
fn delete_from_node(target: &mut node, start: usize, end: usize) {
    if start >= end {
        return;
    }
    match target {
        node::leaf {
            text: leaf_text, ..
        } => {
            let bounded_start = start.min(leaf_text.len());
            let bounded_end = end.min(leaf_text.len());
            if bounded_start < bounded_end {
                leaf_text.drain(bounded_start..bounded_end);
                target.update_metric();
            }
        }
        node::internal { children, .. } => {
            let mut current_offset = 0;
            let mut i = 0;
            while i < children.len() {
                let child_len = children[i].metric().byte_len;
                let child_end = current_offset + child_len;

                if end > current_offset && start < child_end {
                    let sub_start = if start > current_offset {
                        start - current_offset
                    } else {
                        0
                    };
                    let sub_end = if end < child_end {
                        end - current_offset
                    } else {
                        child_len
                    };
                    delete_from_node(&mut children[i], sub_start, sub_end);

                    if children[i].metric().byte_len == 0 && children.len() > 1 {
                        children.remove(i);
                        continue;
                    }
                }

                current_offset = child_end;
                if current_offset >= end {
                    break;
                }
                i += 1;
            }
            target.update_metric();
        }
    }
}
