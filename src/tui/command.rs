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

/// Commands whose name starts with the typed prefix, in registry order.
#[must_use]
pub fn matching(prefix: &str) -> Vec<&'static Command> {
    let p = prefix.trim_start_matches('/');
    COMMANDS.iter().filter(|c| c.name.starts_with(p)).collect()
}

/// Resolve an exact command name, as typed on submit.
#[must_use]
pub fn by_name(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_has_a_distinct_name() {
        let mut names: Vec<&str> = COMMANDS.iter().map(|c| c.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), COMMANDS.len());
    }

    #[test]
    fn names_and_descriptions_are_non_empty() {
        for c in COMMANDS {
            assert!(!c.name.is_empty(), "empty command name");
            assert!(!c.desc.is_empty(), "empty description for {}", c.name);
            assert!(
                !c.name.contains('/'),
                "name must not contain a slash: {}",
                c.name
            );
        }
    }

    #[test]
    fn matching_accepts_prefix_with_and_without_slash() {
        let bare = matching("model");
        let slashed = matching("/model");
        assert_eq!(bare.len(), 1);
        assert_eq!(bare[0].name, "model");
        assert_eq!(slashed.len(), 1);
        assert_eq!(slashed[0].name, "model");
    }

    #[test]
    fn matching_returns_every_prefix_candidate() {
        let names: Vec<&str> = matching("mod").into_iter().map(|c| c.name).collect();
        assert_eq!(names, vec!["model", "mode"]);
    }

    #[test]
    fn pause_resume_and_mode_are_registered() {
        assert_eq!(by_name("pause").map(|c| c.action), Some(Action::Pause));
        assert_eq!(by_name("resume").map(|c| c.action), Some(Action::Resume));
        assert_eq!(by_name("mode").map(|c| c.action), Some(Action::SetMode));
    }

    #[test]
    fn matching_includes_all_commands_for_bare_slash() {
        assert_eq!(matching("/").len(), COMMANDS.len());
    }

    #[test]
    fn matching_is_empty_for_unknown_prefix() {
        assert!(matching("zzz").is_empty());
    }

    #[test]
    fn by_name_resolves_and_rejects() {
        assert_eq!(by_name("quit").map(|c| c.action), Some(Action::Quit));
        assert_eq!(by_name("model").map(|c| c.action), Some(Action::PickModel));
        assert!(by_name("quit ").is_none());
        assert!(by_name("/quit").is_none());
    }

    #[test]
    fn dashboard_commands_resolve() {
        assert_eq!(by_name("status").map(|c| c.action), Some(Action::Status));
        assert_eq!(by_name("targets").map(|c| c.action), Some(Action::Targets));
        assert_eq!(
            by_name("services").map(|c| c.action),
            Some(Action::Services)
        );
        assert_eq!(by_name("matrix").map(|c| c.action), Some(Action::Matrix));
        assert_eq!(by_name("vulns").map(|c| c.action), Some(Action::Vulns));
        assert_eq!(by_name("creds").map(|c| c.action), Some(Action::Creds));
    }
}
