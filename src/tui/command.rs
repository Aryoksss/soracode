//! Slash-command palette: the command registry and its actions.

/// What selecting a command does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    /// Open the model picker (a second-level palette).
    PickModel,
    /// Render the detected host tools into the transcript.
    ShowTools,
    /// Render recent sessions into the transcript.
    ShowSessions,
    /// Render the command list and key bindings into the transcript.
    ShowHelp,
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
        name: "sessions",
        desc: "list recent sessions",
        action: Action::ShowSessions,
    },
    Command {
        name: "help",
        desc: "show commands and key bindings",
        action: Action::ShowHelp,
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
        let bare = matching("mod");
        let slashed = matching("/mod");
        assert_eq!(bare.len(), 1);
        assert_eq!(bare[0].name, "model");
        assert_eq!(slashed.len(), 1);
        assert_eq!(slashed[0].name, "model");
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
}
