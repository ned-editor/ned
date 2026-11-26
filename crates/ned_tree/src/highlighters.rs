//! Incremental syntax highlighter engine coordinating rope buffers and state caches.

use ned_rope::rope;

use crate::grammars::grammar_def;
use crate::lexers::lex_line;
use crate::spans::highlight_span;
use crate::states::state_cache;

/// High-level incremental syntax highlighter for a text document.
#[derive(Debug, Clone, Default)]
pub struct syntax_highlighter {
    grammar: Option<grammar_def>,
    cache: state_cache,
}

impl syntax_highlighter {
    /// Creates a new highlighter with optional grammar.
    #[must_use]
    pub fn new(grammar: Option<grammar_def>) -> Self {
        Self {
            grammar,
            cache: state_cache::new(),
        }
    }

    /// Sets or updates the active grammar definition.
    pub fn set_grammar(&mut self, grammar: Option<grammar_def>) {
        self.grammar = grammar;
        self.cache.clear();
    }

    /// Returns a reference to the active grammar, if configured.
    #[must_use]
    pub fn grammar(&self) -> Option<&grammar_def> {
        self.grammar.as_ref()
    }

    /// Invalidates cached states starting from `line_idx` following an edit.
    pub fn invalidate_from(&mut self, line_idx: usize) {
        self.cache.invalidate_from(line_idx);
    }

    /// Highlights a single line of text from the buffer, filling `out_spans`.
    ///
    /// Automatically synchronizes preceding line states if not already cached.
    pub fn highlight_line(
        &mut self,
        buffer: &rope,
        line_idx: usize,
        out_spans: &mut Vec<highlight_span>,
    ) {
        let grammar = match self.grammar.as_ref() {
            Some(g) => g,
            None => return,
        };

        // Ensure states up to line_idx are populated
        Self::sync_states_up_to(grammar, &mut self.cache, buffer, line_idx);

        let line_text = buffer.line(line_idx).unwrap_or_default();
        let initial_state = self.cache.get(line_idx);
        let end_state = lex_line(&line_text, grammar, initial_state, out_spans);
        self.cache.set(line_idx + 1, end_state);
    }

    /// Highlights a range of visible lines for viewport rendering.
    #[must_use]
    pub fn highlight_viewport(
        &mut self,
        buffer: &rope,
        start_line: usize,
        end_line: usize,
    ) -> Vec<(usize, Vec<highlight_span>)> {
        let mut results = Vec::with_capacity(end_line.saturating_sub(start_line) + 1);
        let mut spans = Vec::new();

        for l in start_line..=end_line {
            spans.clear();
            self.highlight_line(buffer, l, &mut spans);
            results.push((l, spans.clone()));
        }

        results
    }

    /// Synchronizes line states from current cache boundary up to `target_line`.
    fn sync_states_up_to(
        grammar: &grammar_def,
        cache: &mut state_cache,
        buffer: &rope,
        target_line: usize,
    ) {
        let start = cache.len();
        if start > target_line {
            return;
        }

        let mut scratch_spans = Vec::new();
        for l in start..=target_line {
            let line_text = buffer.line(l).unwrap_or_default();
            let initial = cache.get(l);
            scratch_spans.clear();
            let end_state = lex_line(&line_text, grammar, initial, &mut scratch_spans);
            cache.set(l + 1, end_state);
        }
    }
}
