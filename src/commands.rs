//! Read-only informational commands that print and exit.

use crate::agent::AgentEvent;
use crate::config::{Config, ModelSelection};
use crate::models;
use crate::util;

/// Show what models are configured and which one is active.
pub fn print_models(cfg: &Config) {
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
        let key = if p.api_key.is_empty() {
            "no key"
        } else {
            "key set"
        };
        println!("  {name:<16} {:<44} ({key})", p.base_url);
    }

    println!("\nModels:");
    for (name, m) in &cfg.models {
        let mark = match &active {
            ModelSelection::Named(n) if n == name => "*",
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
        println!(
            " {mark} {name:<16} {:<28} via {}{suffix}",
            m.model, m.provider
        );
    }

    let default = cfg.default_model.as_deref().unwrap_or("<none>");
    println!("\n  default_model = {default}");

    match &active {
        ModelSelection::Named(_) => {
            println!("  ( * marks the model a plain run would use )");
        }
        ModelSelection::Default => {
            println!(
                "  warning: default_model `{default}` is not defined; \
                 a plain run falls back to llm.model = {}",
                cfg.llm.model
            );
        }
        ModelSelection::Literal(_) | ModelSelection::Unknown(_) => {}
    }
}

/// Render one agent event to stdout/stderr for the non-interactive mode.
pub fn print_event(ev: AgentEvent) {
    match ev {
        AgentEvent::Text(t) => {
            print!("{t}");
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
        AgentEvent::Reasoning(_) => {}
        AgentEvent::Usage { .. } => {}
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

/// List recent sessions as `id  when  cwd (n entries)` lines.
///
/// # Errors
/// Propagates database errors from the session store.
pub fn print_sessions(db_path: &std::path::Path) -> crate::error::Result<()> {
    let rows = crate::session::Session::list(db_path, 20)?;
    if rows.is_empty() {
        println!("no sessions yet");
    }
    for (id, ts, info) in rows {
        let when = chrono::DateTime::from_timestamp(ts, 0)
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();
        println!("{id}  {when}  {info}");
    }
    Ok(())
}

/// Combine `-i`, `--instruction-file`, and `-t` into one instruction string.
pub fn assemble_instruction(
    parts: &[String],
    file: Option<&std::path::Path>,
    target: Option<&str>,
) -> crate::error::Result<String> {
    let mut instruction = parts.join("\n\n");
    if let Some(path) = file {
        let text = std::fs::read_to_string(path)?;
        if !instruction.is_empty() {
            instruction.push_str("\n\n");
        }
        instruction.push_str(&text);
    }
    if let Some(target) = target {
        if !instruction.is_empty() {
            instruction.push('\n');
        }
        instruction.push_str(&format!("Target: {target}"));
    }
    Ok(instruction)
}

/// Resolve `--resume <id>` / `--continue` into a session id to attach to.
///
/// Returns `Ok(None)` when neither flag was given, so a fresh session is
/// created. `--continue` with no stored session prints a notice and also
/// yields `None`.
///
/// # Errors
/// Propagates database errors from the session store.
pub fn resolve_resume(
    db_path: &std::path::Path,
    resume: Option<&str>,
    cont: bool,
) -> crate::error::Result<Option<String>> {
    match (resume, cont) {
        (Some(id), _) => {
            if !crate::session::Session::exists(db_path, id)? {
                return Err(crate::error::Error::Session(format!(
                    "no session {id} in this database"
                )));
            }
            Ok(Some(id.to_string()))
        }
        (None, true) => match crate::session::Session::latest(db_path)? {
            Some(id) => Ok(Some(id)),
            None => {
                eprintln!("soracode: no previous session to continue");
                Ok(None)
            }
        },
        (None, false) => Ok(None),
    }
}
