//! The `/help` screen: every command and key binding, grouped and aligned.

use super::{command, Line2};

/// Key bindings shown under the command list, as `(keys, action)`.
const KEYS: &[(&str, &str)] = &[
    ("Enter", "send"),
    ("Shift+Enter, Ctrl-J", "new line"),
    ("↑ / ↓", "walk back through instructions"),
    ("Esc", "cancel the running turn"),
    ("Ctrl-U", "clear the input"),
    ("Ctrl-Y", "copy the last reply to the clipboard"),
    ("PageUp / PageDown", "scroll the transcript"),
    ("Home / End", "jump to the top or the newest line"),
    ("Tab", "open the session list"),
    ("Ctrl-B", "toggle the sidebar"),
    ("Ctrl-T", "toggle model thinking"),
    ("Ctrl-L", "clear the transcript"),
    ("F2", "toggle mouse-wheel capture (drag selects either way)"),
    ("Ctrl-C", "cancel the turn; twice to quit"),
    ("Ctrl-Q", "quit"),
];

/// Build the lines for the help screen.
pub(super) fn screen() -> Vec<Line2> {
    let mut out = vec![
        Line2::Heading("help".into()),
        Line2::Info(String::new()),
        Line2::Heading(format!("commands ({})", command::COMMANDS.len())),
    ];
    for c in command::COMMANDS {
        out.push(Line2::Info(format!(
            "  {:<10}  {}",
            format!("/{}", c.name),
            c.desc
        )));
    }
    out.push(Line2::Info(String::new()));
    out.push(Line2::Heading("keys".into()));
    for (keys, action) in KEYS {
        out.push(Line2::Info(format!("  {keys:<19}  {action}")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::screen;
    use crate::tui::command;
    use crate::tui::Line2;

    #[test]
    fn help_lists_every_command_and_key() {
        let lines = screen();
        let text: Vec<String> = lines
            .iter()
            .map(|l| match l {
                Line2::Heading(t) | Line2::Info(t) => t.clone(),
                _ => String::new(),
            })
            .collect();
        for c in command::COMMANDS {
            assert!(
                text.iter().any(|l| l.contains(&format!("/{}", c.name))),
                "help omits /{}",
                c.name
            );
        }
        assert!(text.iter().any(|l| l.contains("Ctrl-Y")));
    }
}
