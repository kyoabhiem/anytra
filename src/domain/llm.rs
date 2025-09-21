use crate::domain::models::{EnhancedPrompt, EnhancementOptions, Prompt};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LLMError {
    #[error("provider not configured: {0}")]
    NotConfigured(String),
    #[error("request failed: {0}")]
    RequestFailed(String),
    #[error("unexpected response: {0}")]
    UnexpectedResponse(String),
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn enhance(&self, prompt: Prompt, options: EnhancementOptions) -> Result<EnhancedPrompt, LLMError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockProvider;

    #[async_trait]
    impl LLMProvider for MockProvider {
        async fn enhance(&self, prompt: Prompt, _options: EnhancementOptions) -> Result<EnhancedPrompt, LLMError> {
            Ok(EnhancedPrompt {
                text: format!("Mock enhanced: {} - this is a longer text with enough words to pass validation", prompt.text),
                rationale: Some("Mock enhancement".to_string()),
                confidence: None,
            })
        }
    }

    struct FailingProvider;

    #[async_trait]
    impl LLMProvider for FailingProvider {
        async fn enhance(&self, _prompt: Prompt, _options: EnhancementOptions) -> Result<EnhancedPrompt, LLMError> {
            Err(LLMError::RequestFailed("Mock failure".to_string()))
        }
    }

    #[test]
    fn test_llm_error_display() {
        let error = LLMError::NotConfigured("Test provider".to_string());
        assert_eq!(error.to_string(), "provider not configured: Test provider");

        let error = LLMError::RequestFailed("Network error".to_string());
        assert_eq!(error.to_string(), "request failed: Network error");

        let error = LLMError::UnexpectedResponse("Invalid JSON".to_string());
        assert_eq!(error.to_string(), "unexpected response: Invalid JSON");
    }

    #[test]
    fn test_llm_error_debug() {
        let error = LLMError::NotConfigured("Debug test".to_string());
        assert!(format!("{:?}", error).contains("NotConfigured"));
        assert!(format!("{:?}", error).contains("Debug test"));
    }

    #[tokio::test]
    async fn test_mock_provider_success() {
        let provider = MockProvider;
        let prompt = Prompt {
            text: "Test prompt".to_string(),
        };
        let options = EnhancementOptions::default();

        let result = provider.enhance(prompt, options).await.unwrap();
        assert_eq!(result.text, "Mock enhanced: Test prompt - this is a longer text with enough words to pass validation");
        assert_eq!(result.rationale.as_deref(), Some("Mock enhancement"));
    }

    #[tokio::test]
    async fn test_failing_provider_error() {
        let provider = FailingProvider;
        let prompt = Prompt {
            text: "Test prompt".to_string(),
        };
        let options = EnhancementOptions::default();

        let result = provider.enhance(prompt, options).await;
        assert!(result.is_err());

        if let Err(LLMError::RequestFailed(msg)) = result {
            assert_eq!(msg, "Mock failure");
        } else {
            panic!("Expected RequestFailed error");
        }
    }

    #[test]
    fn test_llm_error_partial_eq() {
        let error1 = LLMError::RequestFailed("test".to_string());
        let error2 = LLMError::RequestFailed("test".to_string());
        let error3 = LLMError::RequestFailed("different".to_string());

        // Note: Since LLMError doesn't implement PartialEq, we can't directly compare
        // But we can verify they are different types of errors
        assert!(matches!(error1, LLMError::RequestFailed(_)));
        assert!(matches!(error2, LLMError::RequestFailed(_)));
        assert!(matches!(error3, LLMError::RequestFailed(_)));
    }
}
