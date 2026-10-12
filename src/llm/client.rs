//! HTTP transport and the streaming chat call.

use std::collections::BTreeMap;
use std::time::Duration;

use futures_util::StreamExt;

use super::stream::{Assembler, Completion};
use super::types::{ChatRequest, Message, StreamEvent, ToolSpec};
use crate::error::{Error, Result};

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

        let mut asm = Assembler::new();
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

                for ev in asm.push(data) {
                    on_event(ev);
                }
            }
        }

        Ok(asm.finish())
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
