//! `--rules` command: list all cops nitrocop knows about.
//!
//! Shows name, tier, implementation status, baseline presence, and default enabled.

use std::collections::{BTreeMap, HashSet};

use serde::Serialize;

use crate::cop::registry::CopRegistry;
use crate::cop::tiers::{Tier, TierMap};

/// Embedded baseline cops: cop name → default enabled (true/false).
/// Parsed from vendor config/default.yml files.
fn load_baseline_cops() -> BTreeMap<String, bool> {
    serde_json::from_str(include_str!("resources/baseline_cops.json"))
        .expect("resources/baseline_cops.json should be valid JSON")
}

#[derive(Debug, Serialize)]
pub struct RuleEntry {
    pub name: String,
    pub tier: String,
    pub implemented: bool,
    pub in_baseline: bool,
    pub default_enabled: bool,
}

/// Build the full rules list (union of registry + baseline cops).
pub fn build_rules(
    registry: &CopRegistry,
    tier_map: &TierMap,
    tier_filter: Option<&str>,
) -> Vec<RuleEntry> {
    let baseline = load_baseline_cops();
    let registry_names: HashSet<&str> = registry.cops().iter().map(|c| c.name()).collect();

    // Union of all cop names (registry + baseline)
    let mut all_names: Vec<String> = registry_names
        .iter()
        .map(|s| s.to_string())
        .chain(
            baseline
                .keys()
                .filter(|k| !registry_names.contains(k.as_str()))
                .cloned(),
        )
        .collect();
    all_names.sort();

    let mut rules = Vec::new();
    for name in &all_names {
        let tier = tier_map.tier_for(name);
        let tier_str = match tier {
            Tier::Stable => "stable",
            Tier::Preview => "preview",
        };

        // Apply tier filter
        if let Some(filter) = tier_filter {
            if tier_str != filter {
                continue;
            }
        }

        let implemented = registry_names.contains(name.as_str());
        let in_baseline = baseline.contains_key(name.as_str());
        let default_enabled = baseline.get(name.as_str()).copied().unwrap_or(false);

        rules.push(RuleEntry {
            name: name.clone(),
            tier: tier_str.to_string(),
            implemented,
            in_baseline,
            default_enabled,
        });
    }

    rules
}

/// Print rules as a table to stdout.
pub fn print_table(rules: &[RuleEntry]) {
    // Header
    println!(
        "{:<45} {:<8} {:<12} {:<10} Default",
        "Name", "Tier", "Implemented", "Baseline"
    );
    println!("{}", "-".repeat(85));

    for rule in rules {
        let impl_mark = if rule.implemented { "yes" } else { "-" };
        let baseline_mark = if rule.in_baseline { "yes" } else { "-" };
        let default_mark = if rule.default_enabled { "yes" } else { "-" };
        println!(
            "{:<45} {:<8} {:<12} {:<10} {}",
            rule.name, rule.tier, impl_mark, baseline_mark, default_mark
        );
    }

    // Summary
    println!();
    let total = rules.len();
    let implemented = rules.iter().filter(|r| r.implemented).count();
    let in_baseline = rules.iter().filter(|r| r.in_baseline).count();
    let preview = rules.iter().filter(|r| r.tier == "preview").count();
    println!(
        "{total} cops total, {implemented} implemented, {in_baseline} in baseline, {preview} preview-tier"
    );
}

/// Print rules as JSON to stdout.
pub fn print_json(rules: &[RuleEntry]) {
    println!(
        "{}",
        serde_json::to_string_pretty(rules).expect("RuleEntry should be serializable")
    );
}
