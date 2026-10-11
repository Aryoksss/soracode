//! The agent loop: conversation state, tool dispatch, context compaction.

use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;

use crate::config::AgentConfig;
use crate::error::{Error, Result};
use crate::llm::{Client, Completion, Message, StreamEvent, ToolCall};
use crate::prompt;
use crate::session::Session;
use crate::tools::subagent::Spawner;
use crate::tools::{Registry, ToolContext};
use crate::util::truncate_middle;

/// Events emitted by an agent for the UI to render.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Assistant visible text chunk.
    Text(String),
    /// Assistant reasoning chunk.
    Reasoning(String),
    /// A tool is about to run.
    ToolStart {
        /// Tool name.
        name: String,
        /// Raw JSON arguments.
        args: String,
    },
    /// A tool finished.
    ToolEnd {
        /// Tool name.
        name: String,
        /// Truncated output for display.
        output: String,
    },
    /// A subagent started.
    SubagentStart {
        /// Label.
        label: String,
    },
    /// Informational message.
    Info(String),
    /// A fatal or recoverable error.
    Error(String),
    /// The turn finished.
    Done,
}

/// Result of a completed top-level instruction.
#[derive(Debug, Clone)]
pub struct TurnResult {
    /// Final assistant text.
    pub final_text: String,
    /// Number of model round-trips used.
    pub turns: u32,
}

/// Configuration for constructing an agent.
pub struct AgentSpec {
    /// The client to talk to the model.
    pub client: Client,
    /// Effective agent configuration.
    pub config: Arc<AgentConfig>,
    /// Whether this agent is a subagent.
    pub is_subagent: bool,
    /// Nesting depth (0 = root).
    pub depth: u32,
    /// Session store.
    pub session: Arc<Session>,
    /// Where UI events are sent.
    pub events: Option<UnboundedSender<AgentEvent>>,
    /// Bridge used by the `subagent` tool.
    pub spawner: Option<Arc<dyn Spawner>>,
}

/// A running agent with a conversation history.
pub struct Agent {
    client: Client,
    config: Arc<AgentConfig>,
    registry: Registry,
    notes: crate::tools::notes::Notes,
    messages: Vec<Message>,
    session: Arc<Session>,
    events: Option<UnboundedSender<AgentEvent>>,
}

impl Agent {
    fn emit(&self, ev: AgentEvent) {
        if let Some(tx) = &self.events {
            let _ = tx.send(ev);
        }
    }

    /// Build an agent from a spec.
    ///
    /// # Errors
    /// Returns an error if the tool registry cannot be built.
    pub fn new(spec: AgentSpec) -> Result<Self> {
        let notes = crate::tools::notes::Notes::default();

        // Preload persisted notes so resumed sessions keep their memory.
        if let Ok(saved) = spec.session.notes() {
            for (tag, body) in saved {
                notes.add_blocking(&tag, &body);
            }
        }

        let ctx = ToolContext {
            config: spec.config.clone(),
            workspace: spec.config.workspace.clone(),
            depth: spec.depth,
            notes: notes.clone(),
            spawner: spec.spawner,
        };

        let registry = Registry::standard(&ctx);

        let mut messages = vec![Message::System {
            content: prompt::for_role(spec.is_subagent).to_string(),
        }];

        // Root agents get an environment block.
        if !spec.is_subagent {
            let env = prompt::environment_block(
                &spec.config.workspace,
                spec.client.model(),
                spec.session.id(),
                0,
            );
            messages.push(Message::System { content: env });
        }

        Ok(Self {
            client: spec.client,
            config: spec.config,
            registry,
            notes,
            messages,
            session: spec.session,
            events: spec.events,
        })
    }

    /// Run one instruction to completion, returning the final text.
    ///
    /// # Errors
    /// Returns [`Error::Llm`] on unrecoverable model failures.
    pub async fn run(&mut self, instruction: &str) -> Result<TurnResult> {
        self.messages.push(Message::User {
            content: instruction.to_string(),
        });
        let _ = self.session.append("user", instruction, None);

        let mut turns = 0u32;
        let mut final_text = String::new();

        loop {
            if turns >= self.config.max_turns {
                self.emit(AgentEvent::Info(format!(
                    "reached max_turns ({})",
                    self.config.max_turns
                )));
                break;
            }
            turns += 1;

            // Re-inject notes so the model always sees its own memory.
            let with_notes = self.with_notes();

            let tx_text = self.events.clone();
            let tx_reason = self.events.clone();

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
                    self.emit(AgentEvent::Info(
                        "response truncated by max_tokens".into(),
                    ));
                }
                self.messages.push(Message::Assistant {
                    content: if content.is_empty() { None } else { Some(content) },
                    tool_calls: None,
                });
                break;
            }

            self.messages.push(Message::Assistant {
                content: if content.is_empty() { None } else { Some(content) },
                tool_calls: Some(tool_calls.clone()),
            });

            for call in tool_calls {
                let output = self.dispatch(&call).await;
                let _ = self.session.append("tool", &output, Some(&call.function.name));
                self.messages.push(Message::Tool {
                    tool_call_id: call.id.clone(),
                    content: output,
                });
            }
        }

        self.emit(AgentEvent::Done);
        Ok(TurnResult { final_text, turns })
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
    pub fn flush_notes(&self) -> Result<()> {
        for n in self.notes.all_blocking() {
            let _ = self.session.add_note(&n.tag, &n.body);
        }
        Ok(())
    }

    /// Access the shared notes store (used by the spawner to share context).
    #[must_use]
    pub fn notes(&self) -> crate::tools::notes::Notes {
        self.notes.clone()
    }

    /// The configured model name.
    #[must_use]
    pub fn model(&self) -> &str {
        self.client.model()
    }
}

/// Adapter that lets the `subagent` tool start nested agents.
pub struct SubagentSpawner {
    client: Client,
    config: Arc<AgentConfig>,
    session: Arc<Session>,
    events: Option<UnboundedSender<AgentEvent>>,
    depth: u32,
}

impl SubagentSpawner {
    /// Build a spawner for a given depth.
    #[must_use]
    pub fn new(
        client: Client,
        config: Arc<AgentConfig>,
        session: Arc<Session>,
        events: Option<UnboundedSender<AgentEvent>>,
        depth: u32,
    ) -> Self {
        Self {
            client,
            config,
            session,
            events,
            depth,
        }
    }
}

#[async_trait::async_trait]
impl Spawner for SubagentSpawner {
    async fn spawn(&self, instruction: &str, label: Option<String>) -> Result<String> {
        if self.depth >= self.config.subagent_max_depth {
            return Err(Error::Tool("subagent depth limit reached".into()));
        }

        let label = label.unwrap_or_else(crate::util::short_id);
        if let Some(tx) = &self.events {
            let _ = tx.send(AgentEvent::SubagentStart {
                label: label.clone(),
            });
        }

        let nested_spawner = Arc::new(Self::new(
            self.client.clone(),
            self.config.clone(),
            self.session.clone(),
            self.events.clone(),
            self.depth + 1,
        ));

        let spec = AgentSpec {
            client: self.client.clone(),
            config: self.config.clone(),
            is_subagent: true,
            depth: self.depth + 1,
            session: self.session.clone(),
            events: self.events.clone(),
            spawner: Some(nested_spawner),
        };

        let mut agent = Agent::new(spec)?;
        let result = agent.run(instruction).await?;
        agent.flush_notes()?;
        Ok(result.final_text)
    }
}
