//! `--doctor` command: debug/support output.
//!
//! Displays baseline versions, config root + inheritance chain,
//! gem version mismatch warnings, and the skip summary.

use std::collections::BTreeMap;
use std::path::Path;

use crate::config::ResolvedConfig;
use crate::cop::registry::CopRegistry;
use crate::cop::tiers::TierMap;

/// Load embedded baseline versions from resources/baseline.json.
fn load_baseline() -> BTreeMap<String, String> {
    serde_json::from_str(include_str!("resources/baseline.json"))
        .expect("resources/baseline.json should be valid JSON")
}

/// Parse the full version string (e.g. "1.84.2") from a Gemfile.lock for a gem.
fn parse_gem_version_from_lockfile(content: &str, gem_name: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(gem_name) {
            if let Some(ver_str) = rest.strip_prefix(" (") {
                if let Some(ver_str) = ver_str.strip_suffix(')') {
                    return Some(ver_str.to_string());
                }
            }
        }
    }
    None
}

/// Read the config YAML and extract inherit_from / inherit_gem entries.
fn read_inheritance_chain(config_dir: &Path) -> (Vec<String>, Vec<(String, Vec<String>)>) {
    let mut inherit_from = Vec::new();
    let mut inherit_gem = Vec::new();

    let config_path = config_dir.join(".rubocop.yml");
    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return (inherit_from, inherit_gem),
    };
    let doc: serde_yml::Value = match serde_yml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return (inherit_from, inherit_gem),
    };
    let Some(map) = doc.as_mapping() else {
        return (inherit_from, inherit_gem);
    };

    // inherit_from
    if let Some(val) = map.get(serde_yml::Value::String("inherit_from".into())) {
        match val {
            serde_yml::Value::String(s) => inherit_from.push(s.clone()),
            serde_yml::Value::Sequence(seq) => {
                for item in seq {
                    if let Some(s) = item.as_str() {
                        inherit_from.push(s.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    // inherit_gem
    if let Some(val) = map.get(serde_yml::Value::String("inherit_gem".into())) {
        if let Some(gem_map) = val.as_mapping() {
            for (k, v) in gem_map {
                let Some(gem_name) = k.as_str() else {
                    continue;
                };
                let mut paths = Vec::new();
                match v {
                    serde_yml::Value::String(s) => paths.push(s.clone()),
                    serde_yml::Value::Sequence(seq) => {
                        for item in seq {
                            if let Some(s) = item.as_str() {
                                paths.push(s.to_string());
                            }
                        }
                    }
                    _ => {}
                }
                inherit_gem.push((gem_name.to_string(), paths));
            }
        }
    }

    (inherit_from, inherit_gem)
}

/// Run the doctor command and print to stdout.
pub fn run_doctor(
    config: &ResolvedConfig,
    registry: &CopRegistry,
    tier_map: &TierMap,
    target_dir: Option<&Path>,
) {
    let baseline = load_baseline();

    // 1. Baseline versions
    println!("Baseline versions (vendored):");
    for (gem, version) in &baseline {
        println!("  {gem} {version}");
    }

    // 2. Config root + inheritance chain
    println!();
    if let Some(dir) = config.config_dir() {
        println!("Config root: {}", dir.display());
        let (inherit_from, inherit_gem) = read_inheritance_chain(dir);
        if !inherit_from.is_empty() {
            println!("  inherit_from:");
            for path in &inherit_from {
                println!("    - {path}");
            }
        }
        if !inherit_gem.is_empty() {
            println!("  inherit_gem:");
            for (gem, paths) in &inherit_gem {
                for path in paths {
                    println!("    - {gem}: {path}");
                }
            }
        }
    } else {
        println!("Config root: (none — using defaults)");
    }

    // 3. Gem version mismatch warnings
    let lockfile_content = find_and_read_lockfile(config.config_dir(), target_dir);
    println!();
    if let Some(ref content) = lockfile_content {
        let mut any_mismatch = false;
        println!("Installed gem versions (from Gemfile.lock):");
        for (gem, baseline_ver) in &baseline {
            if let Some(installed_ver) = parse_gem_version_from_lockfile(content, gem) {
                let marker = if installed_ver != *baseline_ver {
                    any_mismatch = true;
                    " ← MISMATCH"
                } else {
                    ""
                };
                println!("  {gem} {installed_ver} (baseline: {baseline_ver}){marker}");
            }
        }
        if any_mismatch {
            println!();
            println!(
                "warning: Gem version mismatches may cause different cop availability or behavior."
            );
            println!(
                "  nitrocop targets the baseline versions above. Run `nitrocop --migrate` for details."
            );
        }
    } else {
        println!("Installed gem versions: (no Gemfile.lock found)");
    }

    // 4. Skip summary
    let summary = config.compute_skip_summary(registry, tier_map, false);
    println!();
    if summary.is_empty() {
        println!("Skip summary: no skipped cops");
    } else {
        println!("Skip summary: {} cops skipped", summary.total());
        if !summary.preview_gated.is_empty() {
            println!(
                "  Preview-gated: {} (requires --preview)",
                summary.preview_gated.len()
            );
        }
        if !summary.unimplemented.is_empty() {
            println!(
                "  Unimplemented: {} (in baseline, not in nitrocop)",
                summary.unimplemented.len()
            );
        }
        if !summary.outside_baseline.is_empty() {
            println!(
                "  Outside baseline: {} (custom or unknown)",
                summary.outside_baseline.len()
            );
        }
    }

    // 5. Registry info
    println!();
    println!("Registry: {} cops registered", registry.len());
    let autocorrectable = registry
        .cops()
        .iter()
        .filter(|c| c.supports_autocorrect())
        .count();
    println!("  {} support autocorrect", autocorrectable);
}

/// Find Gemfile.lock by checking config_dir, target_dir, or current directory.
fn find_and_read_lockfile(config_dir: Option<&Path>, target_dir: Option<&Path>) -> Option<String> {
    let candidates: Vec<&Path> = [config_dir, target_dir, Some(Path::new("."))]
        .into_iter()
        .flatten()
        .collect();
    for dir in candidates {
        let path = dir.join("Gemfile.lock");
        if let Ok(content) = std::fs::read_to_string(&path) {
            return Some(content);
        }
        let path = dir.join("gems.locked");
        if let Ok(content) = std::fs::read_to_string(&path) {
            return Some(content);
        }
    }
    None
}
