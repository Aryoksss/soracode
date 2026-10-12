//! Assembling streamed deltas into a complete turn.

use std::collections::BTreeMap;

use super::types::{DeltaToolCall, FunctionCall, StreamChunk, StreamEvent, ToolCall};

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
    /// Prompt tokens billed for the turn, as reported by the provider.
    pub prompt_tokens: u64,
    /// Completion tokens billed for the turn, as reported by the provider.
    pub completion_tokens: u64,
}

/// Accumulates streamed fragments into a [`Completion`].
///
/// Tool calls arrive split across frames and are keyed by index, so they are
/// buffered separately and only assembled once the stream ends. A frame may
/// also carry only part of a tool's name or arguments, which is why those are
/// appended rather than replaced.
#[derive(Default)]
pub(super) struct Assembler {
    out: Completion,
    partial: BTreeMap<usize, (String, String, String)>,
}

impl Assembler {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Fold one SSE `data:` payload into the accumulating completion.
    ///
    /// Returns the incremental events in the order they appeared in the frame,
    /// so the caller can stream them out without re-parsing.
    pub(super) fn push(&mut self, data: &str) -> Vec<StreamEvent> {
        let mut events = Vec::new();
        let Ok(parsed) = serde_json::from_str::<StreamChunk>(data) else {
            return events;
        };

        if let Some(u) = parsed.usage {
            self.out.prompt_tokens = u.prompt_tokens;
            self.out.completion_tokens = u.completion_tokens;
            events.push(StreamEvent::Usage {
                prompt: u.prompt_tokens,
                completion: u.completion_tokens,
            });
        }

        for choice in parsed.choices {
            if let Some(reason) = choice.finish_reason {
                self.out.finish_reason = Some(reason);
            }
            if let Some(r) = choice.delta.reasoning_content {
                if !r.is_empty() {
                    self.out.reasoning.push_str(&r);
                    events.push(StreamEvent::Reasoning(r));
                }
            }
            if let Some(c) = choice.delta.content {
                if !c.is_empty() {
                    self.out.content.push_str(&c);
                    events.push(StreamEvent::Text(c));
                }
            }
            if let Some(calls) = choice.delta.tool_calls {
                for c in calls {
                    self.absorb(c);
                }
            }
        }

        events
    }

    fn absorb(&mut self, c: DeltaToolCall) {
        let slot = self.partial.entry(c.index).or_default();
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

    /// Record token usage reported so far, leaving unset counts unchanged.
    pub(super) fn set_usage(&mut self, prompt: Option<u64>, completion: Option<u64>) {
        if let Some(p) = prompt {
            self.out.prompt_tokens = p;
        }
        if let Some(c) = completion {
            self.out.completion_tokens = c;
        }
    }

    /// Number of tool calls opened so far; used to assign ids on wires that
    /// carry none.
    pub(super) fn tool_count(&self) -> usize {
        self.partial.len()
    }

    /// Open a tool call at `index`, optionally with its id and name.
    pub(super) fn start_tool(&mut self, index: usize, id: Option<&str>, name: Option<&str>) {
        let slot = self.partial.entry(index).or_default();
        if let Some(id) = id.filter(|s| !s.is_empty()) {
            slot.0 = id.to_string();
        }
        if let Some(name) = name {
            slot.1.push_str(name);
        }
    }

    /// Append to a tool call's argument JSON.
    pub(super) fn push_tool_args(&mut self, index: usize, args: &str) {
        self.partial.entry(index).or_default().2.push_str(args);
    }

    /// Append visible assistant text and emit it.
    pub(super) fn push_text(&mut self, text: &str) -> Vec<StreamEvent> {
        if text.is_empty() {
            return Vec::new();
        }
        self.out.content.push_str(text);
        vec![StreamEvent::Text(text.to_string())]
    }

    /// Append reasoning text and emit it.
    pub(super) fn push_reasoning(&mut self, text: &str) -> Vec<StreamEvent> {
        if text.is_empty() {
            return Vec::new();
        }
        self.out.reasoning.push_str(text);
        vec![StreamEvent::Reasoning(text.to_string())]
    }

    /// Finish the turn, materialising accumulated tool calls.
    pub(super) fn finish(mut self) -> Completion {
        for (idx, (id, name, args)) in std::mem::take(&mut self.partial) {
            if name.is_empty() {
                continue;
            }
            self.out.tool_calls.push(ToolCall {
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
        self.out
    }
}
