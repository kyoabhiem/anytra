use anyhow::{self, Result};
use crate::domain::models::EnhancedPrompt;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum ValidationError {
    EmptyPrompt,
    TooShort,
    TooLong,
    InappropriateContent(String),
    TooSimple,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::EmptyPrompt => write!(f, "Enhanced prompt is empty"),
            ValidationError::TooShort => write!(f, "Enhanced prompt is too short"),
            ValidationError::TooLong => write!(f, "Enhanced prompt is too long"),
            ValidationError::InappropriateContent(word) => write!(f, "Inappropriate content detected: {}", word),
            ValidationError::TooSimple => write!(f, "Enhanced prompt is too simple"),
        }
    }
}

impl std::error::Error for ValidationError {}

pub fn validate_enhanced_prompt(prompt: &EnhancedPrompt) -> Result<(), ValidationError> {
    if prompt.text.trim().is_empty() {
        return Err(ValidationError::EmptyPrompt);
    }

    let len = prompt.text.len();
    if len < 10 {
        return Err(ValidationError::TooShort);
    }
    if len > 5000 {
        return Err(ValidationError::TooLong);
    }

    // Word count check
    let word_count = prompt.text.split_whitespace().count();
    if word_count < 10 {
        return Err(ValidationError::TooSimple);
    }

    // Inappropriate content check (placeholder)
    let bad_words = ["inappropriate", "offensive"];
    for word in bad_words {
        if prompt.text.to_lowercase().contains(word) {
            return Err(ValidationError::InappropriateContent(word.to_string()));
        }
    }

    Ok(())
}

pub fn compute_confidence(prompt: &EnhancedPrompt) -> f32 {
    let len_score = (prompt.text.len() as f32 / 1000.0).min(1.0);
    let word_score = (word_count(&prompt.text) as f32 / 50.0).min(1.0);
    (len_score + word_score) / 2.0
}

#[allow(dead_code)]
pub fn check_grammar_and_clarity(text: &str) -> Vec<String> {
    let mut issues = Vec::new();
    // Check for double spaces
    if text.contains("  ") {
        issues.push("Contains double spaces".to_string());
    }
    // Check for lines ending with space
    if text.lines().any(|line| line.trim_end().ends_with(' ')) {
        issues.push("Some lines end with space".to_string());
    }
    // Check sentence length for clarity
    let sentences: Vec<&str> = text.split(|c| c == '.' || c == '!' || c == '?').collect();
    let avg_length = sentences.iter().map(|s| s.len()).sum::<usize>() / sentences.len().max(1);
    if avg_length > 100 {
        issues.push("Average sentence length is too long (>100 chars), may affect clarity".to_string());
    }
    if avg_length < 10 {
        issues.push("Average sentence length is too short (<10 chars), may be too choppy".to_string());
    }
    issues
}

#[allow(dead_code)]
pub fn check_consistency(text: &str) -> Vec<String> {
    let mut issues = Vec::new();
    // Check for duplicate sentences
    let sentences: Vec<&str> = text.split(|c| c == '.' || c == '!' || c == '?').collect();
    let mut seen = HashSet::new();
    for sentence in sentences {
        let trimmed = sentence.trim();
        if !trimmed.is_empty() && !seen.insert(trimmed.to_lowercase()) {
            issues.push("Duplicate sentence found".to_string());
            break;
        }
    }
    issues
}

#[allow(dead_code)]
pub fn check_formatting(text: &str) -> Vec<String> {
    let mut issues = Vec::new();
    // Check for inconsistent spacing around punctuation
    if text.contains(" ,") || text.contains(" .") || text.contains(" !") || text.contains(" ?") {
        issues.push("Inconsistent spacing around punctuation".to_string());
    }
    // Check for missing spaces after punctuation
    if text.contains("..") || text.contains("!!") || text.contains("??") {
        issues.push("Missing spaces after punctuation".to_string());
    }
    issues
}

#[allow(dead_code)]
pub fn track_quality_metrics(text: &str, confidence: f32, issues: &[String]) {
    // Simple logging
    println!("Quality metrics - text length: {}, confidence: {}, issues: {}", text.len(), confidence, issues.len());
}

fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::EnhancedPrompt;

    #[test]
    fn test_valid_prompt() {
        let prompt = EnhancedPrompt {
            text: "This is a valid enhanced prompt with enough length and words to pass validation.".to_string(),
            rationale: None,
            confidence: None,
        };
        assert!(validate_enhanced_prompt(&prompt).is_ok());
    }

    #[test]
    fn test_empty_prompt() {
        let prompt = EnhancedPrompt {
            text: "".to_string(),
            rationale: None,
            confidence: None,
        };
        assert!(matches!(validate_enhanced_prompt(&prompt), Err(ValidationError::EmptyPrompt)));
    }

    #[test]
    fn test_too_short() {
        let prompt = EnhancedPrompt {
            text: "Short".to_string(),
            rationale: None,
            confidence: None,
        };
        assert!(matches!(validate_enhanced_prompt(&prompt), Err(ValidationError::TooShort)));
    }

    #[test]
    fn test_too_long() {
        let text = "a".repeat(5001);
        let prompt = EnhancedPrompt {
            text,
            rationale: None,
            confidence: None,
        };
        assert!(matches!(validate_enhanced_prompt(&prompt), Err(ValidationError::TooLong)));
    }

    #[test]
    fn test_too_simple() {
        let prompt = EnhancedPrompt {
            text: "Short text".to_string(),
            rationale: None,
            confidence: None,
        };
        assert!(matches!(validate_enhanced_prompt(&prompt), Err(ValidationError::TooSimple)));
    }

    #[test]
    fn test_inappropriate_content() {
        let prompt = EnhancedPrompt {
            text: "This is a long prompt that contains inappropriate content and has enough words.".to_string(),
            rationale: None,
            confidence: None,
        };
        assert!(matches!(validate_enhanced_prompt(&prompt), Err(ValidationError::InappropriateContent(_))));
    }

    #[test]
    fn test_confidence_score() {
        let prompt = EnhancedPrompt {
            text: "This is a test prompt with some words to compute confidence score.".to_string(),
            rationale: None,
            confidence: None,
        };
        let score = compute_confidence(&prompt);
        assert!(score > 0.0 && score <= 1.0);
    }

    #[test]
    fn test_grammar_and_clarity_good() {
        let text = "This is a good sentence. It has proper length.";
        let issues = check_grammar_and_clarity(text);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_grammar_and_clarity_double_spaces() {
        let text = "This has  double spaces.";
        let issues = check_grammar_and_clarity(text);
        assert!(issues.contains(&"Contains double spaces".to_string()));
    }

    #[test]
    fn test_grammar_and_clarity_long_sentences() {
        let text = "This is an extremely long sentence that definitely exceeds the average length limit for clarity purposes in automated validation checks and should trigger the too long condition because it contains more than two hundred characters in total length to ensure the test passes correctly.";
        let issues = check_grammar_and_clarity(text);
        assert!(issues.iter().any(|i| i.contains("too long")));
    }

    #[test]
    fn test_grammar_and_clarity_short_sentences() {
        let text = "Short. Very short.";
        let issues = check_grammar_and_clarity(text);
        assert!(issues.iter().any(|i| i.contains("too short")));
    }

    #[test]
    fn test_consistency_good() {
        let text = "This is a unique sentence. Another unique sentence.";
        let issues = check_consistency(text);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_consistency_duplicate() {
        let text = "This is a sentence. This is a sentence.";
        let issues = check_consistency(text);
        assert!(issues.contains(&"Duplicate sentence found".to_string()));
    }

    #[test]
    fn test_formatting_good() {
        let text = "This is good formatting.";
        let issues = check_formatting(text);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_formatting_bad_spacing() {
        let text = "This has bad , spacing.";
        let issues = check_formatting(text);
        assert!(issues.contains(&"Inconsistent spacing around punctuation".to_string()));
    }
}
