//! Adapter letting the `subagent` tool start nested agents.

use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;

use super::{Agent, AgentEvent, AgentSpec};
use crate::config::AgentConfig;
use crate::error::{Error, Result};
use crate::llm::Client;
use crate::session::Session;
use crate::tools::subagent::Spawner;

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
    async fn spawn(
        &self,
        instruction: &str,
        role: Option<String>,
        label: Option<String>,
    ) -> Result<String> {
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
            role,
            session: self.session.clone(),
            events: self.events.clone(),
            spawner: Some(nested_spawner),
            pause: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        let mut agent = Agent::new(spec)?;
        let result = agent.run(instruction).await?;
        agent.flush_notes()?;
        Ok(result.final_text)
    }
}
