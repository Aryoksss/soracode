//! `soracode` — a lightweight, fully operator-controlled security agent.
//!
//! The binary has two modes:
//!   * one-shot CLI: `soracode -i "instruction"`
//!   * interactive TUI: `soracode` (or `soracode -i` and keep typing)

mod agent;
mod cli;
mod commands;
mod config;
mod error;
mod llm;
mod models;
mod prompt;
mod runtime;
mod session;
mod tools;
mod tui;
mod util;

use std::sync::Arc;

use clap::Parser;

use crate::cli::Cli;
use crate::config::Config;
use crate::error::Result;
use crate::llm::Client;
use crate::session::Session;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("soracode: {e}");
        std::process::exit(1);
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
            eprintln!(
                "soracode: {} already exists, leaving it alone",
                path.display()
            );
        } else {
            std::fs::write(&path, models::starter_models_toml())?;
            eprintln!("soracode: wrote {}", path.display());
        }
        return Ok(());
    }

    if cli.models {
        commands::print_models(&cfg);
        return Ok(());
    }

    if cli.print_prompt {
        let inventory = tools::catalog::inventory().await;
        let active = cfg
            .resolve(cfg.requested(cli.model.as_deref()).as_deref())?
            .name;
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
            eprintln!(
                "soracode: cache refreshed at {}",
                tools::catalog::cache_path().display()
            );
        }
        return Ok(());
    }

    if cli.sessions {
        commands::print_sessions(&cfg.db_path)?;
        return Ok(());
    }

    let requested = cfg.requested(cli.model.as_deref());
    cfg.validate(requested.as_deref())?;

    if cli.rescan || tools::catalog::load_cache().is_none() {
        let inv = tools::catalog::rescan().await;
        if !std::io::IsTerminal::is_terminal(&std::io::stderr()) {
            eprintln!("soracode: {}", tools::catalog::summarize(&inv));
        }
    }

    let resolved = cfg.resolve(requested.as_deref())?;

    let instruction = commands::assemble_instruction(
        &cli.instruction,
        cli.instruction_file.as_deref(),
        cli.target.as_deref(),
    )?;

    let resume_id = commands::resolve_resume(&cfg.db_path, cli.resume.as_deref(), cli.cont)?;

    let session = Arc::new(Session::open(&cfg.db_path, resume_id.as_deref())?);
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
        runtime::run_tui(client, cfg, session)
    } else {
        if instruction.trim().is_empty() {
            return Err(error::Error::Config(
                "no instruction given (use -i, --instruction-file, or -t)".into(),
            ));
        }
        runtime::run_once(client, cfg, session, instruction).await
    }
}
