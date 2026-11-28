//! Candidate filtering and matching logic for picker using `ned_fuzz`.

use ned_fuzz::fuzzy_matcher;

/// Filters item candidates matching the query string, returning their indices sorted by score.
#[must_use]
pub fn filter_candidates(query: &str, items: &[String]) -> Vec<usize> {
    let matcher = fuzzy_matcher::default();
    let ranked = matcher.filter_and_rank(query, items, |s| s.as_str());
    ranked.into_iter().map(|(idx, _)| idx).collect()
}

/// Finds the character indices in `text` that match the subsequence `query`.
#[must_use]
pub fn find_subsequence_matches(text: &str, query: &str) -> Vec<usize> {
    ned_fuzz::match_indices(query, text)
}
