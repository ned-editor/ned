//! # `ned_fuzz`
//!
//! A dedicated, bounded fuzzy subsequence scoring and alignment engine engineered for Ned.
//! Provides deterministic ranking with boundary bonuses, consecutive hit bonuses,
//! exact case bonuses, and exact match index extraction for UI highlighting.

pub mod matcher;

pub use matcher::{
    fuzzy_match, fuzzy_matcher, match_indices, score_candidate,
};
