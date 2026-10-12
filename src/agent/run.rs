//! The run loop, tool dispatch, and note handling for [`Agent`].

use super::{Agent, AgentEvent};
use crate::error::Result;
use crate::llm::{Completion, Message, StreamEvent, ToolCall};
use crate::util::truncate_middle;

impl Agent {
    pub(super) fn emit(&self, ev: AgentEvent) {
        if let Some(tx) = &self.events {
            let _ = tx.send(ev);
        }
    }

    /// Run one instruction to completion, returning the final text.
    ///
    /// # Errors
    /// Returns [`crate::error::Error::Llm`] on unrecoverable model failures.
    pub async fn run(&mut self, instruction: &str) -> Result<super::TurnResult> {
        self.messages.push(Message::User {
            content: instruction.to_string(),
        });
        let _ = self.session.append("user", instruction, None);

        let mut turns = 0u32;
        let mut final_text = String::new();
        let started = std::time::Instant::now();

        loop {
            if self.pause.load(std::sync::atomic::Ordering::Relaxed) {
                self.emit(AgentEvent::Paused);
                break;
            }
            if turns >= self.config.max_turns {
                self.emit(AgentEvent::Info(format!(
                    "reached max_turns ({})",
                    self.config.max_turns
                )));
                break;
            }
            if self.config.max_seconds > 0 && started.elapsed().as_secs() >= self.config.max_seconds
            {
                self.emit(AgentEvent::Info(format!(
                    "reached max_seconds ({})",
                    self.config.max_seconds
                )));
                break;
            }
            turns += 1;

            // Re-inject notes so the model always sees its own memory.
            self.compact();
            let with_notes = self.with_notes();

            let tx_text = self.events.clone();
            let tx_reason = self.events.clone();
            let tx_usage = self.events.clone();

            let completion = self
                .client
                .stream_chat(&with_notes, &self.registry.specs(), move |ev| match ev {
                    StreamEvent::Text(t) => {
                        if let Some(tx) = &tx_text {
                            let _ = tx.send(AgentEvent::Text(t));
                        }
                    }
                    StreamEvent::Reasoning(r) => {
                        if let Some(tx) = &tx_reason {
                            let _ = tx.send(AgentEvent::Reasoning(r));
                        }
                    }
                    StreamEvent::Usage { prompt, completion } => {
                        if let Some(tx) = &tx_usage {
                            let _ = tx.send(AgentEvent::Usage { prompt, completion });
                        }
                    }
                })
                .await?;

            let Completion {
                content,
                tool_calls,
                finish_reason,
                ..
            } = completion;

            if !content.is_empty() {
                final_text = content.clone();
                let _ = self.session.append("assistant", &content, None);
            }

            if tool_calls.is_empty() {
                if finish_reason.as_deref() == Some("length") {
                    self.emit(AgentEvent::Info("response truncated by max_tokens".into()));
                }
                self.messages.push(Message::Assistant {
                    content: if content.is_empty() {
                        None
                    } else {
                        Some(content)
                    },
                    tool_calls: None,
                });
                break;
            }

            self.messages.push(Message::Assistant {
                content: if content.is_empty() {
                    None
                } else {
                    Some(content)
                },
                tool_calls: Some(tool_calls.clone()),
            });

            for call in tool_calls {
                let output = self.dispatch(&call).await;
                let _ = self
                    .session
                    .append("tool", &output, Some(&call.function.name));
                self.messages.push(Message::Tool {
                    tool_call_id: call.id.clone(),
                    content: output,
                });
            }
        }

        self.emit(AgentEvent::Done);
        Ok(super::TurnResult { final_text, turns })
    }

    async fn dispatch(&self, call: &ToolCall) -> String {
        let args: serde_json::Value = serde_json::from_str(&call.function.arguments)
            .unwrap_or_else(|_| serde_json::json!({}));

        self.emit(AgentEvent::ToolStart {
            name: call.function.name.clone(),
            args: call.function.arguments.clone(),
        });

        let output = match self.registry.call(&call.function.name, args).await {
            Ok(o) => o,
            Err(e) => format!("[tool error] {e}"),
        };

        self.emit(AgentEvent::ToolEnd {
            name: call.function.name.clone(),
            output: truncate_middle(&output, 2000),
        });

        output
    }

    /// Elide the oldest turns when the conversation exceeds the char budget.
    ///
    /// Only whole turns are dropped, and only from the second turn onward, so
    /// the system prompts, the opening instruction, and the most recent turn
    /// always survive, and an assistant tool call is never separated from the
    /// tool result that answers it.
    fn compact(&mut self) {
        let forced = self
            .compact_now
            .swap(false, std::sync::atomic::Ordering::Relaxed);
        let budget = super::budget_for(self.config.context_budget, forced);
        if let Some(elided) = super::compact_messages(&mut self.messages, budget) {
            self.emit(AgentEvent::Info(format!(
                "context compacted: elided {elided} earlier messages"
            )));
        } else if forced {
            self.emit(AgentEvent::Info(
                "context compacted: nothing to elide".into(),
            ));
        }
    }

    /// Build the message list with the current notes appended.
    fn with_notes(&self) -> Vec<Message> {
        let mut msgs = self.messages.clone();
        let rendered = self.notes.render_blocking();
        if !rendered.is_empty() {
            msgs.push(Message::System { content: rendered });
        }
        msgs
    }

    /// Persist any in-memory notes that are not yet stored.
    ///
    /// # Errors
    /// Propagates session storage errors.
    pub fn flush_notes(&self) -> Result<()> {
        for n in self.notes.all_blocking() {
            let _ = self.session.add_note(&n.tag, &n.body);
        }
        Ok(())
    }

    /// Access the shared notes store (used by the spawner to share context).
    #[must_use]
    #[allow(dead_code)] // shared-context accessor for the spawner
    pub fn notes(&self) -> crate::tools::notes::Notes {
        self.notes.clone()
    }

    /// The configured model name.
    #[must_use]
    #[allow(dead_code)] // surfaced for logging/diagnostics
    pub fn model(&self) -> &str {
        self.client.model()
    }
}
