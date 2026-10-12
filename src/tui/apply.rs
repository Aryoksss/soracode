//! Applying agent events to the UI state.

use super::{App, Line2};
use crate::agent::AgentEvent;

impl App {
    /// Fold one agent event into the view. Returns the next queued instruction
    /// to run, when a turn finishes with one waiting.
    pub(super) fn apply(&mut self, ev: AgentEvent) -> Option<String> {
        match ev {
            AgentEvent::Text(t) => {
                self.current_stream.push_str(&t);
                None
            }
            AgentEvent::Reasoning(r) => {
                self.thinking.push_str(&r);
                None
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
                self.current_tool = Some(format!("{name}({short})"));
                None
            }
            AgentEvent::ToolEnd { name, output } => {
                self.current_tool = None;
                if matches!(name.as_str(), "edit" | "write") {
                    self.push_file_change(&output);
                } else {
                    let mut rows = output.lines();
                    let first = rows.next().unwrap_or("").to_string();
                    let extra = rows.count();
                    let text = if extra > 0 {
                        format!("{first}  (+{extra} more lines)")
                    } else {
                        first
                    };
                    self.transcript.push(Line2::Result(text));
                }
                None
            }
            AgentEvent::SubagentStart { label } => {
                self.transcript
                    .push(Line2::Info(format!("⇢ subagent `{label}` started")));
                None
            }
            AgentEvent::Usage { prompt, completion } => {
                self.prompt_tokens = prompt;
                self.completion_tokens = completion;
                None
            }
            AgentEvent::Info(m) => {
                self.transcript.push(Line2::Info(m));
                None
            }
            AgentEvent::Error(m) => {
                self.transcript.push(Line2::Error(m));
                None
            }
            AgentEvent::Done => {
                let next = self.finish("");
                if next.is_none() {
                    self.status.clear();
                }
                next
            }
            AgentEvent::Cancelled => {
                self.finish("cancelled");
                self.restore_queued();
                None
            }
            AgentEvent::Paused => {
                self.finish("paused");
                self.transcript.push(Line2::Info(
                    "paused — /resume clears the pause, then send again to continue".into(),
                ));
                self.restore_queued();
                None
            }
        }
    }

    /// Close out a turn: flush the stream, drop the busy flag, and hand back any
    /// instruction the operator queued while it ran.
    fn finish(&mut self, status: &str) -> Option<String> {
        self.flush_stream();
        self.busy = false;
        self.current_tool = None;
        self.busy_since = None;
        self.status = status.to_string();
        self.auto_scroll = true;
        if self.show_sidebar {
            super::sidebar::refresh(self);
        }
        self.queued.take()
    }

    /// Return a queued instruction to the prompt after an interrupted turn.
    fn restore_queued(&mut self) {
        if let Some(q) = self.queued.take() {
            self.input = q;
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
    fn push_file_change(&mut self, output: &str) {
        let (head, changes) = super::diff::file_change(output);
        self.transcript.push(Line2::Result(head));
        for line in changes {
            self.transcript.push(Line2::Diff(line));
        }
    }
}
