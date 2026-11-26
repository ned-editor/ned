//! Document-level AST parser converting full source documents into syntax trees.

use crate::grammars::grammar_def;
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

    let mut char_idx = 0;

    while char_idx < num_chars {
        let (byte_pos, ch) = chars[char_idx];
        let rem = &text[byte_pos..];

        // 1. Skip whitespace
        if ch.is_whitespace() {
            char_idx += 1;
            continue;
        }

        // 2. Multi-line block comments (e.g. `/* ... */`)
        if let Some((ref start_delim, ref end_delim)) = grammar.block_comment {
            if !start_delim.is_empty() && rem.starts_with(start_delim) {
                let comment_start = char_idx;
                char_idx += start_delim.chars().count();
                let rem_after = if char_idx < num_chars {
                    &text[chars[char_idx].0..]
                } else {
                    ""
                };

                if let Some(end_offset) = rem_after.find(end_delim) {
                    let end_char = char_idx
                        + rem_after[..end_offset + end_delim.len()]
                            .chars()
                            .count();
                    char_idx = end_char;
                    root.add_child(ast_node::leaf(
                        syntax_kind::comment_block,
                        comment_start,
                        end_char,
                    ));
                } else {
                    // Unclosed block comment spans to end of document
                    root.add_child(ast_node::leaf(
                        syntax_kind::comment_block,
                        comment_start,
                        num_chars,
                    ));
                    char_idx = num_chars;
                }
                continue;
            }
        }

        // 3. Single-line comments (e.g. `//`, `#`)
        if let Some(ref prefix) = grammar.line_comment {
            if !prefix.is_empty() && rem.starts_with(prefix) {
                let comment_start = char_idx;
                char_idx += prefix.chars().count();
                while char_idx < num_chars && chars[char_idx].1 != '\n' {
                    char_idx += 1;
                }
                root.add_child(ast_node::leaf(
                    syntax_kind::comment_line,
                    comment_start,
                    char_idx,
                ));
                continue;
            }
        }

        // 4. Triple-quoted multi-line strings (e.g. `"""`, `'''`)
        if grammar.triple_strings && (rem.starts_with("\"\"\"") || rem.starts_with("'''")) {
            let q = ch;
            let str_start = char_idx;
            char_idx += 3;
            let triple = format!("{q}{q}{q}");
            let rem_after = if char_idx < num_chars {
                &text[chars[char_idx].0..]
            } else {
                ""
            };

            if let Some(end_offset) = rem_after.find(&triple) {
                let end_char = char_idx + rem_after[..end_offset + 3].chars().count();
                char_idx = end_char;
                root.add_child(ast_node::leaf(
                    syntax_kind::string_literal,
                    str_start,
                    end_char,
                ));
            } else {
                root.add_child(ast_node::leaf(
                    syntax_kind::string_literal,
                    str_start,
                    num_chars,
                ));
                char_idx = num_chars;
            }
            continue;
        }

        // 5. Rust raw string literals (`r#"..."#` or `r"..."`)
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
                    &text[chars[char_idx].0..]
                } else {
                    ""
                };
                if let Some(pos) = rem_after.find(&close) {
                    let end_char = char_idx + rem_after[..pos + close.len()].chars().count();
                    char_idx = end_char;
                    root.add_child(ast_node::leaf(
                        syntax_kind::string_literal,
                        str_start,
                        end_char,
                    ));
                } else {
                    root.add_child(ast_node::leaf(
                        syntax_kind::string_literal,
                        str_start,
                        num_chars,
                    ));
                    char_idx = num_chars;
                }
                continue;
            }
        }

        // 6. Rust lifetime or label (e.g. `'static`, `'a`, `'_`)
        if ch == '\'' {
            let rem_chars = &chars[char_idx + 1..];
            if let Some((_, first_after)) = rem_chars.first() {
                if first_after.is_alphabetic() || *first_after == '_' {
                    let is_single_char = rem_chars.len() >= 2 && rem_chars[1].1 == '\'';
                    if !is_single_char {
                        let lt_start = char_idx;
                        char_idx += 1;
                        while char_idx < num_chars
                            && (chars[char_idx].1.is_alphanumeric() || chars[char_idx].1 == '_')
                        {
                            char_idx += 1;
                        }
                        root.add_child(ast_node::leaf(
                            syntax_kind::type_builtin,
                            lt_start,
                            char_idx,
                        ));
                        continue;
                    }
                }
            }
        }

        // 7. Standard string literals (`"..."`) and character literals (`'...'`)
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
                } else if quote == '\'' && curr == '\n' {
                    // Single-quote char literals do not cross line breaks
                    break;
                }
                char_idx += 1;
                if quote == '\'' && char_idx.saturating_sub(str_start) > 6 {
                    break;
                }
            }

            let kind = if quote == '\'' {
                syntax_kind::char_literal
            } else {
                syntax_kind::string_literal
            };
            root.add_child(ast_node::leaf(kind, str_start, char_idx));
            continue;
        }

        // 8. Attributes and macros (e.g. `#[derive(...)]`, `#! [...]`, `@decorator`)
        if ch == '#' && (rem.starts_with("#[") || rem.starts_with("#![")) {
            let attr_start = char_idx;
            char_idx += if rem.starts_with("#![") { 3 } else { 2 };
            let mut depth = 1usize;
            while char_idx < num_chars && depth > 0 {
                let curr = chars[char_idx].1;
                if curr == '[' {
                    depth += 1;
                } else if curr == ']' {
                    depth -= 1;
                }
                char_idx += 1;
            }
            root.add_child(ast_node::leaf(syntax_kind::attribute, attr_start, char_idx));
            continue;
        }

        if ch == '@' {
            let dec_start = char_idx;
            char_idx += 1;
            while char_idx < num_chars
                && (chars[char_idx].1.is_alphanumeric() || chars[char_idx].1 == '_')
            {
                char_idx += 1;
            }
            root.add_child(ast_node::leaf(syntax_kind::attribute, dec_start, char_idx));
            continue;
        }

        // 9. Numeric constants (integers, floats, hex, binary, suffixes)
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

        // 10. Identifiers, keywords, function items
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

            // Rust macro invocation (e.g. `println!`, `vec!`, `format!`)
            if char_idx < num_chars
                && chars[char_idx].1 == '!'
                && (char_idx + 1 >= num_chars || chars[char_idx + 1].1 != '=')
            {
                char_idx += 1;
                root.add_child(ast_node::leaf(syntax_kind::attribute, word_start, char_idx));
                continue;
            }

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

            // If function keyword `fn`, parse composite function item if followed by name
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
                if scan < num_chars
                    && (chars[scan].1.is_alphabetic() || chars[scan].1 == '_')
                {
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

        // 11. Operators
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

        // 12. Punctuation
        if matches!(ch, '(' | ')' | '{' | '}' | '[' | ']' | ';' | ',' | '.') {
            root.add_child(ast_node::leaf(
                syntax_kind::punctuation,
                char_idx,
                char_idx + 1,
            ));
            char_idx += 1;
            continue;
        }

        // Unclassified characters
        char_idx += 1;
    }

    syntax_tree::new(root)
}
