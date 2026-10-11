//! `soracode` — a lightweight, fully operator-controlled security agent.
//!
//! The binary has two modes:
//!   * one-shot CLI: `soracode -i "instruction"`
//!   * interactive TUI: `soracode` (or `soracode -i` and keep typing)

mod agent;
mod config;
mod error;
mod llm;
mod models;
mod prompt;
mod session;
mod tools;
mod tui;
mod util;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use crate::agent::{Agent, AgentEvent, AgentSpec, SubagentSpawner};
use crate::config::Config;
use crate::error::Result;
use crate::llm::Client;
use crate::session::Session;

/// A lightweight, fully operator-controlled security agent.
#[derive(Parser, Debug)]
#[command(name = "soracode", version, about, long_about = None)]
struct Cli {
    /// Instruction to execute. May be given multiple times.
    #[arg(short = 'i', long = "instruction")]
    instruction: Vec<String>,

    /// Read the instruction from a file.
    #[arg(long = "instruction-file")]
    instruction_file: Option<PathBuf>,

    /// Target shorthand: appended to the instruction as "Target: <value>".
    #[arg(short = 't', long = "target")]
    target: Option<String>,

    /// Model override. Either a name from the models file or a raw model string.
    #[arg(short = 'm', long = "model")]
    model: Option<String>,

    /// API base URL override.
    #[arg(long = "base-url")]
    base_url: Option<String>,

    /// API key override.
    #[arg(long = "api-key")]
    api_key: Option<String>,

    /// Workspace directory tools operate in.
    #[arg(short = 'w', long = "workspace")]
    workspace: Option<PathBuf>,

    /// Config file to load on top of the defaults.
    #[arg(short = 'c', long = "config")]
    config: Option<PathBuf>,

    /// Resume an existing session by id.
    #[arg(long = "resume")]
    resume: Option<String>,

    /// List recent sessions and exit.
    #[arg(long = "sessions")]
    sessions: bool,

    /// List the tools detected on this host and exit.
    #[arg(long = "tools")]
    tools: bool,

    /// With --tools or --print-prompt, re-walk PATH instead of using the cache.
    #[arg(long = "rescan")]
    rescan: bool,

    /// List configured models and providers, then exit.
    #[arg(long = "models")]
    models: bool,

    /// Write a starter models file to the user config directory.
    #[arg(long = "init-models")]
    init_models: bool,

    /// Do not start the TUI; print events to stdout and exit.
    #[arg(short = 'n', long = "non-interactive")]
    non_interactive: bool,

    /// Print the resolved system prompt and exit (debugging aid).
    #[arg(long = "print-prompt")]
    print_prompt: bool,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("soracode: {e}");
        std::process::exit(1);
    }
}

/// Show what models are configured and which one is active.
fn print_models(cfg: &Config) {
    let active = cfg.selection(None);

    if cfg.models.is_empty() {
        println!("No named models configured.\n");
        println!("  active: {} (from [llm], no models file)", cfg.llm.model);
        println!("  base:   {}", cfg.llm.base_url);
        println!("\nCreate a starter file with:  soracode --init-models");
        println!("It is read from: {}", models::models_path().display());
        return;
    }

    println!("Providers:");
    for (name, p) in &cfg.providers {
        let key = if p.api_key.is_empty() { "no key" } else { "key set" };
        println!("  {name:<16} {:<44} ({key})", p.base_url);
    }

    println!("\nModels:");
    for (name, m) in &cfg.models {
        let mark = match &active {
            crate::config::ModelSelection::Named(n) if n == name => "*",
            _ => " ",
        };
        let mut notes = Vec::new();
        if let Some(v) = m.max_tokens {
            notes.push(format!("max_tokens={v}"));
        }
        if let Some(v) = m.temperature {
            notes.push(format!("temp={v}"));
        }
        if let Some(v) = m.timeout_secs {
            notes.push(format!("timeout={v}s"));
        }
        let suffix = if notes.is_empty() {
            String::new()
        } else {
            format!("  [{}]", notes.join(", "))
        };
        println!(" {mark} {name:<16} {:<28} via {}{suffix}", m.model, m.provider);
    }

    let default = cfg.default_model.as_deref().unwrap_or("<none>");
    println!("\n  default_model = {default}");

    match &active {
        crate::config::ModelSelection::Named(_) => {
            println!("  ( * marks the model a plain run would use )");
        }
        crate::config::ModelSelection::Default => {
            println!(
                "  warning: default_model `{default}` is not defined; \
                 a plain run falls back to llm.model = {}",
                cfg.llm.model
            );
        }
        crate::config::ModelSelection::Literal(_) | crate::config::ModelSelection::Unknown(_) => {}
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    let mut cfg = Config::load_with_models(cli.config.as_deref())?;
    cfg.apply_overrides(
        cli.base_url.clone(),
        cli.api_key.clone(),
        cli.model.clone(),
        cli.workspace.clone(),
    );

    if cli.init_models {
        let path = models::models_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        if path.exists() {
            eprintln!("soracode: {} already exists, leaving it alone", path.display());
        } else {
            std::fs::write(&path, models::starter_models_toml())?;
            eprintln!("soracode: wrote {}", path.display());
        }
        return Ok(());
    }

    if cli.models {
        print_models(&cfg);
        return Ok(());
    }

    if cli.print_prompt {
        let inventory = tools::catalog::inventory().await;
        let active = cfg
            .resolve(cli.model.as_deref())
            .map_or_else(|_| cfg.llm.model.clone(), |r| r.name);
        let env = prompt::environment_block_with_tools(
            &cfg.agent().workspace,
            &active,
            "preview",
            0,
            Some(&inventory),
        );
        println!("{}", prompt::for_role(false));
        println!("{env}");
        return Ok(());
    }

    if cli.tools {
        let inv = if cli.rescan {
            tools::catalog::rescan().await
        } else {
            tools::catalog::inventory().await
        };
        println!("{}", inv.render());
        if cli.rescan {
            eprintln!("soracode: cache refreshed at {}", tools::catalog::cache_path().display());
        }
        return Ok(());
    }

    if cli.sessions {
        let rows = Session::list(&cfg.db_path, 20)?;
        if rows.is_empty() {
            println!("no sessions yet");
        }
        for (id, ts, info) in rows {
            let when = chrono::DateTime::from_timestamp(ts, 0)
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default();
            println!("{id}  {when}  {info}");
        }
        return Ok(());
    }

    cfg.validate(cli.model.as_deref())?;

    if cli.rescan || tools::catalog::load_cache().is_none() {
        let inv = tools::catalog::rescan().await;
        if !std::io::IsTerminal::is_terminal(&std::io::stderr()) {
            eprintln!("soracode: {}", tools::catalog::summarize(&inv));
        }
    }

    let resolved = cfg.resolve(cli.model.as_deref())?;

    // Assemble the instruction.
    let mut instruction = cli.instruction.join("\n\n");
    if let Some(path) = &cli.instruction_file {
        let text = std::fs::read_to_string(path)?;
        if !instruction.is_empty() {
            instruction.push_str("\n\n");
        }
        instruction.push_str(&text);
    }
    if let Some(target) = &cli.target {
        if !instruction.is_empty() {
            instruction.push('\n');
        }
        instruction.push_str(&format!("Target: {target}"));
    }

    let session = Arc::new(Session::open(&cfg.db_path, cli.resume.as_deref())?);
    let cfg = Arc::new(cfg);

    let client = Client::new(
        &resolved.base_url,
        &resolved.api_key,
        &resolved.model,
        resolved.max_tokens,
        resolved.temperature,
        resolved.timeout_secs,
        resolved.headers.clone(),
    )?;

    eprintln!(
        "soracode: session {} | model {} via {} | workspace {}",
        session.id(),
        resolved.name,
        resolved.provider,
        cfg.agent().workspace.display()
    );

    let interactive = !cli.non_interactive && instruction.is_empty();

    if interactive {
        run_tui(client, cfg, session)
    } else {
        if instruction.trim().is_empty() {
            return Err(error::Error::Config(
                "no instruction given (use -i, --instruction-file, or -t)".into(),
            ));
        }
        run_once(client, cfg, session, instruction).await
    }
}

/// Build a fully wired agent for one instruction.
fn build_agent(
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
        session,
        events,
        spawner: Some(spawner),
    })
}

async fn run_once(
    client: Client,
    cfg: Arc<Config>,
    session: Arc<Session>,
    instruction: String,
) -> Result<()> {
    let (tx, mut rx) = unbounded_channel();

    // Print events as they arrive.
    let printer = tokio::spawn(async move {
        while let Some(ev) = rx.recv().await {
            match ev {
                AgentEvent::Text(t) => {
                    print!("{t}");
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                }
                AgentEvent::Reasoning(_) => {}
                AgentEvent::ToolStart { name, args } => {
                    let short = util::truncate_end(&args, 160);
                    println!("\n\x1b[33m▶ {name} {short}\x1b[0m");
                }
                AgentEvent::ToolEnd { name, output } => {
                    let first = output.lines().next().unwrap_or("");
                    let extra = output.lines().count().saturating_sub(1);
                    if extra > 0 {
                        println!("\x1b[33m◀ {name}: {first}  (+{extra} more lines)\x1b[0m");
                    } else {
                        println!("\x1b[33m◀ {name}: {first}\x1b[0m");
                    }
                }
                AgentEvent::SubagentStart { label } => {
                    println!("\x1b[36m⇢ subagent `{label}`\x1b[0m");
                }
                AgentEvent::Info(m) => println!("\x1b[90m{m}\x1b[0m"),
                AgentEvent::Error(m) => eprintln!("\x1b[31m{m}\x1b[0m"),
                AgentEvent::Done => {}
            }
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

fn run_tui(client: Client, cfg: Arc<Config>, session: Arc<Session>) -> Result<()> {
    let mut app = tui::App::new();

    let cb_cfg = cfg.clone();
    let cb_session = session.clone();
    let cb_client = client;

    app.run(move |instruction, tx| {
        let cfg = cb_cfg.clone();
        let session = cb_session.clone();
        let client = cb_client.clone();
        let tx2 = tx.clone();

        tokio::spawn(async move {
            match build_agent(client, &cfg, session.clone(), Some(tx.clone()), 0, false) {
                Ok(mut agent) => {
                    if let Err(e) = agent.run(&instruction).await {
                        let _ = tx2.send(AgentEvent::Error(e.to_string()));
                    }
                    let _ = agent.flush_notes();
                }
                Err(e) => {
                    let _ = tx2.send(AgentEvent::Error(e.to_string()));
                }
            }
        });
    })?;

    Ok(())
}
