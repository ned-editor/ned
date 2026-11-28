//! Structural syntax adapter deriving parent, child, and sibling relations from AST trees.

use crate::tokens::syntax_kind;
use crate::tree::{ast_node, syntax_tree};

/// Maximum number of structural nodes visited during queries to prevent excessive latency.
pub const MAX_STRUCTURAL_NODES: usize = 100_000;

/// Expands a character selection range `(from, to)` to its enclosing parent syntax node.
#[must_use]
pub fn expand_syntax_selection(
    tree: &syntax_tree,
    from: usize,
    to: usize,
) -> Option<(usize, usize)> {
    let mut candidate = None;
    let mut count = 0;
    find_smallest_enclosing(&tree.root, from, to, &mut candidate, &mut count);
    candidate.map(|node| (node.start, node.end))
}

/// Recursively searches for the smallest AST node strictly larger than `(from, to)`.
fn find_smallest_enclosing<'a>(
    node: &'a ast_node,
    from: usize,
    to: usize,
    best: &mut Option<&'a ast_node>,
    count: &mut usize,
) {
    if *count >= MAX_STRUCTURAL_NODES {
        return;
    }
    *count += 1;

    let node_encloses = node.start <= from && node.end >= to;
    if !node_encloses {
        return;
    }

    let strictly_larger = node.start < from || node.end > to;
    if strictly_larger && node.kind != syntax_kind::root {
        match *best {
            Some(current) => {
                if node.len() < current.len() {
                    *best = Some(node);
                }
            }
            None => {
                *best = Some(node);
            }
        }
    }

    for child in &node.children {
        find_smallest_enclosing(child, from, to, best, count);
    }
}

/// Shrinks a character selection range `(from, to)` to an enclosed child syntax node.
#[must_use]
pub fn shrink_syntax_selection(
    tree: &syntax_tree,
    from: usize,
    to: usize,
) -> Option<(usize, usize)> {
    let mut count = 0;
    let node = find_exact_or_innermost(&tree.root, from, to, &mut count)?;

    if !node.children.is_empty() {
        // Pick the child enclosing the cursor, or the first non-empty child
        for child in &node.children {
            if child.start >= from && child.end <= to && (child.start > from || child.end < to) {
                return Some((child.start, child.end));
            }
        }
        let first = &node.children[0];
        if first.start > from || first.end < to {
            return Some((first.start, first.end));
        }
    }
    None
}

/// Selects the next sibling syntax node sharing the same parent.
#[must_use]
pub fn select_next_syntax_sibling(
    tree: &syntax_tree,
    from: usize,
    to: usize,
) -> Option<(usize, usize)> {
    let mut count = 0;
    let (_, siblings, idx) = find_node_with_siblings(&tree.root, from, to, &mut count)?;
    if idx + 1 < siblings.len() {
        let next = &siblings[idx + 1];
        Some((next.start, next.end))
    } else {
        None
    }
}

/// Selects the previous sibling syntax node sharing the same parent.
#[must_use]
pub fn select_prev_syntax_sibling(
    tree: &syntax_tree,
    from: usize,
    to: usize,
) -> Option<(usize, usize)> {
    let mut count = 0;
    let (_, siblings, idx) = find_node_with_siblings(&tree.root, from, to, &mut count)?;
    if idx > 0 {
        let prev = &siblings[idx - 1];
        Some((prev.start, prev.end))
    } else {
        None
    }
}

/// Finds a syntax node and its sibling slice under their common parent.
fn find_node_with_siblings<'a>(
    node: &'a ast_node,
    from: usize,
    to: usize,
    count: &mut usize,
) -> Option<(&'a ast_node, &'a [ast_node], usize)> {
    if *count >= MAX_STRUCTURAL_NODES {
        return None;
    }
    *count += 1;

    for (idx, child) in node.children.iter().enumerate() {
        if child.start == from && child.end == to {
            return Some((child, &node.children, idx));
        }
    }

    for (idx, child) in node.children.iter().enumerate() {
        if child.start <= from && child.end >= to {
            if let Some(res) = find_node_with_siblings(child, from, to, count) {
                return Some(res);
            }
            return Some((child, &node.children, idx));
        }
    }

    None
}

/// Finds the exact or innermost node enclosing `(from, to)`.
fn find_exact_or_innermost<'a>(
    node: &'a ast_node,
    from: usize,
    to: usize,
    count: &mut usize,
) -> Option<&'a ast_node> {
    if *count >= MAX_STRUCTURAL_NODES {
        return None;
    }
    *count += 1;

    if node.start > from || node.end < to {
        return None;
    }

    for child in &node.children {
        if child.start <= from && child.end >= to {
            if let Some(inner) = find_exact_or_innermost(child, from, to, count) {
                return Some(inner);
            }
        }
    }

    Some(node)
}

/// Finds an enclosing structural text object by semantic category.
///
/// Categories:
/// - `'f'`: function item (`function_item`)
/// - `'t'`: type name or definition (`type_name`, `type_builtin`)
/// - `'c'`: comments (`comment_line`, `comment_block`)
/// - `'a'`: argument or parameter (`argument_item`)
/// - `'T'`: test items
#[must_use]
pub fn find_syntax_textobject(
    tree: &syntax_tree,
    from: usize,
    to: usize,
    category: char,
    inside: bool,
) -> Option<(usize, usize)> {
    let mut candidate = None;
    let mut count = 0;
    find_category_node(&tree.root, from, to, category, &mut candidate, &mut count);

    let node = candidate?;
    if inside {
        match category {
            'f' => {
                // Return function body inside block if present
                for child in &node.children {
                    if child.kind == syntax_kind::block_item {
                        return Some((child.start.saturating_add(1), child.end.saturating_sub(1)));
                    }
                }
                Some((node.start, node.end))
            }
            'c' => {
                // Strip comment tokens ('//', '/*', '*/', '#')
                let start = if node.kind == syntax_kind::comment_line {
                    node.start.saturating_add(2)
                } else if node.kind == syntax_kind::comment_block {
                    node.start.saturating_add(2)
                } else {
                    node.start
                };
                let end = if node.kind == syntax_kind::comment_block {
                    node.end.saturating_sub(2)
                } else {
                    node.end
                };
                Some((start.min(end), end))
            }
            _ => Some((node.start, node.end)),
        }
    } else {
        Some((node.start, node.end))
    }
}

fn find_category_node<'a>(
    node: &'a ast_node,
    from: usize,
    to: usize,
    category: char,
    best: &mut Option<&'a ast_node>,
    count: &mut usize,
) {
    if *count >= MAX_STRUCTURAL_NODES {
        return;
    }
    *count += 1;

    let encloses = node.start <= from && node.end >= to;
    if !encloses {
        return;
    }

    let matches_cat = match category {
        'f' => node.kind == syntax_kind::function_item,
        't' => node.kind == syntax_kind::type_name || node.kind == syntax_kind::type_builtin,
        'c' => node.kind == syntax_kind::comment_line || node.kind == syntax_kind::comment_block,
        'a' => node.kind == syntax_kind::argument_item,
        _ => false,
    };

    if matches_cat {
        match *best {
            Some(current) => {
                if node.len() < current.len() {
                    *best = Some(node);
                }
            }
            None => {
                *best = Some(node);
            }
        }
    }

    for child in &node.children {
        find_category_node(child, from, to, category, best, count);
    }
}
