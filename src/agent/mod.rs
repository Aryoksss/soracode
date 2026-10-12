//! The agent loop: conversation state, tool dispatch, context compaction.

mod replay;
mod run;
mod spawner;
mod types;

use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;

use crate::config::AgentConfig;
use crate::error::Result;
use crate::llm::{Client, Message};
use crate::prompt;
use crate::session::Session;
use crate::tools::{Registry, ToolContext};

pub use self::replay::replay_history;
pub use self::spawner::SubagentSpawner;
pub use self::types::{AgentEvent, AgentSpec, TurnResult};

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

        let session = spec.session;
        let is_subagent = spec.is_subagent;
        let role = spec.role;

        let ctx = ToolContext {
            config: spec.config.clone(),
            workspace: spec.config.workspace.clone(),
            depth: spec.depth,
            notes: notes.clone(),
            engagement: session.engagement()?,
            spawner: spec.spawner,
        };

        let registry = Registry::standard(&ctx);

        let mut messages = vec![Message::System {
            content: prompt::for_role(is_subagent).to_string(),
        }];

        if let Some(preamble) = role.as_deref().and_then(prompt::specialist_preamble) {
            messages.push(Message::System {
                content: preamble.to_string(),
            });
        }

        // Root agents get an environment block covering the host's real tool set.
        if !is_subagent {
            let inventory = crate::tools::catalog::load_cache();
            let mut env = prompt::environment_block_with_tools(
                &spec.config.workspace,
                spec.client.model(),
                session.id(),
                0,
                inventory.as_ref(),
            );
            env.push_str(&crate::skills::catalogue(&spec.config.workspace));
            messages.push(Message::System { content: env });

            replay_history(&session, &mut messages);
        }

        Ok(Self {
            client: spec.client,
            config: spec.config,
            registry,
            notes,
            messages,
            session,
            events: spec.events,
        })
    }
}

#[cfg(test)]
mod tests;
