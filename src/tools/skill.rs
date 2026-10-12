//! `skill` — list and load on-demand knowledge packs.

use async_trait::async_trait;
use serde_json::{json, Value};

use super::{arg_str_opt, Tool, ToolContext};
use crate::error::Result;

/// Lists available skill packs or loads one in full.
pub struct SkillTool {
    ctx: ToolContext,
}

impl SkillTool {
    /// Build the tool for the given context.
    #[must_use]
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &'static str {
        "skill"
    }

    fn description(&self) -> &'static str {
        "Load a knowledge pack that teaches one offensive technique in depth (Active Directory \
         attacks, web exploitation, privilege escalation, and so on). Call with no arguments to \
         list the packs; call with `name` to load that pack's full text. Load a pack only when a \
         task calls for the depth it provides."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of the pack to load. Omit to list what is available."
                }
            }
        })
    }

    async fn run(&self, args: Value) -> Result<String> {
        let workspace = &self.ctx.workspace;
        match arg_str_opt(&args, "name") {
            None => {
                let skills = crate::skills::discover(workspace);
                if skills.is_empty() {
                    return Ok("no skill packs installed".into());
                }
                let mut s = String::from("skill packs:\n");
                for sk in skills {
                    s.push_str(&format!("  {} — {}\n", sk.name, sk.summary));
                }
                Ok(s)
            }
            Some(name) => crate::skills::load(workspace, &name).map_err(crate::error::Error::Tool),
        }
    }
}
