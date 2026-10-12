//! `note` / `recall` — persistent scratch memory for an agent.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{json, Value};

use super::{arg_str, arg_str_opt, Tool, ToolContext};
use crate::error::Result;

/// In-memory note store shared by an agent and browsable across turns.
#[derive(Clone, Default)]
pub struct Notes {
    inner: Arc<Mutex<Vec<Note>>>,
}

/// A single saved observation.
#[derive(Debug, Clone)]
pub struct Note {
    /// Tag / title for the note.
    pub tag: String,
    /// Free-form body.
    pub body: String,
}

impl Notes {
    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Note>> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Save a note (synchronous).
    pub fn add_blocking(&self, tag: &str, body: &str) {
        self.lock().push(Note {
            tag: tag.to_string(),
            body: body.to_string(),
        });
    }

    #[must_use]
    pub fn all_blocking(&self) -> Vec<Note> {
        self.lock().clone()
    }

    #[must_use]
    pub fn render_blocking(&self) -> String {
        let notes = self.all_blocking();
        if notes.is_empty() {
            return String::new();
        }
        let mut s = String::from("\n## Notes so far\n\n");
        for n in notes {
            s.push_str(&format!("- **{}**: {}\n", n.tag, n.body.replace('\n', " ")));
        }
        s
    }

    pub async fn add(&self, tag: &str, body: &str) {
        self.add_blocking(tag, body);
    }

    pub async fn all(&self) -> Vec<Note> {
        self.all_blocking()
    }

    pub async fn search(&self, needle: &str) -> Vec<Note> {
        let needle = needle.to_lowercase();
        self.lock()
            .iter()
            .filter(|n| {
                n.tag.to_lowercase().contains(&needle) || n.body.to_lowercase().contains(&needle)
            })
            .cloned()
            .collect()
    }

    pub async fn len(&self) -> usize {
        self.lock().len()
    }
}

/// Records a note.
pub struct NoteTool {
    ctx: ToolContext,
}

impl NoteTool {
    /// Build the tool for the given context.
    #[must_use]
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for NoteTool {
    fn name(&self) -> &'static str {
        "note"
    }

    fn description(&self) -> &'static str {
        "Save a short note (finding, credential, endpoint, todo) that persists for the rest of \
         the session and is re-injected into your context. Use it to remember discoveries \
         without keeping them in your reasoning."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "tag": { "type": "string", "description": "Short label, e.g. 'open-port' or 'cred'." },
                "body": { "type": "string", "description": "What to remember." }
            },
            "required": ["tag", "body"]
        })
    }

    async fn run(&self, args: Value) -> Result<String> {
        let tag = arg_str(&args, "tag")?;
        let body = arg_str(&args, "body")?;
        self.ctx.notes.add(&tag, &body).await;
        Ok(format!(
            "noted [{tag}] ({} total)",
            self.ctx.notes.len().await
        ))
    }
}

/// Retrieves notes by substring.
pub struct RecallTool {
    ctx: ToolContext,
}

impl RecallTool {
    /// Build the tool for the given context.
    #[must_use]
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for RecallTool {
    fn name(&self) -> &'static str {
        "recall"
    }

    fn description(&self) -> &'static str {
        "Search saved notes. Omit `query` to list all of them."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Substring to match. Omit to list everything." }
            }
        })
    }

    async fn run(&self, args: Value) -> Result<String> {
        let notes = match arg_str_opt(&args, "query") {
            Some(q) if !q.trim().is_empty() => self.ctx.notes.search(&q).await,
            _ => self.ctx.notes.all().await,
        };
        if notes.is_empty() {
            return Ok("(no matching notes)".into());
        }
        let mut s = String::new();
        for n in notes {
            s.push_str(&format!("[{}] {}\n", n.tag, n.body));
        }
        Ok(s)
    }
}
