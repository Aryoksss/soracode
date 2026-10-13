//! Slash-command palette: the command registry and its actions.

/// What selecting a command does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    /// Open the model picker (a second-level palette).
    PickModel,
    /// Render the detected host tools into the transcript.
    ShowTools,
    /// Render the engagement dashboard.
    Status,
    /// Render discovered hosts and services.
    Targets,
    /// Render every service across hosts, as a port matrix.
    Services,
    /// Render the access and reachability graph.
    Matrix,
    /// Render recorded findings.
    Vulns,
    /// Render discovered credentials.
    Creds,
    /// Render recent sessions into the transcript.
    ShowSessions,
    /// Stop the agent before its next tool call.
    Pause,
    /// Clear the pause so the agent may continue.
    Resume,
    /// Ask the next turn to compact the conversation harder.
    Compact,
    /// Switch the autonomy mode (auto/guided/free).
    SetMode,
    /// Turn the camouflage routing of every instruction on or off.
    Camouflage,
    /// Render the command list and key bindings into the transcript.
    ShowHelp,
    /// Remove the last submitted turn from the session and transcript.
    Undo,
    /// Write the full session transcript to a Markdown file.
    Export,
    /// Clear the transcript pane.
    Clear,
    /// Leave the TUI.
    Quit,
}

/// One entry in the palette.
pub struct Command {
    /// The word typed after `/`, without the slash.
    pub name: &'static str,
    /// One-line description shown beside the name.
    pub desc: &'static str,
    /// What selection does.
    pub action: Action,
}

/// Every available command, in display order.
pub const COMMANDS: &[Command] = &[
    Command {
        name: "model",
        desc: "switch the model for this session",
        action: Action::PickModel,
    },
    Command {
        name: "tools",
        desc: "list tools detected on this host",
        action: Action::ShowTools,
    },
    Command {
        name: "status",
        desc: "engagement dashboard: hosts, vulns, creds",
        action: Action::Status,
    },
    Command {
        name: "targets",
        desc: "discovered hosts and services",
        action: Action::Targets,
    },
    Command {
        name: "services",
        desc: "every service across hosts, as a port matrix",
        action: Action::Services,
    },
    Command {
        name: "matrix",
        desc: "access and reachability graph",
        action: Action::Matrix,
    },
    Command {
        name: "vulns",
        desc: "recorded findings by severity",
        action: Action::Vulns,
    },
    Command {
        name: "creds",
        desc: "discovered credentials",
        action: Action::Creds,
    },
    Command {
        name: "sessions",
        desc: "list recent sessions",
        action: Action::ShowSessions,
    },
    Command {
        name: "pause",
        desc: "stop the agent before its next tool call",
        action: Action::Pause,
    },
    Command {
        name: "resume",
        desc: "clear the pause and let the agent continue",
        action: Action::Resume,
    },
    Command {
        name: "compact",
        desc: "trim older turns before the next message",
        action: Action::Compact,
    },
    Command {
        name: "mode",
        desc: "show or set the autonomy mode (auto|guided|free)",
        action: Action::SetMode,
    },
    Command {
        name: "camouflage",
        desc: "route every instruction through a cloaked ask (on|off)",
        action: Action::Camouflage,
    },
    Command {
        name: "help",
        desc: "show commands and key bindings",
        action: Action::ShowHelp,
    },
    Command {
        name: "undo",
        desc: "remove the last turn from the session and view",
        action: Action::Undo,
    },
    Command {
        name: "export",
        desc: "write the session to a Markdown file (/export [path])",
        action: Action::Export,
    },
    Command {
        name: "clear",
        desc: "clear the transcript",
        action: Action::Clear,
    },
    Command {
        name: "quit",
        desc: "exit soracode",
        action: Action::Quit,
    },
];

/// Commands matching the prefix by name, then those matching it in the description.
#[must_use]
pub fn matching(prefix: &str) -> Vec<&'static Command> {
    let p = prefix.trim_start_matches('/').to_ascii_lowercase();
    let mut named: Vec<&'static Command> = Vec::new();
    let mut described: Vec<&'static Command> = Vec::new();
    for c in COMMANDS {
        if c.name.starts_with(p.as_str()) {
            named.push(c);
        } else if !p.is_empty() && c.desc.to_ascii_lowercase().contains(p.as_str()) {
            described.push(c);
        }
    }
    named.extend(described);
    named
}

/// Resolve an exact command name, as typed on submit.
#[must_use]
pub fn by_name(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.name == name)
}

#[cfg(test)]
mod tests;
