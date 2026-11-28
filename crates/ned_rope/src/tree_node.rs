//! b-tree node representations for the chunked rope data structure.

use crate::tree_metric::metric;

/// maximum byte length of a leaf chunk before splitting.
pub const max_leaf_bytes: usize = 1024;

/// maximum number of children in an internal node before splitting.
pub const max_children: usize = 8;

/// an internal or leaf node in the rope b-tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum node {
    /// leaf chunk storing UTF-8 string text.
    leaf {
        /// text segment contained in this leaf.
        text: String,
        /// cached metrics for this leaf.
        metric: metric,
    },
    /// internal node holding child subtrees.
    internal {
        /// child nodes in order.
        children: Vec<node>,
        /// aggregated metrics for all child subtrees.
        metric: metric,
    },
}

impl node {
    /// creates an empty leaf node.
    #[must_use]
    pub fn empty_leaf() -> Self {
        Self::leaf {
            text: String::new(),
            metric: metric::default(),
        }
    }

    /// creates a leaf from a string slice.
    #[must_use]
    pub fn new_leaf(text: String) -> Self {
        let m = metric::of_str(&text);
        Self::leaf { text, metric: m }
    }

    /// returns cached metrics for this node.
    #[must_use]
    pub fn metric(&self) -> metric {
        match self {
            Self::leaf { metric, .. } | Self::internal { metric, .. } => *metric,
        }
    }

    /// recalculates metrics for internal node from its children.
    pub fn update_metric(&mut self) {
        if let Self::internal { children, metric } = self {
            let mut total = metric::default();
            for child in children.iter() {
                total += child.metric();
            }
            *metric = total;
        } else if let Self::leaf { text, metric } = self {
            *metric = metric::of_str(text);
        }
    }

    /// appends full contents of this node into a target buffer.
    pub fn collect_string(&self, buf: &mut String) {
        match self {
            Self::leaf { text, .. } => buf.push_str(text),
            Self::internal { children, .. } => {
                for child in children {
                    child.collect_string(buf);
                }
            }
        }
    }

    /// collects a byte-range slice from this node into target buffer.
    pub fn slice_bytes(&self, start: usize, end: usize, buf: &mut String) {
        if start >= end {
            return;
        }
        match self {
            Self::leaf { text, .. } => {
                let bounded_start = start.min(text.len());
                let bounded_end = end.min(text.len());
                if bounded_start < bounded_end {
                    buf.push_str(&text[bounded_start..bounded_end]);
                }
            }
            Self::internal { children, .. } => {
                let mut current_offset = 0;
                for child in children {
                    let child_len = child.metric().byte_len;
                    let child_end = current_offset + child_len;

                    if end > current_offset && start < child_end {
                        let sub_start = start.saturating_sub(current_offset);
                        let sub_end = if end < child_end {
                            end - current_offset
                        } else {
                            child_len
                        };
                        child.slice_bytes(sub_start, sub_end, buf);
                    }

                    current_offset = child_end;
                    if current_offset >= end {
                        break;
                    }
                }
            }
        }
    }

    /// finds the byte offset where a given 0-based line index begins.
    #[must_use]
    pub fn line_to_byte(&self, line_idx: usize) -> Option<usize> {
        if line_idx == 0 {
            return Some(0);
        }

        match self {
            Self::leaf { text, metric } => {
                if line_idx > metric.newlines {
                    return None;
                }
                let mut seen_lines = 0;
                for (byte_idx, c) in text.char_indices() {
                    if c == '\n' {
                        seen_lines += 1;
                        if seen_lines == line_idx {
                            return Some(byte_idx + 1);
                        }
                    }
                }
                None
            }
            Self::internal { children, metric } => {
                if line_idx > metric.newlines {
                    return None;
                }
                let mut remaining_lines = line_idx;
                let mut accumulated_bytes = 0;

                for child in children {
                    let child_metric = child.metric();
                    if remaining_lines <= child_metric.newlines {
                        let offset_in_child = child.line_to_byte(remaining_lines)?;
                        return Some(accumulated_bytes + offset_in_child);
                    }
                    remaining_lines -= child_metric.newlines;
                    accumulated_bytes += child_metric.byte_len;
                }
                None
            }
        }
    }

    /// returns the 0-based line index containing the specified byte offset.
    #[must_use]
    pub fn byte_to_line(&self, byte_offset: usize) -> usize {
        match self {
            Self::leaf { text, .. } => {
                let bounded_offset = byte_offset.min(text.len());
                let mut lines = 0;
                for (idx, c) in text.char_indices() {
                    if idx >= bounded_offset {
                        break;
                    }
                    if c == '\n' {
                        lines += 1;
                    }
                }
                lines
            }
            Self::internal { children, .. } => {
                let mut accumulated_lines = 0;
                let mut current_offset = 0;

                for child in children {
                    let child_len = child.metric().byte_len;
                    let next_offset = current_offset + child_len;

                    if byte_offset <= next_offset {
                        let offset_in_child = byte_offset.saturating_sub(current_offset);
                        return accumulated_lines + child.byte_to_line(offset_in_child);
                    }

                    accumulated_lines += child.metric().newlines;
                    current_offset = next_offset;
                }
                accumulated_lines
            }
        }
    }

    /// converts a character index to its byte offset.
    #[must_use]
    pub fn char_to_byte(&self, char_idx: usize) -> usize {
        match self {
            Self::leaf { text, .. } => {
                let mut chars_seen = 0;
                for (byte_idx, _) in text.char_indices() {
                    if chars_seen == char_idx {
                        return byte_idx;
                    }
                    chars_seen += 1;
                }
                text.len()
            }
            Self::internal { children, .. } => {
                let mut remaining_chars = char_idx;
                let mut accumulated_bytes = 0;

                for child in children {
                    let child_chars = child.metric().char_len;
                    if remaining_chars <= child_chars {
                        return accumulated_bytes + child.char_to_byte(remaining_chars);
                    }
                    remaining_chars -= child_chars;
                    accumulated_bytes += child.metric().byte_len;
                }
                accumulated_bytes
            }
        }
    }

    /// converts a byte offset to its character index.
    #[must_use]
    pub fn byte_to_char(&self, byte_offset: usize) -> usize {
        match self {
            Self::leaf { text, .. } => {
                let bounded_offset = byte_offset.min(text.len());
                let mut chars = 0;
                for (idx, _) in text.char_indices() {
                    if idx >= bounded_offset {
                        break;
                    }
                    chars += 1;
                }
                chars
            }
            Self::internal { children, .. } => {
                let mut accumulated_chars = 0;
                let mut current_bytes = 0;

                for child in children {
                    let child_bytes = child.metric().byte_len;
                    let next_bytes = current_bytes + child_bytes;

                    if byte_offset <= next_bytes {
                        let offset_in_child = byte_offset.saturating_sub(current_bytes);
                        return accumulated_chars + child.byte_to_char(offset_in_child);
                    }

                    accumulated_chars += child.metric().char_len;
                    current_bytes = next_bytes;
                }
                accumulated_chars
            }
        }
    }

    /// splits a leaf or internal node if it exceeds capacity.
    pub fn split_overflow(&mut self) -> Option<node> {
        match self {
            Self::leaf { text, .. } => {
                if text.len() <= max_leaf_bytes {
                    return None;
                }
                let mid = text.len() / 2;
                let mut split_point = mid;
                while split_point < text.len() && !text.is_char_boundary(split_point) {
                    split_point += 1;
                }

                let right_text = text.split_off(split_point);
                self.update_metric();

                let right_node = node::new_leaf(right_text);
                Some(right_node)
            }
            Self::internal { children, .. } => {
                if children.len() <= max_children {
                    return None;
                }
                let mid = children.len() / 2;
                let right_children = children.split_off(mid);
                self.update_metric();

                let mut right_node = node::internal {
                    children: right_children,
                    metric: metric::default(),
                };
                right_node.update_metric();
                Some(right_node)
            }
        }
    }
}
