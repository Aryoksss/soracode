//! Minimal OpenAI-compatible chat client with SSE streaming.
//!
//! Only the surface soracode needs: streaming chat completions with tools.
//! Everything else (responses API, embeddings, vision) is intentionally absent
//! to keep the binary small.

use std::collections::BTreeMap;
use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

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

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [Message],
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<&'a [ToolSpec]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<&'a str>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(Deserialize)]
struct StreamChunk {
    #[serde(default)]
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    #[serde(default)]
    delta: Delta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize, Default)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<DeltaToolCall>>,
}

#[derive(Deserialize)]
struct DeltaToolCall {
    #[serde(default)]
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<DeltaFunction>,
}

#[derive(Deserialize)]
struct DeltaFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

/// One incremental event emitted while streaming a response.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Visible assistant text.
    Text(String),
    /// Chain-of-thought text, if the provider exposes it.
    Reasoning(String),
}

/// The assembled result of one model turn.
#[derive(Debug, Clone, Default)]
pub struct Completion {
    /// Visible assistant text.
    pub content: String,
    /// Reasoning text, if any.
    pub reasoning: String,
    /// Tool calls the model wants executed.
    pub tool_calls: Vec<ToolCall>,
    /// Provider finish reason.
    pub finish_reason: Option<String>,
}

/// Streaming-capable client for an OpenAI-compatible endpoint.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
    max_tokens: u32,
    temperature: f32,
    extra: BTreeMap<String, String>,
}

impl Client {
    /// Build a client from the given parameters.
    ///
    /// # Errors
    /// Returns [`Error::Llm`] if the HTTP client cannot be constructed.
    pub fn new(
        base_url: &str,
        api_key: &str,
        model: &str,
        max_tokens: u32,
        temperature: f32,
        timeout_secs: u64,
        extra: BTreeMap<String, String>,
    ) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs.max(1)))
            .connect_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(8)
            .build()
            .map_err(|e| Error::Llm(format!("http client: {e}")))?;

        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            max_tokens,
            temperature,
            extra,
        })
    }

    /// The model identifier in use.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Stream one assistant turn, invoking `on_event` for each incremental chunk.
    ///
    /// # Errors
    /// Returns [`Error::Llm`] on transport failure or a non-2xx response.
    pub async fn stream_chat<F>(
        &self,
        messages: &[Message],
        tools: &[ToolSpec],
        mut on_event: F,
    ) -> Result<Completion>
    where
        F: FnMut(StreamEvent) + Send,
    {
        let url = format!("{}/chat/completions", self.base_url);
        let body = ChatRequest {
            model: &self.model,
            messages,
            tools: if tools.is_empty() { None } else { Some(tools) },
            tool_choice: if tools.is_empty() { None } else { Some("auto") },
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            stream: true,
        };

        let mut req = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .json(&body);

        for (k, v) in &self.extra {
            req = req.header(k.as_str(), v.as_str());
        }

        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::Llm(format!(
                "HTTP {status} from {url}: {}",
                truncate(&text, 800)
            )));
        }

        let mut out = Completion::default();
        // Accumulate tool calls by index since providers stream fragments.
        let mut partial: BTreeMap<usize, (String, String, String)> = BTreeMap::new();
        let mut buf = String::new();
        let mut stream = resp.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            buf.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(pos) = buf.find('\n') {
                let line = buf[..pos].trim_end_matches('\r').to_string();
                buf.drain(..=pos);

                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data.is_empty() || data == "[DONE]" {
                    continue;
                }

                let Ok(parsed) = serde_json::from_str::<StreamChunk>(data) else {
                    continue;
                };

                for choice in parsed.choices {
                    if let Some(reason) = choice.finish_reason {
                        out.finish_reason = Some(reason);
                    }
                    if let Some(r) = choice.delta.reasoning_content {
                        if !r.is_empty() {
                            out.reasoning.push_str(&r);
                            on_event(StreamEvent::Reasoning(r));
                        }
                    }
                    if let Some(c) = choice.delta.content {
                        if !c.is_empty() {
                            out.content.push_str(&c);
                            on_event(StreamEvent::Text(c));
                        }
                    }
                    if let Some(calls) = choice.delta.tool_calls {
                        for c in calls {
                            let slot = partial.entry(c.index).or_default();
                            if let Some(id) = c.id {
                                if !id.is_empty() {
                                    slot.0 = id;
                                }
                            }
                            if let Some(f) = c.function {
                                if let Some(n) = f.name {
                                    slot.1.push_str(&n);
                                }
                                if let Some(a) = f.arguments {
                                    slot.2.push_str(&a);
                                }
                            }
                        }
                    }
                }
            }
        }

        for (idx, (id, name, args)) in partial {
            if name.is_empty() {
                continue;
            }
            out.tool_calls.push(ToolCall {
                id: if id.is_empty() {
                    format!("call_{idx}")
                } else {
                    id
                },
                kind: "function".into(),
                function: FunctionCall {
                    name,
                    arguments: if args.is_empty() { "{}".into() } else { args },
                },
            });
        }

        Ok(out)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}
