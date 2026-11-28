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
    let nodes = parse_nodes(text, &chars, grammar, &rules, 0, num_chars, 0);
    for n in nodes {
        root.add_child(n);
    }

    syntax_tree::new(root)
}

/// Finds the index of the matching closing delimiter matching `open` at `start_idx`.
fn find_matching_delimiter(
    chars: &[(usize, char)],
    start_idx: usize,
    open: char,
    close: char,
) -> Option<usize> {
    let mut depth = 1usize;
    let mut idx = start_idx + 1;
    let num_chars = chars.len();

    while idx < num_chars {
        let ch = chars[idx].1;
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(idx);
            }
        }
        idx += 1;
    }
    None
}

/// Recursively parses AST nodes within a character range `[start, end)`.
///
/// This is a compact recursive-descent pass; extracting helpers would thread `text`, `chars`,
/// `grammar`, `rules` and `depth` through many signatures for little clarity gain, so the
/// line-limit lint is allowed here.
#[allow(clippy::too_many_lines)]
fn parse_nodes(
    text: &str,
    chars: &[(usize, char)],
    grammar: &grammar_def,
    rules: &[grammar_rule],
    start: usize,
    end: usize,
    depth: usize,
) -> Vec<ast_node> {
    let mut out = Vec::new();
    if depth > 32 {
        return out;
    }

    let mut char_idx = start;

    while char_idx < end {
        let ch = chars[char_idx].1;

        // 1. Skip whitespace
        if ch.is_whitespace() {
            char_idx += 1;
            continue;
        }

        // 2. Evaluate declarative grammar rules in priority order
        let mut matched_rule = false;
        for rule in rules {
            if let Some((node, next_idx)) = try_match_rule(rule, text, chars, char_idx) {
                out.push(node);
                char_idx = next_idx.min(end);
                matched_rule = true;
                break;
            }
        }
        if matched_rule {
            continue;
        }

        // 3. Blocks: `{ ... }`
        if ch == '{' {
            if let Some(close_idx) = find_matching_delimiter(chars, char_idx, '{', '}') {
                let block_start = char_idx;
                let block_end = close_idx + 1;
                let inner = parse_nodes(
                    text,
                    chars,
                    grammar,
                    rules,
                    block_start + 1,
                    close_idx,
                    depth + 1,
                );
                out.push(ast_node::composite(
                    syntax_kind::block_item,
                    block_start,
                    block_end,
                    inner,
                ));
                char_idx = block_end;
                continue;
            }
        }

        // 4. Universal numeric constants (integers, floats, hex, binary)
        if ch.is_ascii_digit() {
            let num_start = char_idx;
            char_idx += 1;
            while char_idx < end {
                let curr = chars[char_idx].1;
                if curr.is_ascii_alphanumeric() || curr == '.' || curr == '_' {
                    char_idx += 1;
                } else {
                    break;
                }
            }
            out.push(ast_node::leaf(
                syntax_kind::number_literal,
                num_start,
                char_idx,
            ));
            continue;
        }

        // 5. Universal identifiers, keywords, calls, and function declarations
        if ch.is_alphabetic() || ch == '_' {
            let word_start = char_idx;
            while char_idx < end
                && (chars[char_idx].1.is_alphanumeric() || chars[char_idx].1 == '_')
            {
                char_idx += 1;
            }

            let word_byte_start = chars[word_start].0;
            let word_byte_end = if char_idx < chars.len() {
                chars[char_idx].0
            } else {
                text.len()
            };
            let word = &text[word_byte_start..word_byte_end];

            // Lookahead for function call: non-whitespace followed by '('
            let mut lookahead = char_idx;
            while lookahead < end && chars[lookahead].1.is_whitespace() {
                lookahead += 1;
            }
            let is_call = lookahead < end && chars[lookahead].1 == '(';

            let is_fn_keyword = grammar.is_function_keyword(word);

            // Function declaration: `fn name(...) { ... }`
            if is_fn_keyword {
                let mut fn_children = vec![ast_node::leaf(
                    syntax_kind::keyword_function,
                    word_start,
                    char_idx,
                )];
                let mut scan = char_idx;
                while scan < end && chars[scan].1.is_whitespace() {
                    scan += 1;
                }
                if scan < end && (chars[scan].1.is_alphabetic() || chars[scan].1 == '_') {
                    let fn_name_start = scan;
                    while scan < end && (chars[scan].1.is_alphanumeric() || chars[scan].1 == '_') {
                        scan += 1;
                    }
                    fn_children.push(ast_node::leaf(
                        syntax_kind::function_name,
                        fn_name_start,
                        scan,
                    ));
                    scan_params_and_body(
                        text,
                        chars,
                        grammar,
                        rules,
                        scan,
                        end,
                        depth,
                        &mut fn_children,
                    );
                }
                let fn_end = fn_children.last().map_or(char_idx, |n| n.end);
                out.push(ast_node::composite(
                    syntax_kind::function_item,
                    word_start,
                    fn_end,
                    fn_children,
                ));
                char_idx = fn_end;
                continue;
            }

            // Function call: `foo(arg1, arg2)`
            if is_call {
                if let Some(close_paren) = find_matching_delimiter(chars, lookahead, '(', ')') {
                    let mut call_children = vec![ast_node::leaf(
                        syntax_kind::function_call,
                        word_start,
                        char_idx,
                    )];
                    let args = parse_arguments(
                        text,
                        chars,
                        grammar,
                        rules,
                        lookahead + 1,
                        close_paren,
                        depth + 1,
                    );
                    call_children.extend(args);

                    let call_end = close_paren + 1;
                    out.push(ast_node::composite(
                        syntax_kind::call_item,
                        word_start,
                        call_end,
                        call_children,
                    ));
                    char_idx = call_end;
                    continue;
                }
            }

            let kind = if grammar.is_control_keyword(word) {
                syntax_kind::keyword_control
            } else if grammar.is_keyword(word) {
                syntax_kind::keyword
            } else if grammar.is_builtin_type(word) {
                syntax_kind::type_builtin
            } else if ch.is_uppercase() {
                syntax_kind::type_name
            } else {
                syntax_kind::variable_name
            };

            out.push(ast_node::leaf(kind, word_start, char_idx));
            continue;
        }

        // 6. Universal operators
        if matches!(
            ch,
            '+' | '-' | '*' | '/' | '%' | '=' | '<' | '>' | '!' | '&' | '|' | '^' | '~' | '?' | ':'
        ) {
            let op_start = char_idx;
            char_idx += 1;
            while char_idx < end
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
            out.push(ast_node::leaf(syntax_kind::operator, op_start, char_idx));
            continue;
        }

        // 7. Universal punctuation
        if matches!(ch, '(' | ')' | '{' | '}' | '[' | ']' | ';' | ',' | '.') {
            out.push(ast_node::leaf(
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

    out
}

/// Parses comma-separated arguments inside `(` and `)`.
fn parse_arguments(
    text: &str,
    chars: &[(usize, char)],
    grammar: &grammar_def,
    rules: &[grammar_rule],
    start: usize,
    end: usize,
    depth: usize,
) -> Vec<ast_node> {
    let mut args = Vec::new();
    let mut arg_start = start;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;

    for i in start..end {
        let ch = chars[i].1;
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ',' if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                if let Some(arg_node) =
                    create_argument_node(text, chars, grammar, rules, arg_start, i, depth)
                {
                    args.push(arg_node);
                }
                arg_start = i + 1;
            }
            _ => {}
        }
    }

    if let Some(arg_node) = create_argument_node(text, chars, grammar, rules, arg_start, end, depth)
    {
        args.push(arg_node);
    }

    args
}

/// Creates an `argument_item` composite node spanning non-whitespace bounds.
fn create_argument_node(
    text: &str,
    chars: &[(usize, char)],
    grammar: &grammar_def,
    rules: &[grammar_rule],
    mut start: usize,
    mut end: usize,
    depth: usize,
) -> Option<ast_node> {
    while start < end && chars[start].1.is_whitespace() {
        start += 1;
    }
    while end > start && chars[end - 1].1.is_whitespace() {
        end -= 1;
    }

    if start >= end {
        return None;
    }

    let children = parse_nodes(text, chars, grammar, rules, start, end, depth);
    Some(ast_node::composite(
        syntax_kind::argument_item,
        start,
        end,
        children,
    ))
}

/// Scans parameters `(...)` and body block `{ ... }` for a function declaration.
///
/// The grammar context (`text`, `chars`, `grammar`, `rules`, `depth`) is threaded through many
/// parser helpers; grouping it into a struct would add indirection without a clear win, so the
/// argument-count lint is allowed here.
#[allow(clippy::too_many_arguments)]
fn scan_params_and_body(
    text: &str,
    chars: &[(usize, char)],
    grammar: &grammar_def,
    rules: &[grammar_rule],
    mut scan: usize,
    end: usize,
    depth: usize,
    children: &mut Vec<ast_node>,
) {
    while scan < end && chars[scan].1.is_whitespace() {
        scan += 1;
    }

    // Parameters `(...)`
    if scan < end && chars[scan].1 == '(' {
        if let Some(close_paren) = find_matching_delimiter(chars, scan, '(', ')') {
            let params = parse_arguments(
                text,
                chars,
                grammar,
                rules,
                scan + 1,
                close_paren,
                depth + 1,
            );
            children.extend(params);
            scan = close_paren + 1;
        }
    }

    // Scan until body block `{ ... }`
    while scan < end {
        let ch = chars[scan].1;
        if ch == '{' {
            if let Some(close_brace) = find_matching_delimiter(chars, scan, '{', '}') {
                let block_start = scan;
                let block_end = close_brace + 1;
                let inner = parse_nodes(
                    text,
                    chars,
                    grammar,
                    rules,
                    block_start + 1,
                    close_brace,
                    depth + 1,
                );
                children.push(ast_node::composite(
                    syntax_kind::block_item,
                    block_start,
                    block_end,
                    inner,
                ));
            }
            break;
        }
        if ch == ';' {
            break;
        }
        scan += 1;
    }
}

/// Evaluates a single declarative [`grammar_rule`] at `start_idx`.
///
/// The rule arms (span, keyword, etc.) share little structure, so splitting this match into
/// helpers would pass most of the same arguments through each one. The line-limit lint is
/// allowed here.
#[allow(clippy::too_many_lines)]
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
