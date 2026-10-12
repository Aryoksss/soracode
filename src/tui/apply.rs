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
                    .push(Line2::Tool(format!("{name}({short})")));
            }
            AgentEvent::ToolEnd { name, output } => {
                if matches!(name.as_str(), "edit" | "write") {
                    self.push_file_change(&name, &output);
                    return;
                }
                let mut rows = output.lines();
                let first = rows.next().unwrap_or("").to_string();
                let extra = rows.count();
                let text = if extra > 0 {
                    format!("{first}  (+{extra} more lines)")
                } else {
                    first
                };
                let _ = name;
                self.transcript.push(Line2::Result(text));
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
                self.busy_since = None;
                self.status.clear();
                self.auto_scroll = true;
                if self.show_sidebar {
                    super::sidebar::refresh(self);
                }
            }
            AgentEvent::Cancelled => {
                self.flush_stream();
                self.busy = false;
                self.busy_since = None;
                self.status = "cancelled".into();
                self.auto_scroll = true;
            }
            AgentEvent::Paused => {
                self.flush_stream();
                self.busy = false;
                self.busy_since = None;
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
        let _ = name;
        self.transcript.push(Line2::Result(head));
        for line in changes {
            self.transcript.push(Line2::Diff(line));
        }
    }
}
