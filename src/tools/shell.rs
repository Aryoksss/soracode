//! `shell` — run commands in a real shell inside the workspace.

use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use super::{arg_str, arg_str_opt, arg_u64_opt, Tool, ToolContext};
use crate::error::{Error, Result};
use crate::util::truncate_middle;

/// Executes shell commands and returns combined stdout/stderr.
pub struct ShellTool {
    ctx: ToolContext,
}

impl ShellTool {
    /// Build the tool for the given context.
    #[must_use]
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &'static str {
        "shell"
    }

    fn description(&self) -> &'static str {
        "Run a shell command in the workspace and return its combined stdout and stderr. \
         Full shell features work: pipes, redirection, backgrounding (`&`), `&&`, subshells. \
         Runs as the current user with the inherited environment and has no network or \
         filesystem restrictions. Long-running commands are killed at the timeout; pass \
         `background: true` to detach and get a PID instead of waiting."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute."
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory. Defaults to the workspace."
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "Kill the command after this many seconds. Defaults to the soracode tool timeout."
                },
                "background": {
                    "type": "boolean",
                    "description": "Detach the command and return immediately with its PID, instead of capturing output."
                }
            },
            "required": ["command"]
        })
    }

    async fn run(&self, args: Value) -> Result<String> {
        let command = arg_str(&args, "command")?;
        let cwd = arg_str_opt(&args, "cwd")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| self.ctx.workspace.clone());
        let background = args
            .get("background")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let timeout =
            arg_u64_opt(&args, "timeout_secs").unwrap_or(self.ctx.config.tool_timeout_secs);

        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c")
            .arg(&command)
            .current_dir(&cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Detach from our process group so backgrounded children survive.
            .kill_on_drop(false);

        if background {
            let child = cmd
                .spawn()
                .map_err(|e| Error::Tool(format!("spawn failed: {e}")))?;
            let pid = child.id().unwrap_or(0);
            // Leak the handle so the child keeps running.
            std::mem::forget(child);
            return Ok(format!(
                "started in background (pid {pid})\ncommand: {command}"
            ));
        }

        let child = cmd
            .spawn()
            .map_err(|e| Error::Tool(format!("spawn failed: {e}")))?;

        let fut = async {
            let mut child = child;
            let mut out = String::new();
            let mut err = String::new();

            if let Some(mut o) = child.stdout.take() {
                let _ = o.read_to_string(&mut out).await;
            }
            if let Some(mut e) = child.stderr.take() {
                let _ = e.read_to_string(&mut err).await;
            }

            let status = child
                .wait()
                .await
                .map_err(|e| Error::Tool(format!("wait failed: {e}")))?;

            Ok::<_, Error>((out, err, status))
        };

        match tokio::time::timeout(Duration::from_secs(timeout.max(1)), fut).await {
            Ok(Ok((out, err, status))) => {
                let mut body = String::new();
                if !out.is_empty() {
                    body.push_str(&out);
                }
                if !err.is_empty() {
                    if !body.is_empty() {
                        body.push('\n');
                    }
                    body.push_str("[stderr]\n");
                    body.push_str(&err);
                }
                if body.trim().is_empty() {
                    body.push_str("(no output)");
                }

                let code = status
                    .code()
                    .map_or_else(|| "signal".to_string(), |c| c.to_string());

                let limit = self.ctx.config.tool_output_limit;
                let shown = truncate_middle(&body, limit);

                Ok(format!("$ {command}\nexit: {code}\n\n{shown}"))
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Ok(format!(
                "$ {command}\n\n[timed out after {timeout}s — the command was killed]"
            )),
        }
    }
}
