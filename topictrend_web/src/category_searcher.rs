use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub max_results: usize,
    pub min_query_length: usize,
    pub enable_fuzzy: bool,
    pub enable_caching: bool,
    pub enable_token_search: bool,
    pub token_match_weight: i32,     // Weight for token matches
    pub prefix_match_weight: i32,    // Weight for prefix matches
    pub substring_match_weight: i32, // Weight for substring matches
    pub fuzzy_match_weight: i32,     // Weight for fuzzy matches
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            min_query_length: 1,
            enable_fuzzy: true,
            enable_caching: true,
            enable_token_search: true,
            token_match_weight: 80,
            prefix_match_weight: 100,
            substring_match_weight: 50,
            fuzzy_match_weight: 25,
        }
    }
}

pub struct CategorySearcher {
    categories: Vec<String>,
    category_tokens: Vec<HashSet<String>>, // Pre-computed tokens for each category
    config: SearchConfig,
}

impl CategorySearcher {
    pub fn new(categories: Vec<String>) -> Self {
        let category_tokens = categories
            .iter()
            .map(|cat| Self::tokenize_string(cat))
            .collect();

        Self {
            categories,
            category_tokens,
            config: SearchConfig::default(),
        }
    }

    pub fn with_config(mut self, config: SearchConfig) -> Self {
        self.config = config;
        self
    }

    fn tokenize_string(s: &str) -> HashSet<String> {
        s.to_lowercase()
            .split(&[' ', '-', '_', '&', ',', '.'][..])
            .filter(|token| !token.is_empty())
            .map(|token| token.to_string())
            .collect()
    }

    fn calculate_token_score(
        &self,
        category_tokens: &HashSet<String>,
        query_tokens: &HashSet<String>,
    ) -> i32 {
        if !self.config.enable_token_search || query_tokens.is_empty() {
            return 0;
        }

        let matching_tokens = category_tokens.intersection(query_tokens).count();
        if matching_tokens == 0 {
            return 0;
        }

        // Calculate score based on proportion of matching tokens
        let token_ratio = matching_tokens as f32 / query_tokens.len() as f32;
        (token_ratio * self.config.token_match_weight as f32) as i32
    }

    fn is_fuzzy_match(&self, name: &str, query: &str) -> bool {
        if !self.config.enable_fuzzy || query.len() < 2 {
            return false;
        }

        let mut name_chars = name.chars();
        for q_char in query.chars() {
            if !name_chars.any(|n_char| n_char == q_char) {
                return false;
            }
        }
        true
    }

    pub fn search(&self, query: &str) -> Vec<String> {
        if query.len() < self.config.min_query_length {
            return self
                .categories
                .iter()
                .take(self.config.max_results)
                .cloned()
                .collect();
        }

        let query_lower = query.to_lowercase();
        let query_tokens = Self::tokenize_string(&query_lower);

        let mut scored_results: Vec<(i32, &String)> = Vec::new();

        for (index, category) in self.categories.iter().enumerate() {
            let category_lower = category.to_lowercase();
            let mut score = 0;

            // 1. Prefix matching (highest weight)
            if category_lower.starts_with(&query_lower) {
                score += self.config.prefix_match_weight;
            }

            // 2. Word boundary matching
            if category_lower
                .split(&[' ', '-', '_'][..])
                .any(|word| word.starts_with(&query_lower))
            {
                score += self.config.prefix_match_weight - 10; // Slightly less than full prefix
            }

            // 3. Token-based matching
            let token_score =
                self.calculate_token_score(&self.category_tokens[index], &query_tokens);
            score += token_score;

            // 4. Substring matching
            if category_lower.contains(&query_lower) {
                score += self.config.substring_match_weight;
            }

            // 5. Fuzzy matching (optional)
            if self.config.enable_fuzzy && self.is_fuzzy_match(&category_lower, &query_lower) {
                score += self.config.fuzzy_match_weight;
            }

            // 6. Bonus for exact token matches (when query matches a complete token)
            if self.config.enable_token_search {
                for token in &self.category_tokens[index] {
                    if token == &query_lower {
                        score += self.config.token_match_weight + 10; // Extra bonus for exact token match
                        break;
                    }
                }
            }

            if score > 0 {
                scored_results.push((score, category));
            }
        }

        // Sort by score (descending), then alphabetically
        scored_results.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
        });

        scored_results
            .into_iter()
            .take(self.config.max_results)
            .map(|(_, cat)| cat.clone())
            .collect()
    }

    // Helper method for token-based search only
    pub fn search_tokens_only(&self, query: &str) -> Vec<String> {
        if query.len() < self.config.min_query_length {
            return self
                .categories
                .iter()
                .take(self.config.max_results)
                .cloned()
                .collect();
        }

        let query_tokens = Self::tokenize_string(&query.to_lowercase());

        let mut token_results: Vec<(usize, &String)> = self
            .categories
            .iter()
            .enumerate()
            .filter_map(|(index, category)| {
                let matching_tokens = self.category_tokens[index]
                    .intersection(&query_tokens)
                    .count();

                if matching_tokens > 0 {
                    Some((matching_tokens, category))
                } else {
                    None
                }
            })
            .collect();

        // Sort by number of matching tokens (descending), then alphabetically
        token_results.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
        });

        token_results
            .into_iter()
            .take(self.config.max_results)
            .map(|(_, cat)| cat.clone())
            .collect()
    }
}

// Advanced token search with phrase matching
impl CategorySearcher {
    pub fn search_advanced_tokens(&self, query: &str) -> Vec<String> {
        if query.len() < self.config.min_query_length {
            return self
                .categories
                .iter()
                .take(self.config.max_results)
                .cloned()
                .collect();
        }

        let query_lower = query.to_lowercase();
        let query_tokens: Vec<String> = Self::tokenize_string(&query_lower).into_iter().collect();

        let mut scored_results: Vec<(i32, &String)> = Vec::new();

        for (index, category) in self.categories.iter().enumerate() {
            let category_lower = category.to_lowercase();
            let mut score = 0;

            // Phrase matching bonus (if all tokens appear in order)
            if self.is_phrase_match(&category_lower, &query_tokens) {
                score += self.config.token_match_weight + 20;
            }

            // Token order preservation bonus
            if self.preserves_token_order(&category_lower, &query_tokens) {
                score += self.config.token_match_weight + 10;
            }

            // Individual token matches
            let matching_tokens = self.category_tokens[index]
                .intersection(&query_tokens.iter().cloned().collect())
                .count();

            if matching_tokens > 0 {
                let token_ratio = matching_tokens as f32 / query_tokens.len() as f32;
                score += (token_ratio * self.config.token_match_weight as f32) as i32;
            }

            if score > 0 {
                scored_results.push((score, category));
            }
        }

        scored_results.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
        });

        scored_results
            .into_iter()
            .take(self.config.max_results)
            .map(|(_, cat)| cat.clone())
            .collect()
    }

    fn is_phrase_match(&self, text: &str, tokens: &[String]) -> bool {
        let text_lower = text.to_lowercase();
        let mut last_pos = 0;

        for token in tokens {
            if let Some(pos) = text_lower[last_pos..].find(token) {
                last_pos += pos + token.len();
            } else {
                return false;
            }
        }
        true
    }

    fn preserves_token_order(&self, text: &str, tokens: &[String]) -> bool {
        let text_lower = text.to_lowercase();
        let positions: Vec<usize> = tokens
            .iter()
            .filter_map(|token| text_lower.find(token))
            .collect();

        // Check if we found all tokens and they're in order
        positions.len() == tokens.len() && positions.windows(2).all(|w| w[0] <= w[1])
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_search() {
        let categories = vec![
            "Electronics & Computers".to_string(),
            "Home Appliances".to_string(),
            "Books & Media".to_string(),
            "Clothing & Apparel".to_string(),
            "Sports Equipment".to_string(),
            "Computer Accessories".to_string(),
            "Home Entertainment".to_string(),
            "Mobile Phones".to_string(),
        ];

        let searcher = CategorySearcher::new(categories);

        assert_eq!(
            searcher.search("comp"),
            vec!["Computer Accessories", "Electronics & Computers"],
        );
        assert_eq!(searcher.search("home app"), vec!["Home Appliances"]);
        assert_eq!(searcher.search("phone"), vec!["Mobile Phones"]);
    }

    #[test]
    fn test_token_only_search() {
        let categories = vec![
            "Electronics & Computers".to_string(),
            "Home Appliances".to_string(),
            "Books & Media".to_string(),
        ];

        let searcher = CategorySearcher::new(categories);

        assert_eq!(
            searcher.search_tokens_only("comp"),
            vec!["Electronics & Computers"]
        );
        assert_eq!(
            searcher.search_tokens_only("home app"),
            vec!["Home Appliances"]
        );
    }

    #[test]
    fn test_advanced_token_search() {
        let categories = vec![
            "Electronics & Computers".to_string(),
            "Computer Accessories".to_string(),
            "Home Entertainment".to_string(),
        ];

        let searcher = CategorySearcher::new(categories);

        assert_eq!(
            searcher.search_advanced_tokens("comp acc"),
            vec!["Computer Accessories"]
        );
    }

    #[test]
    fn test_custom_config_search() {
        let categories = vec![
            "Electronics & Computers".to_string(),
            "Home Appliances".to_string(),
            "Books & Media".to_string(),
        ];

        let custom_config = SearchConfig {
            max_results: 5,
            min_query_length: 2,
            enable_fuzzy: true,
            enable_caching: false,
            enable_token_search: true,
            token_match_weight: 90,
            prefix_match_weight: 100,
            substring_match_weight: 40,
            fuzzy_match_weight: 20,
        };

        let custom_searcher = CategorySearcher::new(categories).with_config(custom_config);

        assert_eq!(custom_searcher.search("home"), vec!["Home Appliances"]);
    }

    #[test]
    fn test_token_based_search() {
        let categories = vec![
            "Electronics & Computers".to_string(),
            "Home Appliances".to_string(),
            "Books & Media".to_string(),
        ];

        let searcher = CategorySearcher::new(categories);

        // Token search should match across word boundaries
        let results = searcher.search("elec comp");
        assert!(!results.is_empty());
        assert!(results[0].contains("Electronics"));

        // Partial token matches
        let results = searcher.search("applia");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_phrase_matching() {
        let categories = vec![
            "Computer Science Books".to_string(),
            "Science Fiction Books".to_string(),
            "Computer Accessories".to_string(),
        ];

        let searcher = CategorySearcher::new(categories);

        let results = searcher.search_advanced_tokens("computer science");
        assert_eq!(results[0], "Computer Science Books");
    }
}
