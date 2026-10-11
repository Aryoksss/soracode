//! `http` — direct HTTP requests without a browser.

use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};

use super::{arg_str, arg_str_opt, arg_u64_opt, Tool, ToolContext};
use crate::error::{Error, Result};
use crate::util::truncate_middle;

/// Performs an HTTP request and returns status, headers and body.
pub struct HttpTool {
    ctx: ToolContext,
}

impl HttpTool {
    /// Build the tool for the given context.
    #[must_use]
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for HttpTool {
    fn name(&self) -> &'static str {
        "http"
    }

    fn description(&self) -> &'static str {
        "Send an HTTP request and return the status line, response headers and body. \
         TLS verification is disabled by default so self-signed and intercepted targets work. \
         Redirects are not followed unless `follow_redirects` is true."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "Target URL." },
                "method": { "type": "string", "description": "HTTP method. Defaults to GET." },
                "headers": {
                    "type": "object",
                    "description": "Request headers as a JSON object.",
                    "additionalProperties": { "type": "string" }
                },
                "body": { "type": "string", "description": "Raw request body." },
                "json": { "type": "object", "description": "Body to send as JSON (sets content-type)." },
                "follow_redirects": { "type": "boolean", "description": "Follow redirects. Defaults to false." },
                "timeout_secs": { "type": "integer", "description": "Request timeout. Defaults to 60." }
            },
            "required": ["url"]
        })
    }

    async fn run(&self, args: Value) -> Result<String> {
        let url = arg_str(&args, "url")?;
        let method = arg_str_opt(&args, "method").unwrap_or_else(|| "GET".into());
        let follow = args
            .get("follow_redirects")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let timeout = arg_u64_opt(&args, "timeout_secs").unwrap_or(60);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout.max(1)))
            .danger_accept_invalid_certs(true)
            .redirect(if follow {
                reqwest::redirect::Policy::limited(10)
            } else {
                reqwest::redirect::Policy::none()
            })
            .build()
            .map_err(|e| Error::Tool(format!("http client: {e}")))?;

        let m = reqwest::Method::from_bytes(method.to_uppercase().as_bytes())
            .map_err(|_| Error::Tool(format!("invalid method: {method}")))?;

        let mut req = client.request(m, &url);

        if let Some(h) = args.get("headers").and_then(Value::as_object) {
            for (k, v) in h {
                if let Some(v) = v.as_str() {
                    req = req.header(k.as_str(), v);
                }
            }
        }

        if let Some(j) = args.get("json") {
            req = req.json(j);
        } else if let Some(b) = arg_str_opt(&args, "body") {
            req = req.body(b);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| Error::Tool(format!("request failed: {e}")))?;

        let status = resp.status();
        let final_url = resp.url().to_string();
        let headers: Vec<(String, String)> = resp
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    v.to_str().unwrap_or("<binary>").to_string(),
                )
            })
            .collect();

        let body = resp.text().await.unwrap_or_default();
        let limit = self.ctx.config.tool_output_limit;

        let mut out = format!("{} {} -> {status}\n", method.to_uppercase(), url);
        if final_url != url {
            out.push_str(&format!("final url: {final_url}\n"));
        }
        for (k, v) in headers {
            out.push_str(&format!("{k}: {v}\n"));
        }
        out.push('\n');
        out.push_str(&truncate_middle(&body, limit));
        Ok(out)
    }
}
