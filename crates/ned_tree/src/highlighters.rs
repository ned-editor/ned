//! AST syntax highlighter engine coordinating rope buffers and document-level syntax trees.

use ned_rope::rope;

use crate::grammars::grammar_def;
use crate::parsers::parse_document_ast;
use crate::spans::highlight_span;
use crate::tree::syntax_tree;

/// High-level AST syntax highlighter for a text document.
#[derive(Debug, Clone, Default)]
pub struct syntax_highlighter {
    /// Active grammar definition.
    grammar: Option<grammar_def>,
    /// Cached concrete syntax tree for the document.
    tree: Option<syntax_tree>,
    /// Line-indexed highlight spans projected from the AST.
    line_spans: Vec<Vec<highlight_span>>,
    /// Dirty flag indicating document edits require reparsing.
    dirty: bool,
}

impl syntax_highlighter {
    /// Creates a new highlighter with optional grammar.
    #[must_use]
    pub fn new(grammar: Option<grammar_def>) -> Self {
        Self {
            grammar,
            tree: None,
            line_spans: Vec::new(),
            dirty: true,
        }
    }

    /// Sets or updates the active grammar definition.
    pub fn set_grammar(&mut self, grammar: Option<grammar_def>) {
        self.grammar = grammar;
        self.tree = None;
        self.line_spans.clear();
        self.dirty = true;
    }

    /// Returns a reference to the active grammar, if configured.
    #[must_use]
    pub fn grammar(&self) -> Option<&grammar_def> {
        self.grammar.as_ref()
    }

    /// Returns a reference to the parsed AST syntax tree, if available.
    #[must_use]
    pub fn syntax_tree(&self) -> Option<&syntax_tree> {
        self.tree.as_ref()
    }

    /// Invalidates cached AST and line spans starting from `_line_idx`.
    pub fn invalidate_from(&mut self, _line_idx: usize) {
        self.dirty = true;
    }

    /// Highlights a single line of text from the buffer, filling `out_spans`.
    pub fn highlight_line(
        &mut self,
        buffer: &rope,
        line_idx: usize,
        out_spans: &mut Vec<highlight_span>,
    ) {
        if self.dirty {
            self.reparse(buffer);
        }

        if let Some(spans) = self.line_spans.get(line_idx) {
            out_spans.extend_from_slice(spans);
        }
    }

    /// Highlights a range of visible lines for viewport rendering.
    #[must_use]
    pub fn highlight_viewport(
        &mut self,
        buffer: &rope,
        start_line: usize,
        end_line: usize,
    ) -> Vec<(usize, Vec<highlight_span>)> {
        if self.dirty {
            self.reparse(buffer);
        }

        let mut results = Vec::with_capacity(end_line.saturating_sub(start_line) + 1);
        for l in start_line..=end_line {
            let spans = self.line_spans.get(l).cloned().unwrap_or_default();
            results.push((l, spans));
        }

        results
    }

    /// Reparses the entire document text into an AST and projects spans across lines.
    fn reparse(&mut self, buffer: &rope) {
        self.dirty = false;
        self.line_spans.clear();

        let Some(grammar) = self.grammar.as_ref() else {
            self.tree = None;
            return;
        };

        let text = buffer.to_string();
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        let num_chars = chars.len();

        // 1. Parse whole document into AST
        let tree = parse_document_ast(&text, grammar);
        let leaf_spans = tree.collect_spans();
        self.tree = Some(tree);

        // 2. Precompute line start offsets and line lengths in char units
        let mut line_starts: Vec<usize> = Vec::new();
        line_starts.push(0);
        for (i, (_, ch)) in chars.iter().enumerate() {
            if *ch == '\n' {
                line_starts.push(i + 1);
            }
        }

        let num_lines = line_starts.len();
        let mut line_bounds: Vec<(usize, usize)> = Vec::with_capacity(num_lines);
        for l in 0..num_lines {
            let s = line_starts[l];
            let e = if l + 1 < num_lines {
                let mut end = line_starts[l + 1] - 1;
                if end > s && chars[end - 1].1 == '\r' {
                    end -= 1;
                }
                end
            } else {
                num_chars
            };
            line_bounds.push((s, e));
        }

        self.line_spans = vec![Vec::new(); num_lines];

        // 3. Project global leaf spans onto lines
        for span in leaf_spans {
            if span.start >= span.end {
                continue;
            }

            let l1 = match line_starts.binary_search(&span.start) {
                Ok(i) => i,
                Err(i) => i.saturating_sub(1),
            };
            let l2 = match line_starts.binary_search(&span.end.saturating_sub(1)) {
                Ok(i) => i,
                Err(i) => i.saturating_sub(1),
            };

            if l1 == l2 {
                let (line_s, line_e) = line_bounds[l1];
                let start_in_line = span.start.saturating_sub(line_s);
                let end_in_line =
                    (span.end.saturating_sub(line_s)).min(line_e.saturating_sub(line_s));
                if start_in_line < end_in_line {
                    self.line_spans[l1].push(highlight_span::new(
                        start_in_line,
                        end_in_line,
                        span.kind,
                    ));
                }
            } else {
                // First line
                let (line_s, line_e) = line_bounds[l1];
                let start_in_line = span.start.saturating_sub(line_s);
                let end_in_line = line_e.saturating_sub(line_s);
                if start_in_line < end_in_line {
                    self.line_spans[l1].push(highlight_span::new(
                        start_in_line,
                        end_in_line,
                        span.kind,
                    ));
                }

                // Intermediate lines
                for l in (l1 + 1)..l2 {
                    let (ls, le) = line_bounds[l];
                    let line_len = le.saturating_sub(ls);
                    if line_len > 0 {
                        self.line_spans[l].push(highlight_span::new(0, line_len, span.kind));
                    }
                }

                // Last line
                let (line_s, line_e) = line_bounds[l2];
                let end_in_line =
                    (span.end.saturating_sub(line_s)).min(line_e.saturating_sub(line_s));
                if end_in_line > 0 {
                    self.line_spans[l2].push(highlight_span::new(0, end_in_line, span.kind));
                }
            }
        }
    }
}
