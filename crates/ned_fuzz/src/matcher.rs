//! Bounded fuzzy subsequence scoring and optimal alignment engine.
//!
//! Provides deterministic linear/quadratic bounded subsequence matching with
//! bonuses for word/path boundaries, consecutive hits, exact case, and basename
//! locality.

/// Maximum candidate character length evaluated in dynamic programming matrix.
const MAX_DP_CANDIDATE_LEN: usize = 1024;
/// Maximum query character length evaluated.
const MAX_QUERY_LEN: usize = 64;

/// Base score awarded for any matching character.
const SCORE_MATCH: i64 = 16;
/// Bonus awarded for consecutive character matches.
const BONUS_CONSECUTIVE: i64 = 24;
/// Bonus awarded for matching at the start of the candidate string.
const BONUS_START: i64 = 28;
/// Bonus awarded for matching immediately after a path separator (`/` or `\`).
const BONUS_PATH_BOUNDARY: i64 = 32;
/// Bonus awarded for matching immediately after a word delimiter (`_`, `-`, `.`, space).
const BONUS_WORD_BOUNDARY: i64 = 20;
/// Bonus awarded for camelCase boundary (lowercase followed by uppercase).
const BONUS_CAMEL: i64 = 18;
/// Bonus awarded for matching exact case.
const BONUS_EXACT_CASE: i64 = 8;
/// Bonus awarded for characters matched inside the filename basename.
const BONUS_BASENAME: i64 = 12;
/// Bonus awarded for matching the first character of the filename basename.
const BONUS_BASENAME_START: i64 = 30;

/// Represents the result of a successful fuzzy match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct fuzzy_match {
    /// Total computed match score (higher is better).
    pub score: i64,
    /// 0-based character indices within the candidate string that matched.
    pub indices: Vec<usize>,
}

/// Configuration options and execution state for fuzzy matching.
#[derive(Clone, Debug)]
pub struct fuzzy_matcher {
    /// Whether smart-case matching is enabled (case-insensitive unless query has uppercase).
    pub smart_case: bool,
    /// Maximum candidate length in characters before truncation.
    pub max_candidate_len: usize,
}

impl Default for fuzzy_matcher {
    fn default() -> Self {
        Self {
            smart_case: true,
            max_candidate_len: 4096,
        }
    }
}

impl fuzzy_matcher {
    /// Creates a new default fuzzy matcher.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Evaluates `candidate` against `query` and returns match score and indices if matched.
    #[must_use]
    pub fn score_candidate(&self, query: &str, candidate: &str) -> Option<fuzzy_match> {
        if query.is_empty() {
            return Some(fuzzy_match {
                score: 0,
                indices: Vec::new(),
            });
        }

        let q_chars: Vec<char> = query.chars().take(MAX_QUERY_LEN).collect();
        let c_chars: Vec<char> = candidate.chars().take(self.max_candidate_len).collect();

        if q_chars.len() > c_chars.len() {
            return None;
        }

        let case_insensitive = if self.smart_case {
            !q_chars.iter().any(|c| c.is_uppercase())
        } else {
            false
        };

        // 1. Fast linear subsequence check (O(N) early exit with zero allocations)
        if !is_subsequence(&q_chars, &c_chars, case_insensitive) {
            return None;
        }

        // 2. Compute match scores and optimal alignment
        Self::align_and_score(&q_chars, &c_chars, case_insensitive)
    }

    /// Scores and ranks a collection of candidate items against `query`.
    ///
    /// Returns a list of `(original_index, fuzzy_match)` sorted descending by score,
    /// with deterministic tie-breaking preserving original item index order.
    pub fn filter_and_rank<T, F>(
        &self,
        query: &str,
        items: &[T],
        key: F,
    ) -> Vec<(usize, fuzzy_match)>
    where
        F: Fn(&T) -> &str,
    {
        if query.is_empty() {
            return items
                .iter()
                .enumerate()
                .map(|(idx, _)| {
                    (
                        idx,
                        fuzzy_match {
                            score: 0,
                            indices: Vec::new(),
                        },
                    )
                })
                .collect();
        }

        let mut results = Vec::new();
        for (idx, item) in items.iter().enumerate() {
            let text = key(item);
            if let Some(m) = self.score_candidate(query, text) {
                results.push((idx, m));
            }
        }

        // Deterministic sort: highest score first, stable tie-break on lower index
        results.sort_by(|(idx_a, match_a), (idx_b, match_b)| {
            match_b
                .score
                .cmp(&match_a.score)
                .then_with(|| idx_a.cmp(idx_b))
        });

        results
    }

    /// Computes optimal score and matching indices using dynamic programming alignment.
    #[allow(clippy::too_many_lines, clippy::cast_possible_wrap)]
    fn align_and_score(
        q_chars: &[char],
        c_chars: &[char],
        case_insensitive: bool,
    ) -> Option<fuzzy_match> {
        let m = q_chars.len();
        let n = c_chars.len().min(MAX_DP_CANDIDATE_LEN);

        let last_slash_idx = c_chars[..n].iter().rposition(|&c| c == '/' || c == '\\');

        // Matrix dimensions: m rows by n columns
        // dp[i * n + j] = best score matching query prefix 0..=i ending at candidate index j
        // parent[i * n + j] = candidate index k < j from previous row
        let mut dp = vec![i64::MIN; m * n];
        let mut parent = vec![None; m * n];

        // Row 0: matching first query character
        let q0 = q_chars[0];
        for j in 0..n {
            let cj = c_chars[j];
            if chars_match(q0, cj, case_insensitive) {
                let mut score = SCORE_MATCH;
                score += compute_boundary_bonus(c_chars, j, last_slash_idx);
                if q0 == cj {
                    score += BONUS_EXACT_CASE;
                }
                // Slight penalty for distance from start of string or basename
                let dist_penalty = if let Some(slash) = last_slash_idx {
                    if j > slash {
                        ((j - slash - 1) as i64).min(10)
                    } else {
                        (j as i64).min(20)
                    }
                } else {
                    (j as i64).min(20)
                };
                dp[j] = score - dist_penalty;
            }
        }

        // Subsequent rows
        for (i, &qi) in q_chars.iter().enumerate().take(m).skip(1) {
            let row_offset = i * n;
            let prev_row_offset = (i - 1) * n;

            // Track maximum score in previous row up to j - 1
            let mut best_prev_k: Option<usize> = None;
            let mut best_prev_score = i64::MIN;

            for j in 0..n {
                let cj = c_chars[j];

                // Update best_prev before considering matching at j
                if j > 0 {
                    let k = j - 1;
                    let prev_val = dp[prev_row_offset + k];
                    if prev_val > best_prev_score {
                        best_prev_score = prev_val;
                        best_prev_k = Some(k);
                    }
                }

                if !chars_match(qi, cj, case_insensitive) {
                    continue;
                }

                let mut current_best = i64::MIN;
                let mut chosen_parent = None;

                // Option A: Consecutive match from immediately preceding character (j - 1)
                if j > 0 {
                    let consec_prev = dp[prev_row_offset + (j - 1)];
                    if consec_prev != i64::MIN {
                        let mut consec_score = consec_prev + SCORE_MATCH + BONUS_CONSECUTIVE;
                        if qi == cj {
                            consec_score += BONUS_EXACT_CASE;
                        }
                        if let Some(slash) = last_slash_idx {
                            if j > slash {
                                consec_score += BONUS_BASENAME;
                            }
                        }
                        if consec_score > current_best {
                            current_best = consec_score;
                            chosen_parent = Some(j - 1);
                        }
                    }
                }

                // Option B: Match from any earlier candidate position k < j - 1
                if let Some(k) = best_prev_k {
                    if k < j.saturating_sub(1) && best_prev_score != i64::MIN {
                        let mut non_consec = best_prev_score + SCORE_MATCH;
                        non_consec += compute_boundary_bonus(c_chars, j, last_slash_idx);
                        if qi == cj {
                            non_consec += BONUS_EXACT_CASE;
                        }
                        let gap = ((j - k) as i64).min(10);
                        non_consec -= gap;
                        if non_consec > current_best {
                            current_best = non_consec;
                            chosen_parent = Some(k);
                        }
                    }
                }

                if current_best != i64::MIN {
                    dp[row_offset + j] = current_best;
                    parent[row_offset + j] = chosen_parent;
                }
            }
        }

        // Find highest score in the last row (m - 1)
        let last_row_offset = (m - 1) * n;
        let mut best_last_j = None;
        let mut best_total_score = i64::MIN;

        for j in 0..n {
            let score = dp[last_row_offset + j];
            if score > best_total_score {
                best_total_score = score;
                best_last_j = Some(j);
            }
        }

        let best_j = best_last_j?;

        // Backtrack to extract matched indices
        let mut indices = Vec::with_capacity(m);
        let mut curr_j = best_j;
        indices.push(curr_j);

        for i in (1..m).rev() {
            let row_offset = i * n;
            let p = parent[row_offset + curr_j]?;
            indices.push(p);
            curr_j = p;
        }

        indices.reverse();

        // Apply slight total length penalty so more concise matches rank higher
        let length_penalty = (c_chars.len() as i64) / 4;
        let final_score = best_total_score.saturating_sub(length_penalty);

        Some(fuzzy_match {
            score: final_score,
            indices,
        })
    }
}

/// Checks if character `query_char` matches `cand_char` honoring case insensitivity.
#[inline]
fn chars_match(query_char: char, cand_char: char, case_insensitive: bool) -> bool {
    if case_insensitive {
        query_char.eq_ignore_ascii_case(&cand_char)
    } else {
        query_char == cand_char
    }
}

/// Performs a fast linear subsequence verification across `q_chars` and `c_chars`.
fn is_subsequence(q_chars: &[char], c_chars: &[char], case_insensitive: bool) -> bool {
    let mut qi = 0;
    for &c in c_chars {
        if chars_match(q_chars[qi], c, case_insensitive) {
            qi += 1;
            if qi == q_chars.len() {
                return true;
            }
        }
    }
    false
}

/// Computes boundary bonus for candidate character at index `j`.
fn compute_boundary_bonus(c_chars: &[char], j: usize, last_slash_idx: Option<usize>) -> i64 {
    let mut bonus = 0;

    if j == 0 {
        bonus += BONUS_START;
    } else {
        let prev = c_chars[j - 1];
        if prev == '/' || prev == '\\' {
            bonus += BONUS_PATH_BOUNDARY;
        } else if prev == '_' || prev == '-' || prev == '.' || prev == ' ' || prev == ':' {
            bonus += BONUS_WORD_BOUNDARY;
        } else if prev.is_lowercase() && c_chars[j].is_uppercase() {
            bonus += BONUS_CAMEL;
        }
    }

    if let Some(slash) = last_slash_idx {
        if j == slash + 1 {
            bonus += BONUS_BASENAME_START;
        } else if j > slash {
            bonus += BONUS_BASENAME;
        }
    }

    bonus
}

/// Convenience helper scoring a single candidate string with default options.
#[must_use]
pub fn score_candidate(query: &str, candidate: &str) -> Option<fuzzy_match> {
    fuzzy_matcher::default().score_candidate(query, candidate)
}

/// Convenience helper returning the matched character indices for `text` given `query`.
#[must_use]
pub fn match_indices(query: &str, text: &str) -> Vec<usize> {
    score_candidate(query, text)
        .map(|m| m.indices)
        .unwrap_or_default()
}
