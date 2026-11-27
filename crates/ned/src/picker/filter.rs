//! Candidate filtering and matching logic for picker.

/// Filters item candidates matching the query string, returning their indices.
#[must_use]
pub fn filter_candidates(query: &str, items: &[String]) -> Vec<usize> {
    let q = query.to_lowercase();
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            if q.is_empty() {
                return true;
            }
            item.to_lowercase().contains(&q)
        })
        .map(|(i, _)| i)
        .collect()
}

/// Finds the character indices in `text` that match the subsequence `query`.
#[must_use]
pub fn find_subsequence_matches(text: &str, query: &str) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }
    let q_chars: Vec<char> = query.to_lowercase().chars().collect();
    let t_chars: Vec<char> = text.to_lowercase().chars().collect();
    let mut indices = Vec::with_capacity(q_chars.len());
    let mut q_idx = 0;
    for (t_idx, &t_ch) in t_chars.iter().enumerate() {
        if q_idx < q_chars.len() && t_ch == q_chars[q_idx] {
            indices.push(t_idx);
            q_idx += 1;
        }
    }
    if q_idx == q_chars.len() {
        indices
    } else {
        Vec::new()
    }
}
