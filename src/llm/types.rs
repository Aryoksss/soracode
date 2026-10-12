//! Wire types exchanged with an OpenAI-compatible endpoint.

use serde::{Deserialize, Serialize};

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    /// System prompt.
    System {
        /// Prompt text.
        content: String,
    },
    /// User turn.
    User {
        /// Turn text.
        content: String,
    },
    /// Assistant turn, optionally carrying tool calls.
    Assistant {
        /// Text content (may be empty when only calling tools).
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        /// Pending tool calls requested by the model.
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<ToolCall>>,
    },
    /// Result of a tool call, correlated by id.
    Tool {
        /// The `tool_call_id` this responds to.
        tool_call_id: String,
        /// Tool output text.
        content: String,
    },
}

/// A tool invocation requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Provider-assigned id, used to correlate the result.
    pub id: String,
    /// Always `function` for our tools.
    #[serde(rename = "type")]
    pub kind: String,
    /// The function call payload.
    pub function: FunctionCall,
}

/// Name + raw JSON arguments of a requested call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Tool name.
    pub name: String,
    /// JSON-encoded arguments, as produced by the model.
    pub arguments: String,
}

/// JSON-schema description handed to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    /// `function` — kept for wire compatibility.
    #[serde(rename = "type")]
    pub kind: String,
    /// The function description.
    pub function: FunctionSpec,
}

/// Name, description and parameters of an exposed tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSpec {
    /// Tool name the model must call.
    pub name: String,
    /// Human/model readable purpose.
    pub description: String,
    /// JSON Schema for the arguments object.
    pub parameters: serde_json::Value,
}

impl ToolSpec {
    /// Build a spec from name, description and a JSON Schema.
    #[must_use]
    pub fn new(name: &str, description: &str, parameters: serde_json::Value) -> Self {
        Self {
            kind: "function".into(),
            function: FunctionSpec {
                name: name.into(),
                description: description.into(),
                parameters,
            },
        }
    }
}

/// One incremental event emitted while streaming a response.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Visible assistant text.
    Text(String),
    /// Chain-of-thought text, if the provider exposes it.
    Reasoning(String),
    /// Token usage, emitted once near the end of the stream.
    Usage {
        /// Prompt tokens billed for this turn.
        prompt: u64,
        /// Completion tokens billed for this turn.
        completion: u64,
    },
}

/// A single SSE data frame from an OpenAI-compatible stream.
#[derive(Deserialize)]
pub(super) struct StreamChunk {
    #[serde(default)]
    pub(super) choices: Vec<StreamChoice>,
    #[serde(default)]
    pub(super) usage: Option<ChunkUsage>,
}

#[derive(Deserialize)]
pub(super) struct ChunkUsage {
    #[serde(default)]
    pub(super) prompt_tokens: u64,
    #[serde(default)]
    pub(super) completion_tokens: u64,
}

#[derive(Deserialize)]
pub(super) struct StreamChoice {
    #[serde(default)]
    pub(super) delta: Delta,
    #[serde(default)]
    pub(super) finish_reason: Option<String>,
}

#[derive(Deserialize, Default)]
pub(super) struct Delta {
    #[serde(default)]
    pub(super) content: Option<String>,
    #[serde(default)]
    pub(super) reasoning_content: Option<String>,
    #[serde(default)]
    pub(super) tool_calls: Option<Vec<DeltaToolCall>>,
}

#[derive(Deserialize)]
pub(super) struct DeltaToolCall {
    #[serde(default)]
    pub(super) index: usize,
    #[serde(default)]
    pub(super) id: Option<String>,
    #[serde(default)]
    pub(super) function: Option<DeltaFunction>,
}

#[derive(Deserialize)]
pub(super) struct DeltaFunction {
    #[serde(default)]
    pub(super) name: Option<String>,
    #[serde(default)]
    pub(super) arguments: Option<String>,
}
