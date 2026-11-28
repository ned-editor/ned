//! diff engine implementing a lightweight myers line diff algorithm.

/// tag indicating the nature of a diff hunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum diff_tag {
    /// lines were added in the current buffer.
    added,
    /// lines were modified between base and current buffer.
    modified,
    /// lines were deleted from the base buffer.
    deleted,
}

/// a contiguous range of changed lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct hunk {
    /// 0-indexed line start in the base text.
    pub before_start: u32,
    /// number of lines in the base text.
    pub before_count: u32,
    /// 0-indexed line start in the current text.
    pub after_start: u32,
    /// number of lines in the current text.
    pub after_count: u32,
    /// classification of this change.
    pub kind: diff_tag,
}

/// the nature of a single edit step in the myers diff trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum edit_action {
    /// the line is unchanged between base and current.
    keep,
    /// the line was inserted in the current buffer.
    insert,
    /// the line was deleted from the base buffer.
    delete,
}

/// a single step in the myers diff edit script.
#[derive(Debug, Clone, Copy)]
struct edit_step {
    /// the kind of edit this step represents.
    action: edit_action,
    /// index into the base text.
    base_index: usize,
    /// index into the current text.
    current_index: usize,
}

/// computes line-by-line diff hunks comparing `base` to `current`.
#[must_use]
pub fn compute_diff(base: &str, current: &str) -> Vec<hunk> {
    if base == current {
        return Vec::new();
    }

    let base_lines = split_lines(base);
    let current_lines = split_lines(current);

    let edits = myers_diff(&base_lines, &current_lines);
    condense_edits_into_hunks(&edits)
}

/// splits a string into slice lines preserving line boundaries.
fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\n' {
            lines.push(&text[start..=i]);
            start = i + 1;
        }
        i += 1;
    }
    if start < len {
        lines.push(&text[start..len]);
    }
    lines
}

/// implements the myers diff algorithm returning list of edit steps.
fn myers_diff(base: &[&str], current: &[&str]) -> Vec<edit_step> {
    let base_len = base.len();
    let current_len = current.len();
    let max = base_len + current_len;

    if max == 0 {
        return Vec::new();
    }

    let offset = isize::try_from(max).unwrap_or(0);
    let mut v = vec![0isize; 2 * max + 1];
    let mut trace: Vec<Vec<isize>> = Vec::new();

    for distance in 0..=max {
        trace.push(v.clone());
        let d_isize = isize::try_from(distance).unwrap_or(0);
        let mut diagonal = -d_isize;
        while diagonal <= d_isize {
            let k_idx = usize::try_from(diagonal + offset).unwrap_or(0);
            let mut x_pos = if diagonal == -d_isize || (diagonal != d_isize && v[k_idx - 1] < v[k_idx + 1]) {
                v[k_idx + 1]
            } else {
                v[k_idx - 1] + 1
            };
            let mut y_pos = x_pos - diagonal;

            while x_pos < isize::try_from(base_len).unwrap_or(0)
                && y_pos < isize::try_from(current_len).unwrap_or(0)
                && base[usize::try_from(x_pos).unwrap_or(0)] == current[usize::try_from(y_pos).unwrap_or(0)]
            {
                x_pos += 1;
                y_pos += 1;
            }

            v[k_idx] = x_pos;

            if x_pos >= isize::try_from(base_len).unwrap_or(0) && y_pos >= isize::try_from(current_len).unwrap_or(0) {
                return backtrack_trace(&trace, base, current, offset);
            }
            diagonal += 2;
        }
    }

    Vec::new()
}

/// backtracks the myers trace to produce an ordered sequence of edit steps.
fn backtrack_trace(
    trace: &[Vec<isize>],
    base: &[&str],
    current: &[&str],
    offset: isize,
) -> Vec<edit_step> {
    let mut edits = Vec::new();
    let mut x_pos = isize::try_from(base.len()).unwrap_or(0);
    let mut y_pos = isize::try_from(current.len()).unwrap_or(0);

    for (distance, v) in trace.iter().enumerate().rev() {
        let d_isize = isize::try_from(distance).unwrap_or(0);
        let diagonal = x_pos - y_pos;
        let k_idx = usize::try_from(diagonal + offset).unwrap_or(0);

        let prev_diagonal = if diagonal == -d_isize || (diagonal != d_isize && v[k_idx - 1] < v[k_idx + 1]) {
            diagonal + 1
        } else {
            diagonal - 1
        };

        let prev_k_idx = usize::try_from(prev_diagonal + offset).unwrap_or(0);
        let prev_x = v[prev_k_idx];
        let prev_y = prev_x - prev_diagonal;

        while x_pos > prev_x && y_pos > prev_y {
            x_pos -= 1;
            y_pos -= 1;
            edits.push(edit_step {
                action: edit_action::keep,
                base_index: usize::try_from(x_pos).unwrap_or(0),
                current_index: usize::try_from(y_pos).unwrap_or(0),
            });
        }

        if d_isize > 0 {
            if x_pos == prev_x {
                y_pos -= 1;
                edits.push(edit_step {
                    action: edit_action::insert,
                    base_index: usize::try_from(x_pos).unwrap_or(0),
                    current_index: usize::try_from(y_pos).unwrap_or(0),
                });
            } else if y_pos == prev_y {
                x_pos -= 1;
                edits.push(edit_step {
                    action: edit_action::delete,
                    base_index: usize::try_from(x_pos).unwrap_or(0),
                    current_index: usize::try_from(y_pos).unwrap_or(0),
                });
            }
        }
    }

    edits.reverse();
    edits
}

/// groups edit steps into contiguous hunks with line positions.
fn condense_edits_into_hunks(edits: &[edit_step]) -> Vec<hunk> {
    let mut hunks = Vec::new();
    let mut i = 0;
    let len = edits.len();

    while i < len {
        if edits[i].action == edit_action::keep {
            i += 1;
            continue;
        }

        let start_base = edits[i].base_index;
        let start_current = edits[i].current_index;
        let mut count_del = 0u32;
        let mut count_ins = 0u32;

        while i < len && edits[i].action != edit_action::keep {
            match edits[i].action {
                edit_action::delete => count_del += 1,
                edit_action::insert => count_ins += 1,
                edit_action::keep => break,
            }
            i += 1;
        }

        let kind = if count_del > 0 && count_ins > 0 {
            diff_tag::modified
        } else if count_ins > 0 {
            diff_tag::added
        } else {
            diff_tag::deleted
        };

        hunks.push(hunk {
            before_start: u32::try_from(start_base).unwrap_or(0),
            before_count: count_del,
            after_start: u32::try_from(start_current).unwrap_or(0),
            after_count: count_ins,
            kind,
        });
    }

    hunks
}

/// executes diff computation off the main thread using yokio blocking pool.
pub fn compute_diff_async<f: FnOnce(Vec<hunk>) + 'static>(base: String, current: String, cb: f) {
    yokio::blocking(move || compute_diff(&base, &current), cb);
}
