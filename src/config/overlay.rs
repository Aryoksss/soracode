//! Field-by-field overlays for the flat `[llm]` and `[agent]` blocks.

use super::types::default_workspace;
use super::{AgentConfig, LlmConfig};

impl LlmConfig {
    /// Overlay a partial `[llm]` block, treating empty strings as absent.
    pub fn overlay(&mut self, o: &Self) {
        if !o.base_url.is_empty() {
            self.base_url.clone_from(&o.base_url);
        }
        if !o.api_key.is_empty() {
            self.api_key.clone_from(&o.api_key);
        }
        if !o.model.is_empty() {
            self.model.clone_from(&o.model);
        }
        if o.max_tokens != 0 {
            self.max_tokens = o.max_tokens;
        }
        if o.temperature != 0.0 {
            self.temperature = o.temperature;
        }
        if o.timeout_secs != 0 {
            self.timeout_secs = o.timeout_secs;
        }
        if o.max_attempts != 0 {
            self.max_attempts = o.max_attempts;
        }
        if o.retry_backoff_ms != 0 {
            self.retry_backoff_ms = o.retry_backoff_ms;
        }
        self.headers.extend(o.headers.clone());
    }
}

impl AgentConfig {
    /// Overlay a partial `[agent]` block, treating sentinel values as absent.
    pub fn overlay(&mut self, o: &Self) {
        if o.max_turns != 0 {
            self.max_turns = o.max_turns;
        }
        if o.max_seconds != 0 {
            self.max_seconds = o.max_seconds;
        }
        if o.workspace != default_workspace() {
            self.workspace.clone_from(&o.workspace);
        }
        if o.system_prompt_file.is_some() {
            self.system_prompt_file.clone_from(&o.system_prompt_file);
        }
        if o.subagents_enabled {
            self.subagents_enabled = true;
        }
        if o.subagent_max_depth != 0 {
            self.subagent_max_depth = o.subagent_max_depth;
        }
        if o.subagent_max_parallel != 0 {
            self.subagent_max_parallel = o.subagent_max_parallel;
        }
        if o.tool_timeout_secs != 0 {
            self.tool_timeout_secs = o.tool_timeout_secs;
        }
        if o.tool_output_limit != 0 {
            self.tool_output_limit = o.tool_output_limit;
        }
        if o.context_budget != 0 {
            self.context_budget = o.context_budget;
        }
    }
}
