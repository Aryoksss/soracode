//! Command-line surface: flags, aliases, and their documented meanings.

use std::path::PathBuf;

use clap::Parser;

/// A lightweight, fully operator-controlled security agent.
#[derive(Parser, Debug)]
#[command(name = "soracode", version, about, long_about = None)]
pub struct Cli {
    /// Instruction to execute. May be given multiple times.
    #[arg(short = 'i', long = "instruction")]
    pub instruction: Vec<String>,

    /// Read the instruction from a file.
    #[arg(long = "instruction-file")]
    pub instruction_file: Option<PathBuf>,

    /// Target shorthand: appended to the instruction as "Target: <value>".
    #[arg(short = 't', long = "target")]
    pub target: Option<String>,

    /// Model override. Either a name from the models file or a raw model string.
    #[arg(short = 'm', long = "model")]
    pub model: Option<String>,

    /// API base URL override.
    #[arg(long = "base-url")]
    pub base_url: Option<String>,

    /// Wire dialect override: openai (default), anthropic, or google.
    #[arg(long = "wire")]
    pub wire: Option<String>,

    /// API key override.
    #[arg(long = "api-key")]
    pub api_key: Option<String>,

    /// Workspace directory tools operate in.
    #[arg(short = 'w', long = "workspace")]
    pub workspace: Option<PathBuf>,

    /// Autonomy mode: auto (full, default), guided (confirm before acting), or free (no state).
    #[arg(long = "mode")]
    pub mode: Option<String>,

    /// Config file to load on top of the defaults.
    #[arg(short = 'c', long = "config")]
    pub config: Option<PathBuf>,

    /// Resume an existing session by id.
    #[arg(long = "resume")]
    pub resume: Option<String>,

    /// Resume the most recent session in this workspace.
    #[arg(long = "continue", visible_alias = "cont")]
    pub cont: bool,

    /// List recent sessions and exit.
    #[arg(long = "sessions")]
    pub sessions: bool,

    /// List the tools detected on this host and exit.
    #[arg(long = "tools")]
    pub tools: bool,

    /// With --tools or --print-prompt, re-walk PATH instead of using the cache.
    #[arg(long = "rescan")]
    pub rescan: bool,

    /// List configured models and providers, then exit.
    #[arg(long = "models")]
    pub models: bool,

    /// Write a starter models file to the user config directory.
    #[arg(long = "init-models")]
    pub init_models: bool,

    /// Do not start the TUI; print events to stdout and exit.
    #[arg(short = 'n', long = "non-interactive")]
    pub non_interactive: bool,

    /// Print the resolved system prompt and exit (debugging aid).
    #[arg(long = "print-prompt")]
    pub print_prompt: bool,
}
