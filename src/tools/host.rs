//! `tools` — report what is installed on this host, and rescan on demand.

use async_trait::async_trait;
use serde_json::{json, Value};

use super::catalog;
use super::{arg_str_opt, Tool, ToolContext};
use crate::error::Result;

/// Lists the host tool inventory, optionally refreshing it.
pub struct ToolsTool {
    #[allow(dead_code)] // held for registry-uniform construction; run() needs no context
    ctx: ToolContext,
}

impl ToolsTool {
    /// Build the tool for the given context.
    #[must_use]
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for ToolsTool {
    fn name(&self) -> &'static str {
        "tools"
    }

    fn description(&self) -> &'static str {
        "List the security and system tools actually installed on this host, grouped by \
         category and with detected versions. Pass `rescan` true to re-check `PATH` after \
         installing something. Use this instead of guessing whether a tool exists."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "rescan": {
                    "type": "boolean",
                    "description": "Re-walk PATH and probe versions again, refreshing the cache."
                },
                "filter": {
                    "type": "string",
                    "description": "Only show tools whose name or description contains this text."
                }
            }
        })
    }

    async fn run(&self, args: Value) -> Result<String> {
        let filter = arg_str_opt(&args, "filter").map(|f| f.to_lowercase());
        let rescan = args.get("rescan").and_then(Value::as_bool).unwrap_or(false);

        let inv = if rescan {
            catalog::rescan().await
        } else {
            catalog::inventory().await
        };

        let Some(filter) = filter else {
            return Ok(inv.render());
        };

        let mut out = String::new();
        for t in &inv.found {
            let hay = format!("{} {}", t.bin, t.about).to_lowercase();
            if !hay.contains(&filter) {
                continue;
            }
            match &t.version {
                Some(v) => out.push_str(&format!(
                    "{:<22} {:<12} {}  [{}]\n",
                    t.bin,
                    v,
                    t.about,
                    t.path.display()
                )),
                None => out.push_str(&format!(
                    "{:<22} {:<12} {}  [{}]\n",
                    t.bin,
                    "-",
                    t.about,
                    t.path.display()
                )),
            }
        }
        for m in &inv.missing {
            if m.to_lowercase().contains(&filter) {
                out.push_str(&format!("{m:<22} {:<12} (not installed)\n", "-"));
            }
        }

        if out.is_empty() {
            return Ok(format!("no tool matches `{filter}`"));
        }
        Ok(out)
    }
}
