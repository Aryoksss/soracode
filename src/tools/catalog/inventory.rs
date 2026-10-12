//! Rendering an inventory into prompt text and operator summaries.

use std::collections::BTreeMap;

use super::{Category, FoundTool, Inventory};

impl Inventory {
    /// Render the inventory as prompt text.
    #[must_use]
    pub fn render(&self) -> String {
        if self.found.is_empty() && self.missing.is_empty() {
            return String::new();
        }

        let mut by_cat: BTreeMap<Category, Vec<&FoundTool>> = BTreeMap::new();
        for t in &self.found {
            by_cat.entry(t.category).or_default().push(t);
        }

        let mut s = String::new();
        s.push_str("\n## Tools installed on this host\n\n");
        s.push_str(
            "These were discovered by resolving `PATH` on the machine you are running on. \
             Treat this as the authoritative list of what exists here: use a tool if it is \
             listed, and do not assume a tool exists if it is not.\n\n",
        );

        for cat in Category::order() {
            let Some(tools) = by_cat.get(&cat) else {
                continue;
            };
            s.push_str(&format!("### {}\n", cat.heading()));
            for t in tools {
                match &t.version {
                    Some(v) => s.push_str(&format!("- `{}` ({}) — {}\n", t.bin, v, t.about)),
                    None => s.push_str(&format!("- `{}` — {}\n", t.bin, t.about)),
                }
            }
            s.push('\n');
        }

        if !self.missing.is_empty() {
            s.push_str("### Not installed\n\n");
            s.push_str(
                "Looked for and absent. Do not try to run these; find another route or \
                 install one if the task genuinely needs it.\n\n",
            );
            let names: Vec<String> = self.missing.iter().map(|m| format!("`{m}`")).collect();
            s.push_str(&names.join(", "));
            s.push_str("\n\n");
        }

        s.push_str(&format!(
            "Total: {} tools available, {} absent.\n",
            self.found.len(),
            self.missing.len()
        ));
        s
    }
}

/// A short operator-facing summary, used at startup and by the `tools` tool.
#[must_use]
pub fn summarize(inv: &Inventory) -> String {
    format!(
        "{} tools available on this host, {} absent",
        inv.found.len(),
        inv.missing.len()
    )
}
