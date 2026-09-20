use std::collections::HashSet;

#[derive(Clone, Debug, Default)]
pub struct SearchEngine;

impl SearchEngine {
    /// Tokenize text into lowercased alphanumeric keywords.
    pub fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    /// Calculate search relevance score (matches count).
    pub fn score_match(query_tokens: &[String], item_text: &str) -> usize {
        let item_tokens: HashSet<String> = Self::tokenize(item_text).into_iter().collect();
        query_tokens.iter().filter(|t| item_tokens.iter().any(|it| it.contains(t.as_str()))).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_tokenization_and_scoring() {
        let tokens = SearchEngine::tokenize("Hello, World! #Rust_2026");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"rust_2026".to_string()));

        let query = vec!["rust".to_string(), "world".to_string()];
        let score = SearchEngine::score_match(&query, "Rust is a modern systems language for the world.");
        assert_eq!(score, 2);
    }
}
