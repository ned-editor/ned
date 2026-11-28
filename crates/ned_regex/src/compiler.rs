//! NFA bytecode compiler for `ned_regex`.

use crate::parser::{anchor_kind, ast};

/// NFA bytecode instruction executed by `PikeVM`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum inst {
    /// Matches a single character, respecting case sensitivity configuration.
    char {
        /// Target character to match.
        c: char,
        /// Whether comparison ignores casing differences.
        case_insensitive: bool,
    },
    /// Matches a character class with inclusive ranges.
    char_class {
        /// Character ranges `(start, end)`.
        ranges: Vec<(char, char)>,
        /// Whether the character class is negated (`[^...]`).
        negated: bool,
        /// Whether comparison ignores casing differences.
        case_insensitive: bool,
    },
    /// Matches any single character (excluding newline unless specified).
    any_char {
        /// Whether dot also matches newline.
        dot_matches_newline: bool,
    },
    /// Asserts line beginning (`^`).
    assert_start_line,
    /// Asserts line ending (`$`).
    assert_end_line,
    /// Asserts word boundary (`\b`).
    assert_word_boundary,
    /// Asserts non-word boundary (`\B`).
    assert_non_word_boundary,
    /// Non-deterministic split into two execution branches.
    split {
        /// Primary branch target instruction index.
        target1: usize,
        /// Secondary branch target instruction index.
        target2: usize,
    },
    /// Unconditional jump to target instruction index.
    jump {
        /// Target instruction index.
        target: usize,
    },
    /// Acceptance instruction indicating successful match.
    match_success,
}

/// Compiled NFA program containing sequential bytecode instructions.
#[derive(Clone, Debug)]
pub struct nfa_program {
    /// Linear sequence of NFA instructions.
    pub insts: Vec<inst>,
    /// Whether the pattern begins with an anchor assertion.
    pub is_anchored_start: bool,
    /// Whether matching is case-insensitive.
    pub case_insensitive: bool,
}

/// Compiles an AST into an `nfa_program`.
pub struct pattern_compiler {
    insts: Vec<inst>,
    case_insensitive: bool,
}

impl pattern_compiler {
    /// Compiles `root` AST node into an NFA program.
    #[must_use]
    pub fn compile(root: &ast, case_insensitive: bool) -> nfa_program {
        let is_anchored_start = Self::check_anchored_start(root);
        let mut comp = Self {
            insts: Vec::new(),
            case_insensitive,
        };

        comp.compile_node(root);
        comp.insts.push(inst::match_success);

        nfa_program {
            insts: comp.insts,
            is_anchored_start,
            case_insensitive,
        }
    }

    fn check_anchored_start(node: &ast) -> bool {
        match node {
            ast::anchor(anchor_kind::start_line) => true,
            ast::concat(nodes) => nodes.first().is_some_and(Self::check_anchored_start),
            ast::group(inner) => Self::check_anchored_start(inner),
            ast::alternation(branches) => {
                !branches.is_empty() && branches.iter().all(Self::check_anchored_start)
            }
            _ => false,
        }
    }

    fn compile_node(&mut self, node: &ast) {
        match node {
            ast::empty => {}
            ast::literal(c) => {
                self.insts.push(inst::char {
                    c: *c,
                    case_insensitive: self.case_insensitive,
                });
            }
            ast::char_class { ranges, negated } => {
                self.insts.push(inst::char_class {
                    ranges: ranges.clone(),
                    negated: *negated,
                    case_insensitive: self.case_insensitive,
                });
            }
            ast::any_char => {
                self.insts.push(inst::any_char {
                    dot_matches_newline: false,
                });
            }
            ast::anchor(kind) => match kind {
                anchor_kind::start_line => self.insts.push(inst::assert_start_line),
                anchor_kind::end_line => self.insts.push(inst::assert_end_line),
                anchor_kind::word_boundary => self.insts.push(inst::assert_word_boundary),
                anchor_kind::non_word_boundary => self.insts.push(inst::assert_non_word_boundary),
            },
            ast::concat(nodes) => {
                for n in nodes {
                    self.compile_node(n);
                }
            }
            ast::alternation(branches) => {
                if branches.is_empty() {
                    return;
                }
                if branches.len() == 1 {
                    self.compile_node(&branches[0]);
                    return;
                }

                let mut end_jumps = Vec::new();
                let last_idx = branches.len() - 1;

                for (i, branch) in branches.iter().enumerate() {
                    if i < last_idx {
                        let split_idx = self.insts.len();
                        self.insts.push(inst::split {
                            target1: 0,
                            target2: 0,
                        });

                        let b_start = self.insts.len();
                        self.compile_node(branch);
                        let j_idx = self.insts.len();
                        self.insts.push(inst::jump { target: 0 });
                        end_jumps.push(j_idx);

                        let next_branch_start = self.insts.len();
                        if let inst::split {
                            ref mut target1,
                            ref mut target2,
                        } = self.insts[split_idx]
                        {
                            *target1 = b_start;
                            *target2 = next_branch_start;
                        }
                    } else {
                        self.compile_node(branch);
                    }
                }

                let end_pos = self.insts.len();
                for j_idx in end_jumps {
                    if let inst::jump { ref mut target } = self.insts[j_idx] {
                        *target = end_pos;
                    }
                }
            }
            ast::group(inner) => {
                self.compile_node(inner);
            }
            ast::repetition {
                node,
                min,
                max,
                greedy,
            } => {
                // Compile the mandatory minimum repetitions
                for _ in 0..*min {
                    self.compile_node(node);
                }

                match max {
                    Some(m) if *m == *min => {
                        // Exact match count, already emitted min repetitions
                    }
                    Some(m) => {
                        // Bounded optional repetitions
                        let remaining = (*m).saturating_sub(*min);
                        let mut end_patches = Vec::new();

                        for _ in 0..remaining {
                            let split_idx = self.insts.len();
                            self.insts.push(inst::split {
                                target1: 0,
                                target2: 0,
                            });

                            let body_start = self.insts.len();
                            self.compile_node(node);

                            let next_start = self.insts.len();
                            if *greedy {
                                if let inst::split {
                                    ref mut target1,
                                    ref mut target2,
                                } = self.insts[split_idx]
                                {
                                    *target1 = body_start;
                                    *target2 = next_start; // will patch to end
                                }
                            } else if let inst::split {
                                ref mut target1,
                                ref mut target2,
                            } = self.insts[split_idx]
                            {
                                *target1 = next_start; // will patch to end
                                *target2 = body_start;
                            }
                            end_patches.push((split_idx, *greedy));
                        }

                        let end_pos = self.insts.len();
                        for (split_idx, is_greedy) in end_patches {
                            if let inst::split {
                                ref mut target1,
                                ref mut target2,
                            } = self.insts[split_idx]
                            {
                                if is_greedy {
                                    *target2 = end_pos;
                                } else {
                                    *target1 = end_pos;
                                }
                            }
                        }
                    }
                    None => {
                        // Unbounded repetition (*)
                        let split_idx = self.insts.len();
                        self.insts.push(inst::split {
                            target1: 0,
                            target2: 0,
                        });

                        let body_start = self.insts.len();
                        self.compile_node(node);
                        self.insts.push(inst::jump { target: split_idx });

                        let end_pos = self.insts.len();
                        if *greedy {
                            if let inst::split {
                                ref mut target1,
                                ref mut target2,
                            } = self.insts[split_idx]
                            {
                                *target1 = body_start;
                                *target2 = end_pos;
                            }
                        } else if let inst::split {
                            ref mut target1,
                            ref mut target2,
                        } = self.insts[split_idx]
                        {
                            *target1 = end_pos;
                            *target2 = body_start;
                        }
                    }
                }
            }
        }
    }
}
