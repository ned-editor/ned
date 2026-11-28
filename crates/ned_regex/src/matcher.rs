//! Thompson NFA (`PikeVM`) matching engine for `ned_regex`.

use crate::compiler::{inst, nfa_program};

/// Represents a matched range within haystack text by byte offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct regex_match {
    /// Start byte offset (inclusive).
    pub start: usize,
    /// End byte offset (exclusive).
    pub end: usize,
}

impl regex_match {
    /// Constructs a new match range.
    #[inline]
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Returns the matched byte range.
    #[inline]
    #[must_use]
    pub const fn range(&self) -> std::ops::Range<usize> {
        self.start..self.end
    }

    /// Extracts the matched substring from `haystack`.
    #[inline]
    #[must_use]
    pub fn as_str<'a>(&self, haystack: &'a str) -> &'a str {
        &haystack[self.start..self.end]
    }
}

#[derive(Clone, Copy, Debug)]
/// A single NFA execution thread during `PikeVM` simulation.
struct thread {
    /// Current instruction pointer (program counter).
    pc: usize,
    /// Starting byte offset of this thread's match attempt.
    start_byte: usize,
}

/// `PikeVM` execution context.
pub struct pattern_matcher<'a> {
    /// Compiled NFA program to execute against.
    program: &'a nfa_program,
}

impl<'a> pattern_matcher<'a> {
    /// Creates a new matcher for compiled NFA `program`.
    #[inline]
    #[must_use]
    pub const fn new(program: &'a nfa_program) -> Self {
        Self { program }
    }

    /// Searches for the leftmost match in `haystack` starting at `start_byte`.
    ///
    /// The single-threaded simulation loop is intentionally kept together so the
    /// thread visit-generation logic stays local; the line-limit lint is allowed here.
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn find_at(&self, haystack: &str, start_byte: usize) -> Option<regex_match> {
        let start_byte = if start_byte >= haystack.len() {
            haystack.len()
        } else if haystack.is_char_boundary(start_byte) {
            start_byte
        } else {
            haystack
                .char_indices()
                .map(|(i, _)| i)
                .find(|&i| i >= start_byte)
                .unwrap_or(haystack.len())
        };

        let mut curr_threads = Vec::with_capacity(32);
        let mut next_threads = Vec::with_capacity(32);
        let mut visited = vec![0u32; self.program.insts.len()];
        let mut gen = 1u32;

        let mut best_match: Option<regex_match> = None;

        for (rel_offset, c) in haystack[start_byte..].char_indices() {
            let curr_byte = start_byte + rel_offset;
            let next_byte = curr_byte + c.len_utf8();

            // If unanchored (or anchored at start_line and at a newline boundary), spawn a thread
            let can_spawn = if self.program.is_anchored_start {
                curr_byte == 0 || (curr_byte > 0 && haystack[..curr_byte].ends_with('\n'))
            } else {
                true
            };

            if can_spawn {
                let should_spawn = match best_match {
                    Some(ref m) => curr_byte < m.start,
                    None => true,
                };

                if should_spawn {
                    self.add_thread(
                        thread {
                            pc: 0,
                            start_byte: curr_byte,
                        },
                        curr_byte,
                        haystack,
                        &mut curr_threads,
                        &mut visited,
                        gen,
                        &mut best_match,
                    );
                }
            }

            // Check if we have an optimal match and no earlier starting threads remain
            if let Some(m) = best_match {
                if (curr_threads.is_empty() || curr_threads.iter().all(|t| t.start_byte >= m.start))
                    && (m.start != m.end || curr_byte > m.start)
                {
                    return Some(m);
                }
            }

            // Step active threads across character `c`
            gen = gen.wrapping_add(1);
            if gen == 0 {
                visited.fill(0);
                gen = 1;
            }

            for t in curr_threads.drain(..) {
                if let Some(ref m) = best_match {
                    if t.start_byte > m.start {
                        continue;
                    }
                }

                if t.pc >= self.program.insts.len() {
                    continue;
                }

                match &self.program.insts[t.pc] {
                    inst::char {
                        c: target_c,
                        case_insensitive,
                    } => {
                        let matched = if *case_insensitive {
                            c.to_lowercase().eq(target_c.to_lowercase())
                        } else {
                            c == *target_c
                        };

                        if matched {
                            self.add_thread(
                                thread {
                                    pc: t.pc + 1,
                                    start_byte: t.start_byte,
                                },
                                next_byte,
                                haystack,
                                &mut next_threads,
                                &mut visited,
                                gen,
                                &mut best_match,
                            );
                        }
                    }
                    inst::char_class {
                        ranges,
                        negated,
                        case_insensitive,
                    } => {
                        let in_ranges = ranges.iter().any(|(s, e)| {
                            if *case_insensitive {
                                let mut in_r = false;
                                for lc in c.to_lowercase() {
                                    for ls in s.to_lowercase() {
                                        for le in e.to_lowercase() {
                                            if lc >= ls && lc <= le {
                                                in_r = true;
                                            }
                                        }
                                    }
                                }
                                in_r
                            } else {
                                c >= *s && c <= *e
                            }
                        });

                        let matched = in_ranges != *negated;
                        if matched {
                            self.add_thread(
                                thread {
                                    pc: t.pc + 1,
                                    start_byte: t.start_byte,
                                },
                                next_byte,
                                haystack,
                                &mut next_threads,
                                &mut visited,
                                gen,
                                &mut best_match,
                            );
                        }
                    }
                    inst::any_char {
                        dot_matches_newline,
                    } if (*dot_matches_newline || c != '\n') => {
                        self.add_thread(
                            thread {
                                pc: t.pc + 1,
                                start_byte: t.start_byte,
                            },
                            next_byte,
                            haystack,
                            &mut next_threads,
                            &mut visited,
                            gen,
                            &mut best_match,
                        );
                    }
                    _ => {}
                }
            }

            std::mem::swap(&mut curr_threads, &mut next_threads);
        }

        // Final evaluation at end of haystack
        let curr_byte = haystack.len();
        let can_spawn = if self.program.is_anchored_start {
            curr_byte == 0 || (curr_byte > 0 && haystack[..curr_byte].ends_with('\n'))
        } else {
            true
        };

        if can_spawn {
            let should_spawn = match best_match {
                Some(ref m) => curr_byte < m.start,
                None => true,
            };

            if should_spawn {
                self.add_thread(
                    thread {
                        pc: 0,
                        start_byte: curr_byte,
                    },
                    curr_byte,
                    haystack,
                    &mut curr_threads,
                    &mut visited,
                    gen,
                    &mut best_match,
                );
            }
        }

        best_match
    }

    /// Recursively follows NFA transitions from thread `t`, recording matches and
    /// enqueuing successor threads. Takes 8 arguments to carry the full simulation
    /// state; splitting it would thread more state through without clarity gain.
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_lines)]
    fn add_thread(
        &self,
        t: thread,
        curr_byte: usize,
        haystack: &str,
        thread_list: &mut Vec<thread>,
        visited: &mut [u32],
        gen: u32,
        best_match: &mut Option<regex_match>,
    ) {
        if t.pc >= self.program.insts.len() {
            return;
        }

        if visited[t.pc] == gen {
            return;
        }
        visited[t.pc] = gen;

        match &self.program.insts[t.pc] {
            inst::match_success => {
                let candidate = regex_match::new(t.start_byte, curr_byte);
                match best_match {
                    None => {
                        *best_match = Some(candidate);
                    }
                    Some(ref current) => {
                        if candidate.start < current.start
                            || (candidate.start == current.start && candidate.end >= current.end)
                        {
                            *best_match = Some(candidate);
                        }
                    }
                }
            }
            inst::split { target1, target2 } => {
                let (t1, t2) = (*target1, *target2);
                self.add_thread(
                    thread {
                        pc: t1,
                        start_byte: t.start_byte,
                    },
                    curr_byte,
                    haystack,
                    thread_list,
                    visited,
                    gen,
                    best_match,
                );
                self.add_thread(
                    thread {
                        pc: t2,
                        start_byte: t.start_byte,
                    },
                    curr_byte,
                    haystack,
                    thread_list,
                    visited,
                    gen,
                    best_match,
                );
            }
            inst::jump { target } => {
                let target = *target;
                self.add_thread(
                    thread {
                        pc: target,
                        start_byte: t.start_byte,
                    },
                    curr_byte,
                    haystack,
                    thread_list,
                    visited,
                    gen,
                    best_match,
                );
            }
            inst::assert_start_line => {
                let at_line_start =
                    curr_byte == 0 || (curr_byte > 0 && haystack[..curr_byte].ends_with('\n'));
                if at_line_start {
                    self.add_thread(
                        thread {
                            pc: t.pc + 1,
                            start_byte: t.start_byte,
                        },
                        curr_byte,
                        haystack,
                        thread_list,
                        visited,
                        gen,
                        best_match,
                    );
                }
            }
            inst::assert_end_line => {
                let at_line_end = curr_byte == haystack.len()
                    || haystack[curr_byte..].starts_with('\n')
                    || haystack[curr_byte..].starts_with("\r\n");
                if at_line_end {
                    self.add_thread(
                        thread {
                            pc: t.pc + 1,
                            start_byte: t.start_byte,
                        },
                        curr_byte,
                        haystack,
                        thread_list,
                        visited,
                        gen,
                        best_match,
                    );
                }
            }
            inst::assert_word_boundary => {
                let prev_word = if curr_byte > 0 {
                    haystack[..curr_byte]
                        .chars()
                        .next_back()
                        .is_some_and(is_word_char)
                } else {
                    false
                };
                let curr_word = if curr_byte < haystack.len() {
                    haystack[curr_byte..]
                        .chars()
                        .next()
                        .is_some_and(is_word_char)
                } else {
                    false
                };

                if prev_word != curr_word {
                    self.add_thread(
                        thread {
                            pc: t.pc + 1,
                            start_byte: t.start_byte,
                        },
                        curr_byte,
                        haystack,
                        thread_list,
                        visited,
                        gen,
                        best_match,
                    );
                }
            }
            inst::assert_non_word_boundary => {
                let prev_word = if curr_byte > 0 {
                    haystack[..curr_byte]
                        .chars()
                        .next_back()
                        .is_some_and(is_word_char)
                } else {
                    false
                };
                let curr_word = if curr_byte < haystack.len() {
                    haystack[curr_byte..]
                        .chars()
                        .next()
                        .is_some_and(is_word_char)
                } else {
                    false
                };

                if prev_word == curr_word {
                    self.add_thread(
                        thread {
                            pc: t.pc + 1,
                            start_byte: t.start_byte,
                        },
                        curr_byte,
                        haystack,
                        thread_list,
                        visited,
                        gen,
                        best_match,
                    );
                }
            }
            _ => {
                thread_list.push(t);
            }
        }
    }
}

/// Returns true if `c` is a word character (alphanumeric or underscore).
#[inline]
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
