//! Tests for prompt construction and specialist role selection.

use super::{for_role, mode_block, specialist_preamble, SUBAGENT_PROMPT};
use crate::config::Mode;

#[test]
fn root_and_subagent_get_distinct_base_prompts() {
    assert_ne!(for_role(false), for_role(true));
    assert_eq!(for_role(true), SUBAGENT_PROMPT);
}

#[test]
fn known_roles_resolve_to_their_preamble() {
    for role in ["recon", "enum", "exploit", "ad", "web", "report"] {
        let text = specialist_preamble(role).unwrap_or_else(|| panic!("{role} missing"));
        assert!(!text.is_empty());
    }
    assert!(specialist_preamble("recon")
        .unwrap()
        .contains("reconnaissance"));
    assert!(specialist_preamble("ad")
        .unwrap()
        .contains("Active Directory"));
}

#[test]
fn unknown_role_has_no_preamble() {
    assert!(specialist_preamble("").is_none());
    assert!(specialist_preamble("nonsense").is_none());
}

#[test]
fn operating_prompt_teaches_engagement_state_use() {
    let p = for_role(false);
    assert!(p.contains("state_query"));
    assert!(p.contains("state_update"));
    assert!(p.contains("nmap_parse"));
    assert!(p.contains("nuclei_parse"));
    assert!(p.contains("cme_parse"));
}

#[test]
fn each_mode_has_distinct_guidance() {
    let auto = mode_block(Mode::Auto);
    let guided = mode_block(Mode::Guided);
    let free = mode_block(Mode::Free);
    assert_ne!(auto, guided);
    assert_ne!(guided, free);
    assert!(guided.contains("wait for the operator"));
    assert!(free.contains("No engagement bookkeeping"));
}
