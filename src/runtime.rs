//! Agent construction and the two execution modes (one-shot and TUI).

use std::sync::{Arc, Mutex};

use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use crate::agent::{Agent, AgentEvent, AgentSpec, SubagentSpawner};
use crate::commands;
use crate::config::Config;
use crate::error::Result;
use crate::llm::Client;
use crate::session::Session;
use crate::tui;

/// Build a fully wired agent for one instruction.
pub(crate) fn build_agent(
    client: Client,
    cfg: &Arc<Config>,
    session: Arc<Session>,
    events: Option<UnboundedSender<AgentEvent>>,
    depth: u32,
    is_subagent: bool,
) -> Result<Agent> {
    let agent_cfg = Arc::new(cfg.agent().clone());
    let spawner = Arc::new(SubagentSpawner::new(
        client.clone(),
        agent_cfg.clone(),
        session.clone(),
        events.clone(),
        depth,
    ));

    Agent::new(AgentSpec {
        client,
        config: agent_cfg,
        is_subagent,
        depth,
        role: None,
        session,
        events,
        spawner: Some(spawner),
    })
}

/// Run one instruction and print its events to stdout.
pub(crate) async fn run_once(
    client: Client,
    cfg: Arc<Config>,
    session: Arc<Session>,
    instruction: String,
) -> Result<()> {
    let (tx, mut rx) = unbounded_channel();

    // Print events as they arrive.
    let printer = tokio::spawn(async move {
        while let Some(ev) = rx.recv().await {
            commands::print_event(ev);
        }
    });

    let mut agent = build_agent(client, &cfg, session.clone(), Some(tx), 0, false)?;
    agent.run(&instruction).await?;
    agent.flush_notes()?;

    drop(agent);
    let _ = printer.await;
    println!();
    Ok(())
}

/// Run the interactive TUI until the operator quits.
pub(crate) fn run_tui(client: Client, cfg: Arc<Config>, session: Arc<Session>) -> Result<()> {
    let client = Arc::new(Mutex::new(client));
    let shared = tui::shared((*session).clone());
    let mut app = tui::App::new(cfg.clone(), client.clone(), shared.clone());

    let cb_cfg = cfg.clone();
    let cb_shared = shared.clone();
    let cb_client = client.clone();
    let cancel = app.cancel_notify();

    app.run(move |instruction, tx| {
        let cfg = cb_cfg.clone();
        let session = cb_shared.lock().expect("session mutex").clone();
        let client = cb_client.lock().expect("client mutex").clone();
        let tx2 = tx.clone();
        let cancel = cancel.clone();

        tokio::spawn(async move {
            match build_agent(client, &cfg, session.clone(), Some(tx.clone()), 0, false) {
                Ok(mut agent) => {
                    {
                        let turn = agent.run(&instruction);
                        tokio::pin!(turn);
                        tokio::select! {
                            r = &mut turn => {
                                if let Err(e) = r {
                                    let _ = tx2.send(AgentEvent::Error(e.to_string()));
                                }
                            }
                            _ = cancel.notified() => {
                                let _ = tx2.send(AgentEvent::Cancelled);
                            }
                        }
                    }
                    let _ = agent.flush_notes();
                }
                Err(e) => {
                    let _ = tx2.send(AgentEvent::Error(e.to_string()));
                }
            }
        });
    })?;

    let id = shared.lock().expect("session mutex").id().to_string();
    eprintln!("soracode: session {id} — resume with: soracode --resume {id}");

    Ok(())
}
