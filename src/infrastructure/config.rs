use std::env;

/// Centralized application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// OpenRouter API configuration
    pub openrouter: OpenRouterConfig,
    /// Sequential thinking configuration
    pub sequential_thinking: SequentialThinkingConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
}

/// OpenRouter API configuration
#[derive(Debug, Clone)]
pub struct OpenRouterConfig {
    pub api_key: String,
    pub model: String,
    pub referer: Option<String>,
    pub title: Option<String>,
}

/// Sequential thinking configuration
#[derive(Debug, Clone)]
pub struct SequentialThinkingConfig {
    pub default_enabled: bool,
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, String> {
        let openrouter = OpenRouterConfig::from_env()?;
        let sequential_thinking = SequentialThinkingConfig::from_env();
        let logging = LoggingConfig::from_env();

        Ok(Self {
            openrouter,
            sequential_thinking,
            logging,
        })
    }

    /// Get the default sequential thinking setting
    pub fn sequential_thinking_enabled(&self) -> bool {
        self.sequential_thinking.default_enabled
    }
}

impl OpenRouterConfig {
    /// Load OpenRouter configuration from environment variables
    pub fn from_env() -> Result<Self, String> {
        let api_key = env::var("OPENROUTER_API_KEY")
            .map_err(|_| "OPENROUTER_API_KEY environment variable is required")?;

        let model = env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "openrouter/auto".to_string());
        let referer = env::var("OPENROUTER_REFERER").ok();
        let title = env::var("OPENROUTER_TITLE").ok();

        Ok(Self {
            api_key,
            model,
            referer,
            title,
        })
    }
}

impl SequentialThinkingConfig {
    /// Load sequential thinking configuration from environment variables
    pub fn from_env() -> Self {
        let default_enabled = get_enable_sequential_thinking_default();

        Self {
            default_enabled,
        }
    }
}

impl LoggingConfig {
    /// Load logging configuration from environment variables
    pub fn from_env() -> Self {
        let level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        Self {
            level,
        }
    }
}

/// Get the default value for enable_sequential_thinking from environment variables
/// Returns true if ENABLE_SEQUENTIAL_THINKING is set to "true", "1", "yes", or "on" (case-insensitive)
/// Returns false if set to "false", "0", "no", or "off" (case-insensitive)
/// Returns true by default if not set or if the value is not recognized
pub fn get_enable_sequential_thinking_default() -> bool {
    match env::var("ENABLE_SEQUENTIAL_THINKING") {
        Ok(val) => {
            let val_lower = val.to_lowercase();
            match val_lower.as_str() {
                "true" | "1" | "yes" | "on" => true,
                "false" | "0" | "no" | "off" => false,
                _ => {
                    // Default to true for unrecognized values to maintain backwards compatibility
                    // while encouraging the new default behavior
                    true
                }
            }
        }
        Err(_) => true, // Default to true when not set
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn clear_env() {
        env::remove_var("OPENROUTER_API_KEY");
        env::remove_var("OPENROUTER_MODEL");
        env::remove_var("OPENROUTER_REFERER");
        env::remove_var("OPENROUTER_TITLE");
        env::remove_var("ENABLE_SEQUENTIAL_THINKING");
        env::remove_var("LOG_LEVEL");
    }

    fn set_env(vars: &[(&str, &str)]) {
        for (key, value) in vars {
            env::set_var(key, value);
        }
    }

    #[test]
    fn test_config_from_env_complete() {
        clear_env();
        set_env(&[
            ("OPENROUTER_API_KEY", "test-api-key"),
            ("OPENROUTER_MODEL", "test-model"),
            ("OPENROUTER_REFERER", "test-referer"),
            ("OPENROUTER_TITLE", "test-title"),
            ("ENABLE_SEQUENTIAL_THINKING", "false"),
            ("LOG_LEVEL", "debug"),
        ]);

        let config = Config::from_env().unwrap();

        assert_eq!(config.openrouter.api_key, "test-api-key");
        assert_eq!(config.openrouter.model, "test-model");
        assert_eq!(config.openrouter.referer.as_deref(), Some("test-referer"));
        assert_eq!(config.openrouter.title.as_deref(), Some("test-title"));
        assert_eq!(config.sequential_thinking.default_enabled, false);
        assert_eq!(config.logging.level, "debug");
    }

    #[test]
    fn test_config_from_env_minimal() {
        clear_env();
        set_env(&[("OPENROUTER_API_KEY", "test-api-key")]);

        let config = Config::from_env().unwrap();

        assert_eq!(config.openrouter.api_key, "test-api-key");
        assert_eq!(config.openrouter.model, "openrouter/auto");
        assert!(config.openrouter.referer.is_none());
        assert!(config.openrouter.title.is_none());
        assert_eq!(config.logging.level, "info"); // default
    }

    #[test]
    fn test_config_from_env_missing_api_key() {
        clear_env();

        let result = Config::from_env();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("OPENROUTER_API_KEY"));
    }

    #[test]
    fn test_openrouter_config_from_env() {
        clear_env();
        set_env(&[
            ("OPENROUTER_API_KEY", "test-key"),
            ("OPENROUTER_MODEL", "custom-model"),
            ("OPENROUTER_REFERER", "referer"),
            ("OPENROUTER_TITLE", "title"),
        ]);

        let config = OpenRouterConfig::from_env().unwrap();

        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.model, "custom-model");
        assert_eq!(config.referer.as_deref(), Some("referer"));
        assert_eq!(config.title.as_deref(), Some("title"));
    }

    #[test]
    fn test_sequential_thinking_config_from_env() {
        clear_env();

        // Test default (true)
        let config = SequentialThinkingConfig::from_env();
        assert_eq!(config.default_enabled, true);

        // Test explicit false
        set_env(&[("ENABLE_SEQUENTIAL_THINKING", "false")]);
        let config = SequentialThinkingConfig::from_env();
        assert_eq!(config.default_enabled, false);

        // Test explicit true
        set_env(&[("ENABLE_SEQUENTIAL_THINKING", "true")]);
        let config = SequentialThinkingConfig::from_env();
        assert_eq!(config.default_enabled, true);
    }

    #[test]
    fn test_logging_config_from_env() {
        clear_env();

        // Test default
        let config = LoggingConfig::from_env();
        assert_eq!(config.level, "info");

        // Test custom level
        set_env(&[("LOG_LEVEL", "debug")]);
        let config = LoggingConfig::from_env();
        assert_eq!(config.level, "debug");
    }

    #[test]
    fn test_get_enable_sequential_thinking_default_true_values() {
        clear_env();
        let true_values = ["true", "TRUE", "True", "1", "yes", "YES", "on", "ON"];
        for val in &true_values {
            env::set_var("ENABLE_SEQUENTIAL_THINKING", val);
            assert_eq!(get_enable_sequential_thinking_default(), true, "Value '{}' should return true", val);
        }
    }

    #[test]
    fn test_get_enable_sequential_thinking_default_false_values() {
        clear_env();
        let false_values = ["false", "FALSE", "False", "0", "no", "NO", "off", "OFF"];
        for val in &false_values {
            env::set_var("ENABLE_SEQUENTIAL_THINKING", val);
            assert_eq!(get_enable_sequential_thinking_default(), false, "Value '{}' should return false", val);
        }
    }

    #[test]
    fn test_get_enable_sequential_thinking_default_unrecognized() {
        clear_env();
        env::set_var("ENABLE_SEQUENTIAL_THINKING", "maybe");
        assert_eq!(get_enable_sequential_thinking_default(), true, "Unrecognized value should default to true");
    }

    #[test]
    fn test_get_enable_sequential_thinking_default_not_set() {
        clear_env();
        assert_eq!(get_enable_sequential_thinking_default(), true, "Should default to true when not set");
    }
}
