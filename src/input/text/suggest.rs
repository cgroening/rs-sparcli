//! Suggestion matching (prefix or fuzzy subsequence) for the text prompt.

use crate::input::text::{MatchMode, TextInput};

impl TextInput {
    /// Returns the suggestion indices matching `value` (in declared order).
    pub(super) fn matches(&self, value: &str) -> Vec<usize> {
        if value.is_empty() {
            return Vec::new();
        }
        let needle = value.to_lowercase();
        self.suggestions
            .iter()
            .enumerate()
            .filter(|(_, s)| matches_suggestion(&needle, s, self.match_mode))
            .map(|(index, _)| index)
            .collect()
    }
}

/// Returns whether `suggestion` matches the lowercase `needle`.
fn matches_suggestion(needle: &str, suggestion: &str, mode: MatchMode) -> bool {
    let hay = suggestion.to_lowercase();
    match mode {
        MatchMode::Prefix => hay.starts_with(needle),
        MatchMode::Subsequence => is_subsequence(needle, &hay),
    }
}

/// Returns whether all chars of `needle` appear in `hay` in order.
fn is_subsequence(needle: &str, hay: &str) -> bool {
    let mut chars = hay.chars();
    needle.chars().all(|target| chars.any(|c| c == target))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_mode_matches_only_leading_text() {
        assert!(matches_suggestion("ap", "Apple", MatchMode::Prefix));
        assert!(!matches_suggestion("pl", "Apple", MatchMode::Prefix));
    }

    #[test]
    fn subsequence_mode_matches_gapped_characters() {
        assert!(matches_suggestion("fb", "foobar", MatchMode::Subsequence));
        assert!(!matches_suggestion("bf", "foobar", MatchMode::Subsequence));
    }

    #[test]
    fn is_subsequence_respects_order() {
        assert!(is_subsequence("abc", "aXbXc"));
        assert!(!is_subsequence("cba", "aXbXc"));
    }
}
