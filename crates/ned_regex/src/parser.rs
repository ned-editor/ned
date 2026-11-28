//! Regular expression pattern parser for `ned_regex`.

use std::fmt;

/// Errors that can occur during regular expression parsing or compilation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum regex_error {
    /// Unexpected end of regular expression pattern.
    unexpected_end_of_pattern,
    /// Unclosed parenthesis group.
    unclosed_group,
    /// Unclosed character class bracket.
    unclosed_character_class,
    /// Invalid quantifier syntax.
    invalid_quantifier(String),
    /// Target preceding a quantifier is missing or invalid.
    quantifier_without_target,
    /// Invalid character range in class, such as `z-a`.
    invalid_character_range(char, char),
    /// Unrecognized or trailing escape sequence.
    invalid_escape(char),
    /// Pattern complexity exceeded allowable bounds.
    pattern_too_complex,
}

impl fmt::Display for regex_error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::unexpected_end_of_pattern => write!(f, "unexpected end of regular expression"),
            Self::unclosed_group => write!(f, "unclosed group in regular expression"),
            Self::unclosed_character_class => {
                write!(f, "unclosed character class in regular expression")
            }
            Self::invalid_quantifier(s) => write!(f, "invalid quantifier: {s}"),
            Self::quantifier_without_target => {
                write!(f, "quantifier operand missing in regular expression")
            }
            Self::invalid_character_range(start, end) => {
                write!(f, "invalid character range: '{start}'-'{end}'")
            }
            Self::invalid_escape(c) => write!(f, "invalid escape sequence '\\{c}'"),
            Self::pattern_too_complex => write!(f, "regular expression exceeded complexity limit"),
        }
    }
}

impl std::error::Error for regex_error {}

/// Kinds of zero-width assertions supported in regular expressions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum anchor_kind {
    /// Beginning of line (`^`).
    start_line,
    /// End of line (`$`).
    end_line,
    /// Word boundary (`\b`).
    word_boundary,
    /// Non-word boundary (`\B`).
    non_word_boundary,
}

/// Abstract syntax tree representation of a regular expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ast {
    /// Empty pattern matching zero characters.
    empty,
    /// Exact literal character match.
    literal(char),
    /// Character class matching sets and ranges.
    char_class {
        /// Inclusive character ranges `(start, end)`.
        ranges: Vec<(char, char)>,
        /// Whether the class is negated (`[^...]`).
        negated: bool,
    },
    /// Any character (excluding newline by default).
    any_char,
    /// Zero-width anchor assertion.
    anchor(anchor_kind),
    /// Sequential concatenation of sub-patterns.
    concat(Vec<ast>),
    /// Disjunction/alternation between sub-patterns (`|`).
    alternation(Vec<ast>),
    /// Repetition quantifier (`*`, `+`, `?`, `{n,m}`).
    repetition {
        /// Sub-pattern being repeated.
        node: Box<ast>,
        /// Minimum occurrences.
        min: usize,
        /// Maximum occurrences, or `None` for unbounded.
        max: Option<usize>,
        /// Whether the repetition is greedy.
        greedy: bool,
    },
    /// Explicit grouping `(...)`.
    group(Box<ast>),
}

/// Parses regular expression string into an AST.
pub struct pattern_parser<'a> {
    chars: Vec<char>,
    pos: usize,
    _marker: std::marker::PhantomData<&'a str>,
}

impl<'a> pattern_parser<'a> {
    /// Creates a new regular expression parser for `pattern`.
    #[must_use]
    pub fn new(pattern: &'a str) -> Self {
        Self {
            chars: pattern.chars().collect(),
            pos: 0,
            _marker: std::marker::PhantomData,
        }
    }

    /// Parses the entire pattern into a root `ast`.
    pub fn parse(&mut self) -> Result<ast, regex_error> {
        let node = self.parse_alternation()?;
        if self.pos < self.chars.len() {
            if self.chars[self.pos] == ')' {
                return Err(regex_error::unclosed_group);
            }
            return Err(regex_error::pattern_too_complex);
        }
        Ok(node)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<char> {
        if self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }

    fn parse_alternation(&mut self) -> Result<ast, regex_error> {
        let mut branches = Vec::new();
        let first = self.parse_concat()?;
        branches.push(first);

        while self.peek() == Some('|') {
            self.next(); // consume '|'
            let branch = self.parse_concat()?;
            branches.push(branch);
        }

        if branches.len() == 1 {
            Ok(branches.pop().unwrap())
        } else {
            Ok(ast::alternation(branches))
        }
    }

    fn parse_concat(&mut self) -> Result<ast, regex_error> {
        let mut nodes = Vec::new();

        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            let atom = self.parse_quantified()?;
            nodes.push(atom);
        }

        if nodes.is_empty() {
            Ok(ast::empty)
        } else if nodes.len() == 1 {
            Ok(nodes.pop().unwrap())
        } else {
            Ok(ast::concat(nodes))
        }
    }

    fn parse_quantified(&mut self) -> Result<ast, regex_error> {
        let atom = self.parse_atom()?;

        let Some(c) = self.peek() else {
            return Ok(atom);
        };

        match c {
            '*' => {
                self.next();
                let greedy = self.check_greedy();
                Ok(ast::repetition {
                    node: Box::new(atom),
                    min: 0,
                    max: None,
                    greedy,
                })
            }
            '+' => {
                self.next();
                let greedy = self.check_greedy();
                Ok(ast::repetition {
                    node: Box::new(atom),
                    min: 1,
                    max: None,
                    greedy,
                })
            }
            '?' => {
                self.next();
                let greedy = self.check_greedy();
                Ok(ast::repetition {
                    node: Box::new(atom),
                    min: 0,
                    max: Some(1),
                    greedy,
                })
            }
            '{' => {
                if let Some((min, max)) = self.try_parse_range_quantifier()? {
                    let greedy = self.check_greedy();
                    Ok(ast::repetition {
                        node: Box::new(atom),
                        min,
                        max,
                        greedy,
                    })
                } else {
                    Ok(atom)
                }
            }
            _ => Ok(atom),
        }
    }

    fn check_greedy(&mut self) -> bool {
        if self.peek() == Some('?') {
            self.next();
            false
        } else {
            true
        }
    }

    fn try_parse_range_quantifier(
        &mut self,
    ) -> Result<Option<(usize, Option<usize>)>, regex_error> {
        let saved_pos = self.pos;
        self.next(); // consume '{'

        let mut min_str = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                min_str.push(c);
                self.next();
            } else {
                break;
            }
        }

        if min_str.is_empty() {
            self.pos = saved_pos;
            return Ok(None);
        }

        let Ok(min) = min_str.parse::<usize>() else {
            self.pos = saved_pos;
            return Ok(None);
        };

        if self.peek() == Some('}') {
            self.next();
            return Ok(Some((min, Some(min))));
        }

        if self.peek() == Some(',') {
            self.next(); // consume ','
            let mut max_str = String::new();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    max_str.push(c);
                    self.next();
                } else {
                    break;
                }
            }

            if self.peek() == Some('}') {
                self.next();
                if max_str.is_empty() {
                    return Ok(Some((min, None)));
                }
                let Ok(max) = max_str.parse::<usize>() else {
                    return Err(regex_error::invalid_quantifier(format!("{min},{max_str}")));
                };
                if max < min {
                    return Err(regex_error::invalid_quantifier(format!("{min},{max}")));
                }
                return Ok(Some((min, Some(max))));
            }
        }

        self.pos = saved_pos;
        Ok(None)
    }

    fn parse_atom(&mut self) -> Result<ast, regex_error> {
        let Some(c) = self.next() else {
            return Err(regex_error::unexpected_end_of_pattern);
        };

        match c {
            '.' => Ok(ast::any_char),
            '^' => Ok(ast::anchor(anchor_kind::start_line)),
            '$' => Ok(ast::anchor(anchor_kind::end_line)),
            '(' => {
                let is_non_capturing = if self.peek() == Some('?') {
                    self.next();
                    if self.peek() == Some(':') {
                        self.next();
                        true
                    } else {
                        return Err(regex_error::pattern_too_complex);
                    }
                } else {
                    false
                };
                let inner = self.parse_alternation()?;
                if self.next() != Some(')') {
                    return Err(regex_error::unclosed_group);
                }
                if is_non_capturing {
                    Ok(inner)
                } else {
                    Ok(ast::group(Box::new(inner)))
                }
            }
            '[' => self.parse_character_class(),
            '\\' => self.parse_escape(),
            '*' | '+' | '?' => Err(regex_error::quantifier_without_target),
            literal => Ok(ast::literal(literal)),
        }
    }

    fn parse_escape(&mut self) -> Result<ast, regex_error> {
        let Some(c) = self.next() else {
            return Err(regex_error::unexpected_end_of_pattern);
        };

        match c {
            'n' => Ok(ast::literal('\n')),
            'r' => Ok(ast::literal('\r')),
            't' => Ok(ast::literal('\t')),
            '0' => Ok(ast::literal('\0')),
            'b' => Ok(ast::anchor(anchor_kind::word_boundary)),
            'B' => Ok(ast::anchor(anchor_kind::non_word_boundary)),
            'd' => Ok(ast::char_class {
                ranges: vec![('0', '9')],
                negated: false,
            }),
            'D' => Ok(ast::char_class {
                ranges: vec![('0', '9')],
                negated: true,
            }),
            'w' => Ok(ast::char_class {
                ranges: vec![('a', 'z'), ('A', 'Z'), ('0', '9'), ('_', '_')],
                negated: false,
            }),
            'W' => Ok(ast::char_class {
                ranges: vec![('a', 'z'), ('A', 'Z'), ('0', '9'), ('_', '_')],
                negated: true,
            }),
            's' => Ok(ast::char_class {
                ranges: vec![
                    (' ', ' '),
                    ('\t', '\t'),
                    ('\n', '\n'),
                    ('\r', '\r'),
                    ('\x0C', '\x0C'),
                ],
                negated: false,
            }),
            'S' => Ok(ast::char_class {
                ranges: vec![
                    (' ', ' '),
                    ('\t', '\t'),
                    ('\n', '\n'),
                    ('\r', '\r'),
                    ('\x0C', '\x0C'),
                ],
                negated: true,
            }),
            punct => Ok(ast::literal(punct)),
        }
    }

    fn parse_character_class(&mut self) -> Result<ast, regex_error> {
        let negated = if self.peek() == Some('^') {
            self.next();
            true
        } else {
            false
        };

        let mut ranges = Vec::new();
        let mut first = true;

        while let Some(c) = self.next() {
            if c == ']' && !first {
                return Ok(ast::char_class { ranges, negated });
            }
            first = false;

            let actual_char = if c == '\\' {
                let Some(esc) = self.next() else {
                    return Err(regex_error::unexpected_end_of_pattern);
                };
                match esc {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '0' => '\0',
                    'd' => {
                        ranges.push(('0', '9'));
                        continue;
                    }
                    'w' => {
                        ranges.push(('a', 'z'));
                        ranges.push(('A', 'Z'));
                        ranges.push(('0', '9'));
                        ranges.push(('_', '_'));
                        continue;
                    }
                    's' => {
                        ranges.push((' ', ' '));
                        ranges.push(('\t', '\t'));
                        ranges.push(('\n', '\n'));
                        ranges.push(('\r', '\r'));
                        continue;
                    }
                    other => other,
                }
            } else {
                c
            };

            if self.peek() == Some('-') {
                let saved = self.pos;
                self.next(); // consume '-'
                if let Some(next_c) = self.peek() {
                    if next_c != ']' {
                        self.next(); // consume next_c
                        let end_char = if next_c == '\\' {
                            match self.next() {
                                Some('n') => '\n',
                                Some('r') => '\r',
                                Some('t') => '\t',
                                Some('0') => '\0',
                                Some(o) => o,
                                None => return Err(regex_error::unexpected_end_of_pattern),
                            }
                        } else {
                            next_c
                        };

                        if actual_char > end_char {
                            return Err(regex_error::invalid_character_range(
                                actual_char,
                                end_char,
                            ));
                        }
                        ranges.push((actual_char, end_char));
                        continue;
                    }
                }
                self.pos = saved;
            }

            ranges.push((actual_char, actual_char));
        }

        Err(regex_error::unclosed_character_class)
    }
}
