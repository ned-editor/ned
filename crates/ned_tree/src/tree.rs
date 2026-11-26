//! Concrete syntax tree (CST / AST) representations for language parsing.

use crate::spans::highlight_span;
use crate::tokens::syntax_kind;

/// Node within the concrete syntax tree representing a token or composite construct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ast_node {
    /// Semantic kind of this AST node.
    pub kind: syntax_kind,
    /// Document-level character start offset (inclusive).
    pub start: usize,
    /// Document-level character end offset (exclusive).
    pub end: usize,
    /// Nested child AST nodes.
    pub children: Vec<ast_node>,
}

impl ast_node {
    /// Creates a leaf AST node without children.
    #[must_use]
    pub const fn leaf(kind: syntax_kind, start: usize, end: usize) -> Self {
        Self {
            kind,
            start,
            end,
            children: Vec::new(),
        }
    }

    /// Creates a composite AST node containing child nodes.
    #[must_use]
    pub fn composite(kind: syntax_kind, start: usize, end: usize, children: Vec<Self>) -> Self {
        Self {
            kind,
            start,
            end,
            children,
        }
    }

    /// Appends a child node and updates the bounding end offset if needed.
    pub fn add_child(&mut self, child: Self) {
        if child.end > self.end {
            self.end = child.end;
        }
        self.children.push(child);
    }

    /// Returns `true` if this node is a leaf (contains no children).
    #[must_use]
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Character length covered by this node.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns `true` if this node spans zero characters.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Recursively flattens leaf tokens into highlight spans.
    pub fn collect_leaf_spans(&self, out: &mut Vec<highlight_span>) {
        if self.is_leaf() {
            if self.start < self.end && self.kind != syntax_kind::text {
                out.push(highlight_span::new(self.start, self.end, self.kind));
            }
        } else {
            for child in &self.children {
                child.collect_leaf_spans(out);
            }
        }
    }
}

/// Complete syntax tree of a parsed document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct syntax_tree {
    /// Root node of the document syntax tree.
    pub root: ast_node,
}

impl syntax_tree {
    /// Creates a new syntax tree from a root node.
    #[must_use]
    pub const fn new(root: ast_node) -> Self {
        Self { root }
    }

    /// Returns a reference to the root node.
    #[must_use]
    pub const fn root(&self) -> &ast_node {
        &self.root
    }

    /// Collects all leaf highlight spans across the entire document syntax tree.
    #[must_use]
    pub fn collect_spans(&self) -> Vec<highlight_span> {
        let mut spans = Vec::new();
        self.root.collect_leaf_spans(&mut spans);
        spans
    }

    /// Finds the innermost AST node enclosing the given character offset.
    #[must_use]
    pub fn node_at_offset(&self, offset: usize) -> Option<&ast_node> {
        Self::find_innermost(&self.root, offset)
    }

    fn find_innermost(node: &ast_node, offset: usize) -> Option<&ast_node> {
        if offset < node.start || offset >= node.end {
            return None;
        }

        for child in &node.children {
            if let Some(matching) = Self::find_innermost(child, offset) {
                return Some(matching);
            }
        }

        Some(node)
    }
}
