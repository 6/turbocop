//! `--migrate` command: config analysis without linting.
//!
//! Classifies every enabled cop into one of four buckets (stable, preview,
//! unimplemented, outside_baseline) and reports counts + top examples.

use std::collections::{BTreeMap, HashSet};

use serde::Serialize;

use crate::cli::Args;
use crate::config::ResolvedConfig;
use crate::cop::registry::CopRegistry;
use crate::cop::tiers::TierMap;

/// Per-cop classification status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CopStatus {
    Stable,
    Preview,
    Unimplemented,
    OutsideBaseline,
}

/// Full migrate report.
#[derive(Debug, Serialize)]
pub struct MigrateReport {
    pub baseline: BTreeMap<String, String>,
    pub counts: MigrateCounts,
    pub cops: Vec<CopEntry>,
}

#[derive(Debug, Serialize)]
pub struct MigrateCounts {
    pub stable: usize,
    pub preview: usize,
    pub unimplemented: usize,
    pub outside_baseline: usize,
}

#[derive(Debug, Serialize)]
pub struct CopEntry {
    pub name: String,
    pub status: CopStatus,
}

/// Load embedded baseline versions from resources/baseline.json.
fn load_baseline() -> BTreeMap<String, String> {
    serde_json::from_str(include_str!("resources/baseline.json"))
        .expect("resources/baseline.json should be valid JSON")
}

/// Classify all enabled cops and build the migrate report.
pub fn build_report(
    config: &ResolvedConfig,
    registry: &CopRegistry,
    tier_map: &TierMap,
) -> MigrateReport {
    let registry_names: HashSet<&str> = registry.cops().iter().map(|c| c.name()).collect();
    let summary = config.compute_skip_summary(registry, tier_map, false);

    let preview_set: HashSet<&str> = summary.preview_gated.iter().map(|s| s.as_str()).collect();
    let unimplemented_set: HashSet<&str> =
        summary.unimplemented.iter().map(|s| s.as_str()).collect();
    let outside_set: HashSet<&str> = summary
        .outside_baseline
        .iter()
        .map(|s| s.as_str())
        .collect();

    let mut cops = Vec::new();

    // Classify every enabled cop
    let mut enabled_names = config.enabled_cop_names();
    enabled_names.sort();

    for name in &enabled_names {
        let status = if preview_set.contains(name.as_str()) {
            CopStatus::Preview
        } else if unimplemented_set.contains(name.as_str()) {
            CopStatus::Unimplemented
        } else if outside_set.contains(name.as_str()) {
            CopStatus::OutsideBaseline
        } else if registry_names.contains(name.as_str()) {
            // In registry and not skipped → stable (will run)
            CopStatus::Stable
        } else {
            // Shouldn't happen, but classify as outside_baseline
            CopStatus::OutsideBaseline
        };
        cops.push(CopEntry {
            name: name.clone(),
            status,
        });
    }

    let counts = MigrateCounts {
        stable: cops
            .iter()
            .filter(|c| c.status == CopStatus::Stable)
            .count(),
        preview: cops
            .iter()
            .filter(|c| c.status == CopStatus::Preview)
            .count(),
        unimplemented: cops
            .iter()
            .filter(|c| c.status == CopStatus::Unimplemented)
            .count(),
        outside_baseline: cops
            .iter()
            .filter(|c| c.status == CopStatus::OutsideBaseline)
            .count(),
    };

    MigrateReport {
        baseline: load_baseline(),
        counts,
        cops,
    }
}

/// Print migrate report as text to stdout.
pub fn print_text(report: &MigrateReport, args: &Args) {
    // Baseline
    println!("Baseline versions:");
    for (gem, version) in &report.baseline {
        println!("  {gem} {version}");
    }
    println!();

    // Counts
    println!("Enabled cops: {}", report.cops.len());
    println!(
        "  Stable:           {:>4}  (runs by default)",
        report.counts.stable
    );
    println!(
        "  Preview:          {:>4}  (requires --preview)",
        report.counts.preview
    );
    println!(
        "  Unimplemented:    {:>4}  (in baseline, not in nitrocop)",
        report.counts.unimplemented
    );
    println!(
        "  Outside baseline: {:>4}  (custom or unknown)",
        report.counts.outside_baseline
    );

    // Top examples per non-stable category (up to 5 each)
    let max_examples = 5;

    let preview_cops: Vec<&str> = report
        .cops
        .iter()
        .filter(|c| c.status == CopStatus::Preview)
        .map(|c| c.name.as_str())
        .collect();
    if !preview_cops.is_empty() {
        println!();
        println!(
            "Preview-gated cops (top {}):",
            preview_cops.len().min(max_examples)
        );
        for name in preview_cops.iter().take(max_examples) {
            println!("  - {name}");
        }
        if preview_cops.len() > max_examples {
            println!("  ... and {} more", preview_cops.len() - max_examples);
        }
    }

    let unimpl_cops: Vec<&str> = report
        .cops
        .iter()
        .filter(|c| c.status == CopStatus::Unimplemented)
        .map(|c| c.name.as_str())
        .collect();
    if !unimpl_cops.is_empty() {
        println!();
        println!(
            "Unimplemented cops (top {}):",
            unimpl_cops.len().min(max_examples)
        );
        for name in unimpl_cops.iter().take(max_examples) {
            println!("  - {name}");
        }
        if unimpl_cops.len() > max_examples {
            println!("  ... and {} more", unimpl_cops.len() - max_examples);
        }
    }

    let outside_cops: Vec<&str> = report
        .cops
        .iter()
        .filter(|c| c.status == CopStatus::OutsideBaseline)
        .map(|c| c.name.as_str())
        .collect();
    if !outside_cops.is_empty() {
        println!();
        println!(
            "Outside-baseline cops (top {}):",
            outside_cops.len().min(max_examples)
        );
        for name in outside_cops.iter().take(max_examples) {
            println!("  - {name}");
        }
        if outside_cops.len() > max_examples {
            println!("  ... and {} more", outside_cops.len() - max_examples);
        }
    }

    // Suggested CI command
    println!();
    let skipped =
        report.counts.preview + report.counts.unimplemented + report.counts.outside_baseline;
    if skipped == 0 {
        println!("All enabled cops are stable. No migration needed.");
    } else {
        println!("Suggested CI command:");
        if report.counts.preview > 0
            && report.counts.unimplemented == 0
            && report.counts.outside_baseline == 0
        {
            println!("  nitrocop --strict .");
            println!("  # Fails if preview-gated cops are skipped. Add --preview to run them.");
        } else if report.counts.unimplemented > 0 || report.counts.outside_baseline > 0 {
            if args.strict.as_deref() == Some("all") {
                println!("  nitrocop --strict=all .");
                println!(
                    "  # Fails if any cops are skipped (preview + unimplemented + outside baseline)."
                );
            } else {
                println!("  nitrocop --strict .");
                println!(
                    "  # Fails only for preview-gated cops. Use --strict=all to include unimplemented."
                );
            }
        }
    }
}

/// Print migrate report as JSON to stdout.
pub fn print_json(report: &MigrateReport) {
    println!(
        "{}",
        serde_json::to_string_pretty(report).expect("MigrateReport should be serializable")
    );
}
