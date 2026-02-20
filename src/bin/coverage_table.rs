//! Generate a Markdown coverage table showing turbocop cop coverage vs vendor RuboCop.
//!
//! Usage:
//!   cargo run --bin coverage_table                                  # print to stdout
//!   cargo run --bin coverage_table -- --show-missing                # include missing cop lists
//!   cargo run --bin coverage_table -- --output docs/coverage.md     # write to file

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;

#[derive(Parser)]
#[command(about = "Generate turbocop cop coverage table")]
struct Args {
    /// Output file path (default: stdout)
    #[arg(long)]
    output: Option<PathBuf>,

    /// Show missing cops per department
    #[arg(long)]
    show_missing: bool,
}

/// Mapping from vendor directory name to the departments it defines (not overrides).
struct VendorSource {
    dir: &'static str,
    owned_departments: &'static [&'static str],
}

static VENDOR_SOURCES: &[VendorSource] = &[
    VendorSource {
        dir: "rubocop",
        owned_departments: &[
            "Layout", "Lint", "Style", "Metrics", "Naming",
            "Security", "Bundler", "Gemspec", "Migration",
        ],
    },
    VendorSource { dir: "rubocop-rails", owned_departments: &["Rails"] },
    VendorSource { dir: "rubocop-performance", owned_departments: &["Performance"] },
    VendorSource { dir: "rubocop-rspec", owned_departments: &["RSpec"] },
    VendorSource { dir: "rubocop-rspec_rails", owned_departments: &["RSpecRails"] },
    VendorSource { dir: "rubocop-factory_bot", owned_departments: &["FactoryBot"] },
];

/// Repo display order for conformance table (matches bench_turbocop REPOS order).
const REPO_ORDER: &[&str] = &[
    "mastodon", "discourse", "rails", "rubocop", "chatwoot", "errbit",
    "activeadmin", "good_job", "docuseal", "rubygems.org", "doorkeeper", "fat_free_crm",
];

// --- Conformance JSON types (must match bench_turbocop) ---

#[derive(serde::Deserialize)]
struct ConformResult {
    turbocop_count: usize,
    rubocop_count: usize,
    matches: usize,
    false_positives: usize,
    false_negatives: usize,
    match_rate: f64,
    per_cop: BTreeMap<String, CopStats>,
}

#[derive(serde::Deserialize)]
struct CopStats {
    matches: usize,
    fp: usize,
    #[serde(rename = "fn")]
    fn_: usize,
}

// --- Vendor YAML parsing ---

fn parse_vendor_cops(vendor_dir: &Path, source: &VendorSource) -> BTreeMap<String, BTreeSet<String>> {
    let yml_path = vendor_dir.join(source.dir).join("config").join("default.yml");
    let mut dept_cops: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    let content = match fs::read_to_string(&yml_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: could not read {}: {e}", yml_path.display());
            return dept_cops;
        }
    };

    for line in content.lines() {
        if !line.starts_with(|c: char| c.is_ascii_uppercase()) { continue; }
        let Some(colon) = line.find(':') else { continue; };
        let key = &line[..colon];
        let Some(slash) = key.find('/') else { continue; };
        let dept = &key[..slash];
        if source.owned_departments.iter().any(|&d| d == dept) {
            dept_cops.entry(dept.to_string()).or_default().insert(key.to_string());
        }
    }
    dept_cops
}

fn display_dept(dept: &str) -> &str {
    match dept {
        "RSpecRails" => "RSpecRails (gem)",
        _ => dept,
    }
}

fn dept_order(dept: &str) -> usize {
    match dept {
        "Layout" => 0, "Lint" => 1, "Style" => 2, "Metrics" => 3, "Naming" => 4,
        "Security" => 5, "Bundler" => 6, "Gemspec" => 7, "Migration" => 8,
        "Rails" => 9, "Performance" => 10, "RSpec" => 11, "RSpecRails" => 12,
        "FactoryBot" => 13, _ => 99,
    }
}

// --- Conformance section from bench/conform.json ---

fn render_conformance(project_root: &Path, cop_count: usize) -> Option<String> {
    let json_path = project_root.join("bench").join("conform.json");
    let content = fs::read_to_string(&json_path).ok()?;
    let data: HashMap<String, ConformResult> = serde_json::from_str(&content).ok()?;

    if data.is_empty() { return None; }

    let mut out = String::new();
    writeln!(out, "## Conformance").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "Location-level comparison: file + line + cop_name. Only cops implemented by turbocop ({cop_count}) are compared.").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "| Repo | turbocop | rubocop | Matches | FP (turbocop only) | FN (rubocop only) | Match rate |").unwrap();
    writeln!(out, "|------|-------:|--------:|--------:|-----------------:|------------------:|-----------:|").unwrap();

    for &repo in REPO_ORDER {
        if let Some(c) = data.get(repo) {
            writeln!(out, "| {repo} | {} | {} | {} | {} | {} | **{:.1}%** |",
                c.turbocop_count, c.rubocop_count, c.matches,
                c.false_positives, c.false_negatives, c.match_rate,
            ).unwrap();
        }
    }
    writeln!(out).unwrap();

    // Per-cop divergence details
    for &repo in REPO_ORDER {
        if let Some(c) = data.get(repo) {
            let mut divergent: Vec<(&String, &CopStats)> = c.per_cop.iter()
                .filter(|(_, s)| s.fp > 0 || s.fn_ > 0)
                .collect();
            divergent.sort_by_key(|(_, s)| std::cmp::Reverse(s.fp + s.fn_));

            if divergent.is_empty() {
                writeln!(out, "**{repo}:** All cops match perfectly!").unwrap();
                writeln!(out).unwrap();
                continue;
            }

            let shown = divergent.len().min(30);
            writeln!(out, "<details>").unwrap();
            writeln!(out, "<summary>Divergent cops \u{2014} {repo} ({shown} of {} shown)</summary>", divergent.len()).unwrap();
            writeln!(out).unwrap();
            writeln!(out, "| Cop | Matches | FP | FN |").unwrap();
            writeln!(out, "|-----|--------:|---:|---:|").unwrap();
            for (cop, stats) in divergent.iter().take(30) {
                writeln!(out, "| {cop} | {} | {} | {} |", stats.matches, stats.fp, stats.fn_).unwrap();
            }
            writeln!(out).unwrap();
            writeln!(out, "</details>").unwrap();
            writeln!(out).unwrap();
        }
    }

    Some(out)
}

// --- Main ---

fn main() {
    let args = Args::parse();
    let project_root = std::env::current_dir().expect("could not get current dir");
    let vendor_dir = project_root.join("vendor");

    // 1. Vendor cops per department
    let mut vendor_cops: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for source in VENDOR_SOURCES {
        for (dept, cops) in parse_vendor_cops(&vendor_dir, source) {
            vendor_cops.entry(dept).or_default().extend(cops);
        }
    }

    // 2. turbocop cops per department from registry
    let registry = turbocop::cop::registry::CopRegistry::default_registry();
    let mut turbocop_cops: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for name in registry.names() {
        if let Some(slash) = name.find('/') {
            turbocop_cops.entry(name[..slash].to_string()).or_default().insert(name.to_string());
        }
    }

    // 3. Sorted departments
    let mut all_depts: BTreeSet<String> = BTreeSet::new();
    all_depts.extend(vendor_cops.keys().cloned());
    all_depts.extend(turbocop_cops.keys().cloned());
    let mut sorted_depts: Vec<String> = all_depts.into_iter().collect();
    sorted_depts.sort_by_key(|d| dept_order(d));

    // 4. Generate cop coverage table
    let mut out = String::new();
    writeln!(out, "# turbocop Coverage Report").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "> Auto-generated by `cargo run --bin coverage_table`. Do not edit manually.").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "## Cop Coverage").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "**turbocop cops:** {}", registry.len()).unwrap();
    writeln!(out).unwrap();
    writeln!(out, "| Department | RuboCop | turbocop | Coverage |").unwrap();
    writeln!(out, "|------------|--------:|-------:|---------:|").unwrap();

    let mut total_v = 0usize;
    let mut total_r = 0usize;

    for dept in &sorted_depts {
        let v = vendor_cops.get(dept).map_or(0, |s| s.len());
        let r = turbocop_cops.get(dept).map_or(0, |s| s.len());
        total_v += v;
        total_r += r;
        let pct = if r >= v && v > 0 { "**100%**".to_string() }
            else if v > 0 { format!("{:.0}%", r as f64 / v as f64 * 100.0) }
            else { "100%".to_string() };
        writeln!(out, "| {} | {v} | {r} | {pct} |", display_dept(dept)).unwrap();
    }

    let total_pct = if total_v > 0 { total_r as f64 / total_v as f64 * 100.0 } else { 0.0 };
    writeln!(out, "| **Total** | **{total_v}** | **{total_r}** | **{total_pct:.1}%** |").unwrap();

    // 5. Missing cops
    if args.show_missing {
        writeln!(out).unwrap();
        writeln!(out, "## Missing Cops").unwrap();
        writeln!(out).unwrap();
        let mut any = false;
        for dept in &sorted_depts {
            if let Some(vs) = vendor_cops.get(dept) {
                let rs = turbocop_cops.get(dept).cloned().unwrap_or_default();
                let missing: Vec<_> = vs.difference(&rs).collect();
                if !missing.is_empty() {
                    any = true;
                    writeln!(out, "### {} ({} missing)", display_dept(dept), missing.len()).unwrap();
                    writeln!(out).unwrap();
                    for cop in &missing { writeln!(out, "- `{cop}`").unwrap(); }
                    writeln!(out).unwrap();
                }
            }
        }
        if !any { writeln!(out, "All vendor cops are implemented.").unwrap(); }

        let mut extras: Vec<String> = Vec::new();
        for dept in &sorted_depts {
            let vs = vendor_cops.get(dept).cloned().unwrap_or_default();
            if let Some(rs) = turbocop_cops.get(dept) {
                extras.extend(rs.difference(&vs).cloned());
            }
        }
        if !extras.is_empty() {
            writeln!(out).unwrap();
            writeln!(out, "### Extra Cops (in turbocop, not in vendor)").unwrap();
            writeln!(out).unwrap();
            for cop in &extras { writeln!(out, "- `{cop}`").unwrap(); }
            writeln!(out).unwrap();
        }
    }

    // 6. Conformance (from bench/conform.json)
    writeln!(out).unwrap();
    if let Some(conform) = render_conformance(&project_root, registry.len()) {
        out.push_str(&conform);
    } else {
        writeln!(out, "## Conformance").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "*No data. Run `cargo run --release --bin bench_turbocop -- conform` to generate bench/conform.json.*").unwrap();
    }

    // 7. Output
    if let Some(path) = args.output {
        let abs = if path.is_absolute() { path } else { project_root.join(path) };
        if let Some(p) = abs.parent() { fs::create_dir_all(p).ok(); }
        fs::write(&abs, &out).expect("failed to write output file");
        eprintln!("Wrote {}", abs.display());
    } else {
        print!("{out}");
    }
}
