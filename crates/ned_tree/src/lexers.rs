//! Zero-copy, high-performance lexical analysis engine for syntax highlighting.

use crate::grammars::grammar_def;
use crate::spans::highlight_span;
use crate::states::line_state;
use crate::tokens::syntax_kind;

/// Tokenizes a single line of text against a language grammar and incoming multiline state.
///
/// Appends resulting highlight spans to `out_spans` and returns the state at the end of the line.
#[must_use]
pub fn lex_line(
    line: &str,
    grammar: &grammar_def,
    initial_state: line_state,
    out_spans: &mut Vec<highlight_span>,
) -> line_state {
    if line.is_empty() {
        return initial_state;
    }

    let mut char_idx = 0;
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let num_chars = chars.len();

    // 1. Resume from multiline state if active
    let mut state = initial_state;
    match state {
        line_state::in_block_comment(depth) => {
            if let Some((_, ref end_delim)) = grammar.block_comment {
                if let Some(pos) = line.find(end_delim) {
                    let end_char = line[..pos + end_delim.len()].chars().count();
                    out_spans.push(highlight_span::new(0, end_char, syntax_kind::comment_block));
                    char_idx = end_char;
                    state = if depth > 1 {
                        line_state::in_block_comment(depth - 1)
                    } else {
                        line_state::normal
                    };
                } else {
                    out_spans.push(highlight_span::new(0, num_chars, syntax_kind::comment_block));
                    return state;
                }
            }
        }
        line_state::in_triple_string(q) => {
            let triple = format!("{q}{q}{q}");
            if let Some(pos) = line.find(&triple) {
                let end_char = line[..pos + triple.len()].chars().count();
                out_spans.push(highlight_span::new(0, end_char, syntax_kind::string_literal));
                char_idx = end_char;
                state = line_state::normal;
            } else {
                out_spans.push(highlight_span::new(0, num_chars, syntax_kind::string_literal));
                return state;
            }
        }
        line_state::in_raw_string(hashes) => {
            let mut close = String::with_capacity(hashes as usize + 1);
            close.push('"');
            for _ in 0..hashes {
                close.push('#');
            }
            if let Some(pos) = line.find(&close) {
                let end_char = line[..pos + close.len()].chars().count();
                out_spans.push(highlight_span::new(0, end_char, syntax_kind::string_literal));
                char_idx = end_char;
                state = line_state::normal;
            } else {
                out_spans.push(highlight_span::new(0, num_chars, syntax_kind::string_literal));
                return state;
            }
        }
        line_state::normal => {}
    }

    // 2. Normal lexical tokenization
    while char_idx < num_chars {
        let (byte_pos, ch) = chars[char_idx];
        let rem = &line[byte_pos..];

        // Skip whitespace
        if ch.is_whitespace() {
            char_idx += 1;
            continue;
        }

        // Single-line comments (e.g. `//`, `#`)
        if let Some(ref prefix) = grammar.line_comment {
            if !prefix.is_empty() && rem.starts_with(prefix) {
                out_spans.push(highlight_span::new(char_idx, num_chars, syntax_kind::comment_line));
                break;
            }
        }

        // Multi-line block comments (e.g. `/* ... */`)
        if let Some((ref start_delim, ref end_delim)) = grammar.block_comment {
            if !start_delim.is_empty() && rem.starts_with(start_delim) {
                let comment_start = char_idx;
                char_idx += start_delim.chars().count();
                let rem_after = &line[chars[char_idx.min(num_chars - 1)].0..];

                if let Some(end_offset) = rem_after.find(end_delim) {
                    let end_char = comment_start + start_delim.chars().count() + rem_after[..end_offset + end_delim.len()].chars().count();
                    out_spans.push(highlight_span::new(comment_start, end_char, syntax_kind::comment_block));
                    char_idx = end_char;
                    continue;
                }
                out_spans.push(highlight_span::new(comment_start, num_chars, syntax_kind::comment_block));
                return line_state::in_block_comment(1);
            }
        }

        // Triple-quoted multi-line strings (e.g. `"""`, `'''`)
        if grammar.triple_strings && (rem.starts_with("\"\"\"") || rem.starts_with("'''")) {
            let q = ch;
            let str_start = char_idx;
            char_idx += 3;
            let triple = format!("{q}{q}{q}");
            let rem_after = &line[chars[char_idx.min(num_chars - 1)].0..];

            if let Some(end_offset) = rem_after.find(&triple) {
                let end_char = str_start + 3 + rem_after[..end_offset + 3].chars().count();
                out_spans.push(highlight_span::new(str_start, end_char, syntax_kind::string_literal));
                char_idx = end_char;
                continue;
            }
            out_spans.push(highlight_span::new(str_start, num_chars, syntax_kind::string_literal));
            return line_state::in_triple_string(q);
        }

        // Rust raw string literals (`r#"..."#` or `r"..."`)
        if grammar.raw_strings && (rem.starts_with("r#") || rem.starts_with("r\"")) {
            let str_start = char_idx;
            char_idx += 1; // skip 'r'
            let mut hashes: u8 = 0;
            while char_idx < num_chars && chars[char_idx].1 == '#' {
                hashes += 1;
                char_idx += 1;
            }
            if char_idx < num_chars && chars[char_idx].1 == '"' {
                char_idx += 1;
                let mut close = String::with_capacity(hashes as usize + 1);
                close.push('"');
                for _ in 0..hashes {
                    close.push('#');
                }
                let rem_after = if char_idx < num_chars {
                    &line[chars[char_idx].0..]
                } else {
                    ""
                };
                if let Some(pos) = rem_after.find(&close) {
                    let end_char = char_idx + rem_after[..pos + close.len()].chars().count();
                    out_spans.push(highlight_span::new(str_start, end_char, syntax_kind::string_literal));
                    char_idx = end_char;
                    continue;
                }
                out_spans.push(highlight_span::new(str_start, num_chars, syntax_kind::string_literal));
                return line_state::in_raw_string(hashes);
            }
        }

        // Standard string literals (`"` or `'`)
        if ch == '"' || ch == '\'' {
            let str_start = char_idx;
            let quote = ch;
            char_idx += 1;
            let mut escaped = false;

            while char_idx < num_chars {
                let curr = chars[char_idx].1;
                if escaped {
                    escaped = false;
                } else if curr == '\\' {
                    escaped = true;
                } else if curr == quote {
                    char_idx += 1;
                    break;
                }
                char_idx += 1;
            }

            let kind = if quote == '\'' && (char_idx - str_start) <= 4 {
                syntax_kind::char_literal
            } else {
                syntax_kind::string_literal
            };
            out_spans.push(highlight_span::new(str_start, char_idx, kind));
            continue;
        }

        // Numbers (integers, floats, hex, binary)
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
            out_spans.push(highlight_span::new(num_start, char_idx, syntax_kind::number_literal));
            continue;
        }

        // Identifiers and keywords
        if ch.is_alphabetic() || ch == '_' {
            let word_start = char_idx;
            while char_idx < num_chars {
                let curr = chars[char_idx].1;
                if curr.is_alphanumeric() || curr == '_' {
                    char_idx += 1;
                } else {
                    break;
                }
            }

            let word_byte_start = chars[word_start].0;
            let word_byte_end = if char_idx < num_chars {
                chars[char_idx].0
            } else {
                line.len()
            };
            let word = &line[word_byte_start..word_byte_end];

            // Look ahead for function call: non-whitespace followed by '('
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

            out_spans.push(highlight_span::new(word_start, char_idx, kind));
            continue;
        }

        // Operators
        if matches!(ch, '+' | '-' | '*' | '/' | '%' | '=' | '<' | '>' | '!' | '&' | '|' | '^' | '~' | '?' | ':') {
            let op_start = char_idx;
            char_idx += 1;
            while char_idx < num_chars && matches!(chars[char_idx].1, '+' | '-' | '*' | '/' | '%' | '=' | '<' | '>' | '!' | '&' | '|' | '^' | '~' | '?' | ':') {
                char_idx += 1;
            }
            out_spans.push(highlight_span::new(op_start, char_idx, syntax_kind::operator));
            continue;
        }

        // Punctuation
        if matches!(ch, '(' | ')' | '{' | '}' | '[' | ']' | ';' | ',' | '.') {
            out_spans.push(highlight_span::new(char_idx, char_idx + 1, syntax_kind::punctuation));
            char_idx += 1;
            continue;
        }

        // Unclassified characters
        char_idx += 1;
    }

    state
}
