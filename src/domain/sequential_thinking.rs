use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Result, Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtData {
    pub thought: String,
    pub thought_number: u32,
    pub total_thoughts: u32,
    pub is_revision: Option<bool>,
    pub revises_thought: Option<u32>,
    pub branch_from_thought: Option<u32>,
    pub branch_id: Option<String>,
    pub needs_more_thoughts: Option<bool>,
    pub next_thought_needed: bool,
}

impl ThoughtData {
    pub fn new(
        thought: String,
        thought_number: u32,
        total_thoughts: u32,
        next_thought_needed: bool,
    ) -> Self {
        Self {
            thought,
            thought_number,
            total_thoughts,
            is_revision: None,
            revises_thought: None,
            branch_from_thought: None,
            branch_id: None,
            needs_more_thoughts: None,
            next_thought_needed,
        }
    }

    pub fn with_revision(mut self, revises_thought: u32) -> Self {
        self.is_revision = Some(true);
        self.revises_thought = Some(revises_thought);
        self
    }

    pub fn with_branch(mut self, branch_from_thought: u32, branch_id: String) -> Self {
        self.branch_from_thought = Some(branch_from_thought);
        self.branch_id = Some(branch_id);
        self
    }
}

pub struct SequentialThinking {
    thought_history: Vec<ThoughtData>,
    branches: HashMap<String, Vec<ThoughtData>>,
}

impl SequentialThinking {
    pub fn new() -> Self {
        Self {
            thought_history: Vec::new(),
            branches: HashMap::new(),
        }
    }

    pub fn validate_thought_data(&self, input: &serde_json::Value) -> Result<ThoughtData> {
        let data = input.as_object()
            .ok_or_else(|| Error::msg("Input must be a JSON object"))?;

        let thought = data.get("thought")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::msg("Invalid thought: must be a string"))?
            .to_string();

        let thought_number = data.get("thoughtNumber")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| Error::msg("Invalid thoughtNumber: must be a number"))? as u32;

        let total_thoughts = data.get("totalThoughts")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| Error::msg("Invalid totalThoughts: must be a number"))? as u32;

        let next_thought_needed = data.get("nextThoughtNeeded")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| Error::msg("Invalid nextThoughtNeeded: must be a boolean"))?;

        let branch_from_thought = data.get("branchFromThought")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let branch_id = data.get("branchId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut thought_data = ThoughtData::new(thought, thought_number, total_thoughts, next_thought_needed);

        if let (Some(branch_from), Some(branch_id)) = (branch_from_thought, branch_id) {
            thought_data = thought_data.with_branch(branch_from, branch_id);
        }

        Ok(thought_data)
    }

    pub fn format_thought(&self, thought_data: &ThoughtData) -> String {
        let prefix = if thought_data.is_revision.unwrap_or(false) {
            "🔄 Revision"
        } else if thought_data.branch_from_thought.is_some() {
            "🌿 Branch"
        } else {
            "💭 Thought"
        };

        let context = if let Some(revises) = thought_data.revises_thought {
            format!(" (revising thought {})", revises)
        } else if let Some(branch_from) = thought_data.branch_from_thought {
            format!(" (from thought {}, ID: {:?})", branch_from, thought_data.branch_id)
        } else {
            String::new()
        };

        let header = format!("{} {}/{}{}", prefix, thought_data.thought_number, thought_data.total_thoughts, context);

        format!("{}\n{}", header, thought_data.thought)
    }

    pub fn process_thought(&mut self, input: serde_json::Value) -> Result<serde_json::Value> {
        let mut thought_data = self.validate_thought_data(&input)?;

        if thought_data.thought_number > thought_data.total_thoughts {
            thought_data.total_thoughts = thought_data.thought_number;
        }

        self.thought_history.push(thought_data.clone());

        if let (Some(_branch_from), Some(branch_id)) = (thought_data.branch_from_thought, &thought_data.branch_id) {
            self.branches.entry(branch_id.clone()).or_insert_with(Vec::new).push(thought_data.clone());
        }

        let formatted_thought = self.format_thought(&thought_data);
        eprintln!("{}", formatted_thought);

        let response = serde_json::json!({
            "thoughtNumber": thought_data.thought_number,
            "totalThoughts": thought_data.total_thoughts,
            "nextThoughtNeeded": thought_data.next_thought_needed,
            "branches": self.branches.keys().collect::<Vec<_>>(),
            "thoughtHistoryLength": self.thought_history.len()
        });

        Ok(response)
    }

    pub fn get_thought_history(&self) -> &[ThoughtData] {
        &self.thought_history
    }

    pub fn get_branches(&self) -> &HashMap<String, Vec<ThoughtData>> {
        &self.branches
    }
}

impl Default for SequentialThinking {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_thought_data_creation() {
        let thought = ThoughtData::new("Test thought".to_string(), 1, 3, true);
        assert_eq!(thought.thought, "Test thought");
        assert_eq!(thought.thought_number, 1);
        assert_eq!(thought.total_thoughts, 3);
        assert!(thought.next_thought_needed);
        assert!(thought.is_revision.is_none());
    }

    #[test]
    fn test_thought_data_with_revision() {
        let thought = ThoughtData::new("Revised thought".to_string(), 2, 3, false)
            .with_revision(1);
        assert!(thought.is_revision.unwrap());
        assert_eq!(thought.revises_thought, Some(1));
    }

    #[test]
    fn test_thought_data_with_branch() {
        let thought = ThoughtData::new("Branch thought".to_string(), 3, 3, false)
            .with_branch(2, "branch1".to_string());
        assert_eq!(thought.branch_from_thought, Some(2));
        assert_eq!(thought.branch_id, Some("branch1".to_string()));
    }

    #[test]
    fn test_sequential_thinking_creation() {
        let st = SequentialThinking::new();
        assert!(st.thought_history.is_empty());
        assert!(st.branches.is_empty());
    }

    #[test]
    fn test_validate_thought_data_valid() {
        let st = SequentialThinking::new();
        let input = json!({
            "thought": "Valid thought",
            "thoughtNumber": 1,
            "totalThoughts": 3,
            "nextThoughtNeeded": true
        });

        let result = st.validate_thought_data(&input);
        assert!(result.is_ok());
        let thought = result.unwrap();
        assert_eq!(thought.thought, "Valid thought");
        assert_eq!(thought.thought_number, 1);
        assert_eq!(thought.total_thoughts, 3);
        assert!(thought.next_thought_needed);
    }

    #[test]
    fn test_validate_thought_data_invalid() {
        let st = SequentialThinking::new();
        let input = json!({
            "thought": "Test",
            "thoughtNumber": "invalid",
            "totalThoughts": 3,
            "nextThoughtNeeded": true
        });

        let result = st.validate_thought_data(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid thoughtNumber"));
    }

    #[test]
    fn test_format_thought() {
        let st = SequentialThinking::new();
        let thought = ThoughtData::new("Test thought".to_string(), 1, 3, true);

        let formatted = st.format_thought(&thought);
        assert!(formatted.contains("💭 Thought"));
        assert!(formatted.contains("1/3"));
        assert!(formatted.contains("Test thought"));
    }

    #[test]
    fn test_format_revision_thought() {
        let st = SequentialThinking::new();
        let thought = ThoughtData::new("Revised".to_string(), 2, 3, false)
            .with_revision(1);

        let formatted = st.format_thought(&thought);
        assert!(formatted.contains("🔄 Revision"));
        assert!(formatted.contains("(revising thought 1)"));
    }

    #[test]
    fn test_process_thought() {
        let mut st = SequentialThinking::new();
        let input = json!({
            "thought": "Process this thought",
            "thoughtNumber": 1,
            "totalThoughts": 3,
            "nextThoughtNeeded": true
        });

        let result = st.process_thought(input);
        assert!(result.is_ok());
        assert_eq!(st.thought_history.len(), 1);
        assert_eq!(st.thought_history[0].thought, "Process this thought");
    }

    #[test]
    fn test_process_thought_with_branch() {
        let mut st = SequentialThinking::new();
        let input = json!({
            "thought": "Branch thought",
            "thoughtNumber": 2,
            "totalThoughts": 3,
            "nextThoughtNeeded": false,
            "branchFromThought": 1,
            "branchId": "test_branch"
        });

        let result = st.process_thought(input);
        assert!(result.is_ok());
        assert_eq!(st.thought_history.len(), 1);
        assert!(st.branches.contains_key("test_branch"));
        assert_eq!(st.branches["test_branch"].len(), 1);
    }

    #[test]
    fn test_process_thought_invalid() {
        let mut st = SequentialThinking::new();
        let input = json!({
            "thought": "Test",
            "thoughtNumber": "invalid",
            "totalThoughts": 3,
            "nextThoughtNeeded": true
        });

        let result = st.process_thought(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid thoughtNumber"));
    }
}
