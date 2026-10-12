//! Minimal OpenAI-compatible chat client with SSE streaming.
//!
//! Only the surface soracode needs: streaming chat completions with tools.
//! Everything else (responses API, embeddings, vision) is intentionally absent
//! to keep the binary small.

mod client;
mod stream;
mod types;
mod wire;

pub use self::client::{Client, ClientConfig};
pub use self::stream::Completion;
pub use self::types::{Message, StreamEvent, ToolCall, ToolSpec};
