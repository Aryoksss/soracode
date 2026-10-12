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

/// Operator-controlled knobs that can change between turns.
#[derive(Clone)]
pub(crate) struct Runtime {
    /// Shared pause flag the loop reads before each tool call.
    pub pause: Arc<std::sync::atomic::AtomicBool>,
    /// Autonomy mode.
    pub mode: crate::config::Mode,
}

/// Build a fully wired agent for one instruction.
pub(crate) fn build_agent(
    client: Client,
    cfg: &Arc<Config>,
    session: Arc<Session>,
    events: Option<UnboundedSender<AgentEvent>>,
    depth: u32,
    is_subagent: bool,
    rt: Runtime,
) -> Result<Agent> {
    let mut agent_cfg = cfg.agent().clone();
    agent_cfg.mode = rt.mode;
    let agent_cfg = Arc::new(agent_cfg);
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
        pause: rt.pause,
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

    let mut agent = build_agent(
        client,
        &cfg,
        session.clone(),
        Some(tx),
        0,
        false,
        Runtime {
            pause: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            mode: cfg.agent().mode,
        },
    )?;
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
    let pause = app.pause_flag();
    let mode = app.mode();

    app.run(move |instruction, tx| {
        let cfg = cb_cfg.clone();
        let session = cb_shared.lock().expect("session mutex").clone();
        let client = cb_client.lock().expect("client mutex").clone();
        let tx2 = tx.clone();
        let cancel = cancel.clone();
        let pause = pause.clone();

        tokio::spawn(async move {
            let rt = Runtime {
                pause: pause.clone(),
                mode,
            };
            match build_agent(
                client,
                &cfg,
                session.clone(),
                Some(tx.clone()),
                0,
                false,
                rt,
            ) {
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
