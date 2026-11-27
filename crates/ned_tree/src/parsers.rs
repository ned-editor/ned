//! Language-agnostic document-level AST parser driven by declarative grammar rules.

use crate::grammars::{char_class, grammar_def, grammar_rule};
use crate::tokens::syntax_kind;
use crate::tree::{ast_node, syntax_tree};

/// Parses an entire source document against a language grammar into an AST [`syntax_tree`].
#[must_use]
pub fn parse_document_ast(text: &str, grammar: &grammar_def) -> syntax_tree {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let num_chars = chars.len();

    let mut root = ast_node::composite(syntax_kind::root, 0, num_chars, Vec::new());
    if num_chars == 0 {
        return syntax_tree::new(root);
    }

    let rules = grammar.effective_rules();
    let mut char_idx = 0;

    while char_idx < num_chars {
        let (byte_pos, ch) = chars[char_idx];

        // 1. Skip whitespace
        if ch.is_whitespace() {
            char_idx += 1;
            continue;
        }

        // 2. Evaluate declarative grammar rules in priority order
        let mut matched_rule = false;
        for rule in &rules {
            if let Some((node, next_idx)) = try_match_rule(rule, text, &chars, char_idx) {
                root.add_child(node);
                char_idx = next_idx;
                matched_rule = true;
                break;
            }
        }
        if matched_rule {
            continue;
        }

        let _ = byte_pos;

        // 3. Universal numeric constants (integers, floats, hex, binary)
        if ch.is_ascii_digit() {
            let num_start = char_idx;
            char_idx += 1;
            while char_idx < num_chars {
                let curr = chars[char_idx].1;
                if curr.is_ascii_alphanumeric() || curr == '.' || curr == '_' {
                    char_idx += 1;
                } else {
                    break;
                }
            }
            root.add_child(ast_node::leaf(
                syntax_kind::number_literal,
                num_start,
                char_idx,
            ));
            continue;
        }

        // 4. Universal identifiers, keywords, and composite function items
        if ch.is_alphabetic() || ch == '_' {
            let word_start = char_idx;
            while char_idx < num_chars
                && (chars[char_idx].1.is_alphanumeric() || chars[char_idx].1 == '_')
            {
                char_idx += 1;
            }

            let word_byte_start = chars[word_start].0;
            let word_byte_end = if char_idx < num_chars {
                chars[char_idx].0
            } else {
                text.len()
            };
            let word = &text[word_byte_start..word_byte_end];

            // Lookahead for function call: non-whitespace followed by '('
            let mut lookahead = char_idx;
            while lookahead < num_chars && chars[lookahead].1.is_whitespace() {
                lookahead += 1;
            }
            let is_call = lookahead < num_chars && chars[lookahead].1 == '(';

            let kind = if grammar.is_function_keyword(word) {
                syntax_kind::keyword_function
            } else if grammar.is_control_keyword(word) {
                syntax_kind::keyword_control
            } else if grammar.is_keyword(word) {
                syntax_kind::keyword
            } else if grammar.is_builtin_type(word) {
                syntax_kind::type_builtin
            } else if ch.is_uppercase() {
                syntax_kind::type_name
            } else if is_call {
                syntax_kind::function_call
            } else {
                syntax_kind::variable_name
            };

            if kind == syntax_kind::keyword_function {
                let mut fn_children = vec![ast_node::leaf(
                    syntax_kind::keyword_function,
                    word_start,
                    char_idx,
                )];
                let mut scan = char_idx;
                while scan < num_chars && chars[scan].1.is_whitespace() {
                    scan += 1;
                }
                if scan < num_chars && (chars[scan].1.is_alphabetic() || chars[scan].1 == '_') {
                    let fn_name_start = scan;
                    while scan < num_chars
                        && (chars[scan].1.is_alphanumeric() || chars[scan].1 == '_')
                    {
                        scan += 1;
                    }
                    fn_children.push(ast_node::leaf(
                        syntax_kind::function_name,
                        fn_name_start,
                        scan,
                    ));
                    char_idx = scan;
                }
                root.add_child(ast_node::composite(
                    syntax_kind::function_item,
                    word_start,
                    char_idx,
                    fn_children,
                ));
                continue;
            }

            root.add_child(ast_node::leaf(kind, word_start, char_idx));
            continue;
        }

        // 5. Universal operators
        if matches!(
            ch,
            '+' | '-' | '*' | '/' | '%' | '=' | '<' | '>' | '!' | '&' | '|' | '^' | '~' | '?' | ':'
        ) {
            let op_start = char_idx;
            char_idx += 1;
            while char_idx < num_chars
                && matches!(
                    chars[char_idx].1,
                    '+' | '-'
                        | '*'
                        | '/'
                        | '%'
                        | '='
                        | '<'
                        | '>'
                        | '!'
                        | '&'
                        | '|'
                        | '^'
                        | '~'
                        | '?'
                        | ':'
                )
            {
                char_idx += 1;
            }
            root.add_child(ast_node::leaf(syntax_kind::operator, op_start, char_idx));
            continue;
        }

        // 6. Universal punctuation
        if matches!(ch, '(' | ')' | '{' | '}' | '[' | ']' | ';' | ',' | '.') {
            root.add_child(ast_node::leaf(
                syntax_kind::punctuation,
                char_idx,
                char_idx + 1,
            ));
            char_idx += 1;
            continue;
        }

        // Unclassified character
        char_idx += 1;
    }

    syntax_tree::new(root)
}

/// Evaluates a single declarative [`grammar_rule`] at `start_idx`.
fn try_match_rule(
    rule: &grammar_rule,
    text: &str,
    chars: &[(usize, char)],
    start_idx: usize,
) -> Option<(ast_node, usize)> {
    let num_chars = chars.len();
    let rem = &text[chars[start_idx].0..];

    match rule {
        grammar_rule::span {
            begin,
            end,
            kind,
            escape,
            multiline,
            nested,
        } => {
            if begin.is_empty() || !rem.starts_with(begin) {
                return None;
            }

            let begin_chars = begin.chars().count();
            let end_chars = end.chars().count();
            let mut idx = start_idx + begin_chars;
            let mut depth = 1usize;

            while idx < num_chars {
                let curr = chars[idx].1;
                if !*multiline && curr == '\n' {
                    break;
                }
                if let Some(esc) = escape {
                    if curr == *esc {
                        idx = (idx + 2).min(num_chars);
                        continue;
                    }
                }

                let curr_rem = &text[chars[idx].0..];
                if *nested && curr_rem.starts_with(begin) {
                    depth += 1;
                    idx += begin_chars;
                    continue;
                }
                if !end.is_empty() && curr_rem.starts_with(end) {
                    depth -= 1;
                    if end != "\n" {
                        idx += end_chars;
                    }
                    if depth == 0 {
                        break;
                    }
                    continue;
                }

                idx += 1;
            }

            Some((ast_node::leaf(*kind, start_idx, idx), idx))
        }

        grammar_rule::paired_span {
            prefix,
            repeat_char,
            min_repeats,
            open,
            close,
            kind,
        } => {
            if !prefix.is_empty() && !rem.starts_with(prefix) {
                return None;
            }

            let mut idx = start_idx + prefix.chars().count();
            let mut repeats = 0usize;
            while idx < num_chars && chars[idx].1 == *repeat_char {
                repeats += 1;
                idx += 1;
            }

            if repeats < *min_repeats {
                return None;
            }

            if !open.is_empty() {
                if idx >= num_chars || !text[chars[idx].0..].starts_with(open) {
                    return None;
                }
                idx += open.chars().count();
            }

            let mut close_delim = String::with_capacity(close.len() + repeats);
            close_delim.push_str(close);
            for _ in 0..repeats {
                close_delim.push(*repeat_char);
            }

            let rem_after = if idx < num_chars {
                &text[chars[idx].0..]
            } else {
                ""
            };

            let end_idx = if let Some(pos) = rem_after.find(&close_delim) {
                idx + rem_after[..pos + close_delim.len()].chars().count()
            } else {
                num_chars
            };

            Some((ast_node::leaf(*kind, start_idx, end_idx), end_idx))
        }

        grammar_rule::pattern {
            prefix,
            body,
            suffix,
            not_followed_by,
            kind,
        } => {
            let mut idx = start_idx;
            if let Some(ref p) = prefix {
                if !rem.starts_with(p) {
                    return None;
                }
                idx += p.chars().count();
            }

            if idx >= num_chars {
                return None;
            }

            let body_start = idx;
            match body {
                char_class::ident => {
                    let first = chars[idx].1;
                    if !first.is_alphabetic() && first != '_' {
                        return None;
                    }
                    idx += 1;
                    while idx < num_chars && (chars[idx].1.is_alphanumeric() || chars[idx].1 == '_')
                    {
                        idx += 1;
                    }
                }
                char_class::digits => {
                    if !chars[idx].1.is_ascii_digit() {
                        return None;
                    }
                    idx += 1;
                    while idx < num_chars
                        && (chars[idx].1.is_ascii_alphanumeric()
                            || chars[idx].1 == '.'
                            || chars[idx].1 == '_')
                    {
                        idx += 1;
                    }
                }
                char_class::non_whitespace => {
                    while idx < num_chars && !chars[idx].1.is_whitespace() {
                        idx += 1;
                    }
                }
            }

            if idx == body_start {
                return None;
            }

            if let Some(ref s) = suffix {
                if idx >= num_chars || !text[chars[idx].0..].starts_with(s) {
                    return None;
                }
                idx += s.chars().count();
            }

            if let Some(forbidden) = not_followed_by {
                if idx < num_chars && chars[idx].1 == *forbidden {
                    return None;
                }
            }

            Some((ast_node::leaf(*kind, start_idx, idx), idx))
        }
    }
}
