//! Fuzzy matching for command palette

/// Calculate fuzzy match score for a query against a target string.
/// Returns None if no match, Some(score) if matches.
/// Higher scores are better matches.
pub fn fuzzy_score(query: &str, target: &str) -> Option<u32> {
    let query = query.to_lowercase();
    let target_lower = target.to_lowercase();

    // Empty query matches everything with score 0
    if query.is_empty() {
        return Some(0);
    }

    // Exact match - highest score
    if target_lower == query {
        return Some(1000);
    }

    // Prefix match - very high score
    if target_lower.starts_with(&query) {
        // Bonus for shorter targets (more specific match)
        let bonus = 100_u32.saturating_sub(target.len() as u32);
        return Some(500 + bonus);
    }

    // Contains match - medium score
    if target_lower.contains(&query) {
        return Some(250);
    }

    // Subsequence match - lower score based on how spread out the match is
    let mut score = 0_u32;
    let mut query_chars = query.chars().peekable();
    let mut prev_matched = false;
    let mut consecutive_bonus = 0_u32;

    for c in target_lower.chars() {
        if query_chars.peek() == Some(&c) {
            query_chars.next();
            score += 10;
            // Bonus for consecutive matches
            if prev_matched {
                consecutive_bonus += 5;
            }
            prev_matched = true;
        } else {
            prev_matched = false;
        }
    }

    // Only return a score if all query chars were found
    if query_chars.peek().is_none() {
        Some(score + consecutive_bonus)
    } else {
        None
    }
}

/// Match and score against multiple fields, returning the best score
pub fn fuzzy_score_multi(query: &str, targets: &[&str]) -> Option<u32> {
    targets.iter().filter_map(|t| fuzzy_score(query, t)).max()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_query() {
        assert_eq!(fuzzy_score("", "anything"), Some(0));
    }

    #[test]
    fn test_exact_match() {
        assert_eq!(fuzzy_score("copy", "Copy"), Some(1000));
        assert_eq!(fuzzy_score("COPY", "copy"), Some(1000));
    }

    #[test]
    fn test_prefix_match() {
        let score = fuzzy_score("cop", "Copy").unwrap();
        assert!(score >= 500 && score < 1000);
    }

    #[test]
    fn test_contains_match() {
        assert_eq!(fuzzy_score("opy", "Copy"), Some(250));
    }

    #[test]
    fn test_subsequence_match() {
        let score = fuzzy_score("cy", "Copy").unwrap();
        assert!(score > 0 && score < 250);
    }

    #[test]
    fn test_no_match() {
        assert_eq!(fuzzy_score("xyz", "Copy"), None);
    }

    #[test]
    fn test_multi_score() {
        // Should pick the best score from multiple targets
        let score = fuzzy_score_multi("git", &["Git", "Version Control", "G"]).unwrap();
        assert_eq!(score, 1000); // Exact match on "Git"
    }
}
