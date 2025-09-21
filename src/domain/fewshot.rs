use itertools::Itertools;
pub struct FewShotExample {
    pub input: String,
    pub output: String,
    pub category: String,
    pub quality_score: f32,
}

pub fn get_examples() -> Vec<FewShotExample> {
    vec![
        FewShotExample {
            input: "Write a function to calculate factorial".to_string(),
            output: "```rust\nfn factorial(n: u32) -> u32 {\n    if n == 0 {\n        1\n    } else {\n        n * factorial(n - 1)\n    }\n}\n```".to_string(),
            category: "code".to_string(),
            quality_score: 0.9,
        },
        FewShotExample {
            input: "Explain what a loop is in programming".to_string(),
            output: "A loop is a control structure that allows code to be executed repeatedly based on a condition. There are different types of loops like for loops, while loops, and do-while loops.".to_string(),
            category: "explanation".to_string(),
            quality_score: 0.8,
        },
        FewShotExample {
            input: "Write a simple hello world program".to_string(),
            output: "```python\nprint('Hello, World!')\n```".to_string(),
            category: "code".to_string(),
            quality_score: 0.95,
        },
        FewShotExample {
            input: "What is machine learning?".to_string(),
            output: "Machine learning is a subset of artificial intelligence that enables computers to learn and make decisions from data without being explicitly programmed for every scenario.".to_string(),
            category: "definition".to_string(),
            quality_score: 0.85,
        },
    ]
}

pub fn select_examples(category: &str, limit: usize) -> Vec<FewShotExample> {
    let examples = get_examples();
    examples
        .into_iter()
        .filter(|ex| ex.category == category)
        .sorted_by(|a, b| b.quality_score.partial_cmp(&a.quality_score).unwrap_or(std::cmp::Ordering::Equal))
        .take(limit)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_examples() {
        let examples = get_examples();
        assert!(!examples.is_empty());
        assert_eq!(examples.len(), 4);
    }

    #[test]
    fn test_select_examples_by_quality() {
        let examples = select_examples("code", 3);
        assert_eq!(examples.len(), 2); // Only 2 code examples exist

        // Verify examples are sorted by quality score (highest first)
        if examples.len() > 1 {
            assert!(examples[0].quality_score >= examples[1].quality_score);
        }

        // Verify highest quality example is selected first
        assert_eq!(examples[0].input, "Write a simple hello world program"); // 0.95 quality
        assert_eq!(examples[1].input, "Write a function to calculate factorial"); // 0.9 quality
    }
}
