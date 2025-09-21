use crate::domain::llm::{LLMError, LLMProvider};
use crate::domain::models::{EnhancedPrompt, EnhancementOptions, Prompt};
use crate::domain::fewshot;
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use std::env;

pub struct OpenRouterClient {
    http: reqwest::Client,
    api_key: String,
    model: String,
    referer: Option<String>,
    title: Option<String>,
}

impl OpenRouterClient {
    pub fn from_env() -> Result<Self, LLMError> {
        let api_key = env::var("OPENROUTER_API_KEY")
            .map_err(|_| LLMError::NotConfigured("OPENROUTER_API_KEY missing".into()))?;
        let model = env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "openrouter/auto".into());
        let referer = env::var("OPENROUTER_REFERER").ok();
        let title = env::var("OPENROUTER_TITLE").ok();
        let http = reqwest::Client::builder()
            .user_agent("anytra/0.1")
            .build()
            .map_err(|e| LLMError::RequestFailed(e.to_string()))?;
        Ok(Self { http, api_key, model, referer, title })
    }
}

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: String,
}

#[async_trait]
impl LLMProvider for OpenRouterClient {
    async fn enhance(&self, prompt: Prompt, options: EnhancementOptions) -> Result<EnhancedPrompt, LLMError> {
        let system = "You are an expert prompt engineering assistant. Your ONLY task is to refine and enhance user prompts for Large Language Models. You must return ONLY the enhanced prompt text - no introductions, no explanations, no additional commentary of any kind. Simply output the improved prompt directly.

CRITICAL: Your response must contain ONLY the enhanced prompt. No prefixes like 'Enhanced prompt:' or 'Here is the enhanced version:'. No meta-commentary. No acknowledgments. Just the enhanced prompt text itself.

Guidelines for enhancement:
- Maximize clarity and specificity
- Specify clear goals and constraints
- Resolve ambiguities while staying faithful to original intent
- Structure the prompt for optimal LLM performance
- If a specific language is requested, write the entire enhanced prompt in that language

Remember: Output ONLY the enhanced prompt. Nothing else.";

        let mut instruction = String::new();
        if let Some(goal) = options.goal { instruction.push_str(&format!("Goal: {}\n", goal)); }
        if let Some(style) = options.style { instruction.push_str(&format!("Style: {}\n", style)); }
        if let Some(tone) = options.tone { instruction.push_str(&format!("Tone: {}\n", tone)); }
        if let Some(level) = options.level { instruction.push_str(&format!("Enhancement level: {} (1-5)\n", level)); }
        if let Some(audience) = options.audience { instruction.push_str(&format!("Audience: {}\n", audience)); }
        if let Some(language) = options.language { instruction.push_str(&format!("Language: {}\n", language)); }

        let mut user = if instruction.is_empty() {
            prompt.text.clone()
        } else {
            format!("{}\n\n---\nOriginal prompt:\n{}", instruction, prompt.text.clone())
        };

        // Add few-shot examples
        let category = if prompt.text.to_lowercase().contains("code") || prompt.text.to_lowercase().contains("function") || prompt.text.to_lowercase().contains("program") {
            "code"
        } else if prompt.text.to_lowercase().contains("explain") || prompt.text.to_lowercase().contains("what is") {
            "explanation"
        } else if prompt.text.to_lowercase().contains("define") || prompt.text.to_lowercase().contains("definition") {
            "definition"
        } else {
            "general"
        };

        let examples = fewshot::select_examples(&category, 2);
        if !examples.is_empty() {
            let examples_text = examples.iter().map(|ex| format!("Example Input: {}\nExample Output: {}", ex.input, ex.output)).collect::<Vec<_>>().join("\n\n");
            user = format!("Here are some examples to guide your response:\n\n{}\n\n{}", examples_text, user);
        }

        let payload = ChatRequest {
            model: &self.model,
            messages: vec![
                ChatMessage { role: "system", content: system },
                ChatMessage { role: "user", content: &user },
            ],
            temperature: 0.2,
        };

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", self.api_key)).map_err(|e| LLMError::RequestFailed(e.to_string()))?);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(ref referer) = self.referer {
            headers.insert("HTTP-Referer", HeaderValue::from_str(referer).map_err(|e| LLMError::RequestFailed(e.to_string()))?);
        }
        if let Some(ref title) = self.title {
            headers.insert("X-Title", HeaderValue::from_str(title).map_err(|e| LLMError::RequestFailed(e.to_string()))?);
        }

        const MAX_RETRIES: u32 = 3;
        let mut attempts = 0;

        let resp = loop {
            attempts += 1;
            match self.http.post("https://openrouter.ai/api/v1/chat/completions").headers(headers.clone()).json(&payload).send().await {
                Ok(r) => break r,
                Err(_e) => {
                    if attempts >= MAX_RETRIES {
                        // Graceful degradation: return a simple enhanced prompt
                        return Ok(EnhancedPrompt {
                            text: format!("Enhanced: {}", prompt.text),
                            rationale: Some("Fallback due to API failure after retries".to_string()),
                            confidence: Some(0.3),
                        });
                    }
                    let delay = Duration::from_millis(500 * 2u64.pow(attempts - 1));
                    sleep(delay).await;
                }
            }
        };

        if !resp.status().is_success() {
            return Err(LLMError::RequestFailed(format!("status {}", resp.status())));
        }

        let parsed: ChatResponse = resp.json().await.map_err(|e| LLMError::UnexpectedResponse(e.to_string()))?;
        let text = parsed
            .choices
            .get(0)
            .map(|c| c.message.content.trim().to_string())
            .ok_or_else(|| LLMError::UnexpectedResponse("no choices".into()))?;

        Ok(EnhancedPrompt { text, rationale: None, confidence: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn set_test_env() {
        env::set_var("OPENROUTER_API_KEY", "test-api-key");
        env::set_var("OPENROUTER_MODEL", "test-model");
        env::set_var("OPENROUTER_REFERER", "test-referer");
        env::set_var("OPENROUTER_TITLE", "test-title");
    }

    fn clear_test_env() {
        env::remove_var("OPENROUTER_API_KEY");
        env::remove_var("OPENROUTER_MODEL");
        env::remove_var("OPENROUTER_REFERER");
        env::remove_var("OPENROUTER_TITLE");
    }

    #[test]
    fn test_from_env_success() {
        // Save original env vars to restore later
        let original_api_key = env::var("OPENROUTER_API_KEY").ok();
        let original_model = env::var("OPENROUTER_MODEL").ok();
        let original_referer = env::var("OPENROUTER_REFERER").ok();
        let original_title = env::var("OPENROUTER_TITLE").ok();

        // Set all required environment variables
        env::set_var("OPENROUTER_API_KEY", "test-api-key");
        env::set_var("OPENROUTER_MODEL", "test-model");
        env::set_var("OPENROUTER_REFERER", "test-referer");
        env::set_var("OPENROUTER_TITLE", "test-title");

        let client = OpenRouterClient::from_env().unwrap();

        assert_eq!(client.api_key, "test-api-key");
        assert_eq!(client.model, "test-model");
        assert_eq!(client.referer.as_ref().unwrap(), "test-referer");
        assert_eq!(client.title.as_ref().unwrap(), "test-title");

        // Restore original environment
        if let Some(key) = original_api_key {
            env::set_var("OPENROUTER_API_KEY", key);
        } else {
            env::remove_var("OPENROUTER_API_KEY");
        }
        if let Some(model) = original_model {
            env::set_var("OPENROUTER_MODEL", model);
        } else {
            env::remove_var("OPENROUTER_MODEL");
        }
        if let Some(referer) = original_referer {
            env::set_var("OPENROUTER_REFERER", referer);
        } else {
            env::remove_var("OPENROUTER_REFERER");
        }
        if let Some(title) = original_title {
            env::set_var("OPENROUTER_TITLE", title);
        } else {
            env::remove_var("OPENROUTER_TITLE");
        }
    }

    #[test]
    fn test_from_env_missing_api_key() {
        // Save original env vars to restore later
        let original_api_key = env::var("OPENROUTER_API_KEY").ok();
        let original_model = env::var("OPENROUTER_MODEL").ok();
        let original_referer = env::var("OPENROUTER_REFERER").ok();
        let original_title = env::var("OPENROUTER_TITLE").ok();

        // Clear all environment variables
        env::remove_var("OPENROUTER_API_KEY");
        env::remove_var("OPENROUTER_MODEL");
        env::remove_var("OPENROUTER_REFERER");
        env::remove_var("OPENROUTER_TITLE");

        let result = OpenRouterClient::from_env();
        assert!(result.is_err());

        if let Err(LLMError::NotConfigured(msg)) = result {
            assert_eq!(msg, "OPENROUTER_API_KEY missing");
        } else {
            panic!("Expected NotConfigured error");
        }

        // Restore original environment
        if let Some(key) = original_api_key {
            env::set_var("OPENROUTER_API_KEY", key);
        }
        if let Some(model) = original_model {
            env::set_var("OPENROUTER_MODEL", model);
        }
        if let Some(referer) = original_referer {
            env::set_var("OPENROUTER_REFERER", referer);
        }
        if let Some(title) = original_title {
            env::set_var("OPENROUTER_TITLE", title);
        }
    }

    #[test]
    fn test_from_env_defaults() {
        // Save original env vars to restore later
        let original_api_key = env::var("OPENROUTER_API_KEY").ok();
        let original_model = env::var("OPENROUTER_MODEL").ok();
        let original_referer = env::var("OPENROUTER_REFERER").ok();
        let original_title = env::var("OPENROUTER_TITLE").ok();

        // Set only the API key to test defaults
        env::set_var("OPENROUTER_API_KEY", "test-api-key");
        // Don't set other vars to test defaults

        let client = OpenRouterClient::from_env().unwrap();

        assert_eq!(client.api_key, "test-api-key");
        assert_eq!(client.model, "openrouter/auto"); // Default model
        assert!(client.referer.is_none());
        assert!(client.title.is_none());

        // Restore original environment
        if let Some(key) = original_api_key {
            env::set_var("OPENROUTER_API_KEY", key);
        } else {
            env::remove_var("OPENROUTER_API_KEY");
        }
        if let Some(model) = original_model {
            env::set_var("OPENROUTER_MODEL", model);
        } else {
            env::remove_var("OPENROUTER_MODEL");
        }
        if let Some(referer) = original_referer {
            env::set_var("OPENROUTER_REFERER", referer);
        } else {
            env::remove_var("OPENROUTER_REFERER");
        }
        if let Some(title) = original_title {
            env::set_var("OPENROUTER_TITLE", title);
        } else {
            env::remove_var("OPENROUTER_TITLE");
        }
    }
}
