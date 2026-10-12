//! HTTP transport and the streaming chat call.

use std::collections::BTreeMap;
use std::time::Duration;

use futures_util::StreamExt;

use super::stream::{Assembler, Completion};
use super::types::{Message, StreamEvent, ToolSpec};
use super::wire;
use crate::error::{Error, Result};
use crate::models::Wire;

/// Everything needed to open a client connection.
#[derive(Clone)]
pub struct ClientConfig {
    /// Provider endpoint.
    pub base_url: String,
    /// Request/response dialect.
    pub wire: Wire,
    /// Provider credential.
    pub api_key: String,
    /// Remote model string.
    pub model: String,
    /// Token cap per turn.
    pub max_tokens: u32,
    /// Sampling temperature.
    pub temperature: f32,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
    /// Attempts per request, including the first.
    pub max_attempts: u32,
    /// Base delay in milliseconds for the exponential retry backoff.
    pub retry_backoff_ms: u64,
    /// Extra headers sent on every request.
    pub extra: BTreeMap<String, String>,
}

/// Streaming-capable client for one provider endpoint.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    wire: Wire,
    api_key: String,
    model: String,
    max_tokens: u32,
    temperature: f32,
    max_attempts: u32,
    retry_backoff_ms: u64,
    extra: BTreeMap<String, String>,
}

impl Client {
    /// Build a client from a connection description.
    ///
    /// # Errors
    /// Returns [`Error::Llm`] if the HTTP client cannot be constructed.
    pub fn new(cfg: ClientConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(cfg.timeout_secs.max(1)))
            .connect_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(8)
            .build()
            .map_err(|e| Error::Llm(format!("http client: {e}")))?;

        Ok(Self {
            http,
            base_url: cfg.base_url.trim_end_matches('/').to_string(),
            wire: cfg.wire,
            api_key: cfg.api_key,
            model: cfg.model,
            max_tokens: cfg.max_tokens,
            temperature: cfg.temperature,
            max_attempts: cfg.max_attempts.max(1),
            retry_backoff_ms: cfg.retry_backoff_ms,
            extra: cfg.extra,
        })
    }

    /// The model identifier in use.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// The request/response dialect in use.
    #[must_use]
    pub fn wire(&self) -> Wire {
        self.wire
    }

    /// Stream one assistant turn, invoking `on_event` for each incremental chunk.
    ///
    /// The request is retried on a transport failure or a retryable HTTP status
    /// (`408`, `409`, `425`, `429`, or any `5xx`) with an exponential backoff.
    /// A failure after the response body has started streaming is not retried,
    /// because earlier chunks have already been delivered to `on_event`.
    ///
    /// # Errors
    /// Returns [`Error::Llm`] once every attempt fails, or on a non-retryable
    /// status.
    pub async fn stream_chat<F>(
        &self,
        messages: &[Message],
        tools: &[ToolSpec],
        mut on_event: F,
    ) -> Result<Completion>
    where
        F: FnMut(StreamEvent) + Send,
    {
        let out = wire::build(
            self.wire,
            &self.model,
            messages,
            tools,
            self.max_tokens,
            self.temperature,
        );
        let url = format!("{}{}", self.base_url, out.path);

        let resp = self.send_with_retry(&url, &out.body).await?;

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

                for ev in wire::absorb(self.wire, &mut asm, data) {
                    on_event(ev);
                }
            }
        }

        Ok(asm.finish())
    }

    /// Send one request, retrying a transport failure or a retryable status.
    async fn send_with_retry(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::Response> {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let result = self.send_once(url, body).await;

            match result {
                Ok(resp) => return Ok(resp),
                Err(_) if attempt < self.max_attempts => {
                    let shift = attempt.min(8) - 1;
                    let delay = self.retry_backoff_ms * 2u64.pow(shift);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Send one request and classify the response as usable or retryable.
    async fn send_once(&self, url: &str, body: &serde_json::Value) -> Result<reqwest::Response> {
        let mut req = self
            .http
            .post(url)
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .json(body);
        for (name, value) in wire::auth_headers(self.wire, &self.api_key) {
            req = req.header(name, value);
        }
        for (k, v) in &self.extra {
            req = req.header(k.as_str(), v.as_str());
        }

        let resp = req.send().await.map_err(|e| {
            Error::Llm(if e.is_connect() || e.is_timeout() || e.is_request() {
                format!("transport: {e}")
            } else {
                format!("non-retryable transport: {e}")
            })
        })?;

        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }

        let code = status.as_u16();
        let text = resp.text().await.unwrap_or_default();
        if is_retryable_status(code) {
            Err(Error::Llm(format!(
                "retryable HTTP {status} from {url}: {}",
                truncate(&text, 800)
            )))
        } else {
            Err(Error::Llm(format!(
                "HTTP {status} from {url}: {}",
                truncate(&text, 800)
            )))
        }
    }
}

fn is_retryable_status(code: u16) -> bool {
    matches!(code, 408 | 409 | 425 | 429 | 500..=599)
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

#[cfg(test)]
mod tests;
