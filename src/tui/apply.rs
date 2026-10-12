//! Applying agent events to the UI state.

use super::{App, Line2};
use crate::agent::AgentEvent;

impl App {
    pub(super) fn apply(&mut self, ev: AgentEvent) {
        match ev {
            AgentEvent::Text(t) => {
                self.current_stream.push_str(&t);
            }
            AgentEvent::Reasoning(r) => {
                self.thinking.push_str(&r);
            }
            AgentEvent::ToolStart { name, args } => {
                self.flush_stream();
                let short = if args.len() > 160 {
                    format!("{}…", &args[..160])
                } else {
                    args
                };
                self.transcript
                    .push(Line2::Tool(format!("▶ {name} {short}")));
            }
            AgentEvent::ToolEnd { name, output } => {
                if matches!(name.as_str(), "edit" | "write") {
                    self.push_file_change(&name, &output);
                    return;
                }
                let first = output.lines().next().unwrap_or("").to_string();
                let extra = output.lines().count().saturating_sub(1);
                let text = if extra > 0 {
                    format!("◀ {name}: {first}  (+{extra} more lines)")
                } else {
                    format!("◀ {name}: {first}")
                };
                self.transcript.push(Line2::Info(text));
            }
            AgentEvent::SubagentStart { label } => {
                self.transcript
                    .push(Line2::Info(format!("⇢ subagent `{label}` started")));
            }
            AgentEvent::Usage { prompt, completion } => {
                self.prompt_tokens = prompt;
                self.completion_tokens = completion;
            }
            AgentEvent::Info(m) => self.transcript.push(Line2::Info(m)),
            AgentEvent::Error(m) => self.transcript.push(Line2::Error(m)),
            AgentEvent::Done => {
                self.flush_stream();
                self.busy = false;
                self.status =
                    "ready — Enter send  Tab sessions  ↑↓ history  F2 copy  / commands".into();
                self.auto_scroll = true;
                if self.show_sidebar {
                    super::sidebar::refresh(self);
                }
            }
            AgentEvent::Cancelled => {
                self.flush_stream();
                self.busy = false;
                self.status = "cancelled".into();
                self.auto_scroll = true;
            }
            AgentEvent::Paused => {
                self.flush_stream();
                self.busy = false;
                self.transcript.push(Line2::Info(
                    "paused — /resume clears the pause, then send again to continue".into(),
                ));
                self.status = "paused".into();
                self.auto_scroll = true;
            }
        }
    }

    fn flush_stream(&mut self) {
        if !self.current_stream.trim().is_empty() {
            self.transcript
                .push(Line2::Assistant(self.current_stream.clone()));
        }
        self.current_stream.clear();
    }

    /// Render a `write`/`edit` result as a header plus a colored diff.
    fn push_file_change(&mut self, name: &str, output: &str) {
        let (head, changes) = super::diff::file_change(output);
        self.transcript
            .push(Line2::Tool(format!("◀ {name}: {head}")));
        for line in changes {
            self.transcript.push(Line2::Diff(line));
        }
    }
}
