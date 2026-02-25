use std::collections::{BTreeMap, HashSet};
use std::process::Command;

use anyhow::{Context, Result};

use crate::cli::Args;
use crate::config::ResolvedConfig;
use crate::cop::autocorrect_allowlist::AutocorrectAllowlist;
use crate::cop::registry::CopRegistry;
use crate::cop::tiers::TierMap;
use crate::diagnostic::Diagnostic;
use crate::fs::discover_files;
use crate::linter::run_linter;

// ---------- RuboCop JSON structures ----------

#[derive(serde::Deserialize)]
struct RubocopOutput {
    files: Vec<RubocopFile>,
}

#[derive(serde::Deserialize)]
struct RubocopFile {
    path: String,
    offenses: Vec<RubocopOffense>,
}

#[derive(serde::Deserialize)]
struct RubocopOffense {
    cop_name: String,
    location: RubocopLocation,
}

#[derive(serde::Deserialize)]
struct RubocopLocation {
    start_line: usize,
}

// ---------- Result types ----------

#[derive(Debug, serde::Serialize)]
pub struct VerifyResult {
    pub nitrocop_count: usize,
    pub rubocop_count: usize,
    pub matches: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub match_rate: f64,
    pub per_cop: BTreeMap<String, CopStats>,
}

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct CopStats {
    pub matches: usize,
    pub fp: usize,
    #[serde(rename = "fn")]
    pub fn_: usize,
}

// ---------- Offense tuple ----------

type Offense = (String, usize, String); // (path, line, cop_name)

fn normalize_path(path: &str) -> &str {
    path.strip_prefix("./").unwrap_or(path)
}

fn diagnostics_to_set(diagnostics: &[Diagnostic]) -> HashSet<Offense> {
    diagnostics
        .iter()
        .map(|d| {
            let path = normalize_path(&d.path);
            (path.to_string(), d.location.line, d.cop_name.clone())
        })
        .collect()
}

fn rubocop_to_set(output: &RubocopOutput, covered: &HashSet<&str>) -> HashSet<Offense> {
    output
        .files
        .iter()
        .flat_map(|f| {
            let path = normalize_path(&f.path);
            f.offenses.iter().filter_map(move |o| {
                if covered.contains(o.cop_name.as_str()) {
                    Some((path.to_string(), o.location.start_line, o.cop_name.clone()))
                } else {
                    None
                }
            })
        })
        .collect()
}

// ---------- Core verify logic ----------

pub fn run_verify(
    args: &Args,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    tier_map: &TierMap,
    allowlist: &AutocorrectAllowlist,
) -> Result<VerifyResult> {
    // 1. Run nitrocop internally
    let discovered = discover_files(&args.paths, config)?;
    let lint_result = run_linter(&discovered, config, registry, args, tier_map, allowlist);
    let nitrocop_set = diagnostics_to_set(&lint_result.diagnostics);

    // 2. Run RuboCop subprocess
    let rubocop_json = run_rubocop(args)?;
    let rubocop_output: RubocopOutput =
        serde_json::from_str(&rubocop_json).context("Failed to parse RuboCop JSON output")?;

    // 3. Build covered cop set
    let covered: HashSet<&str> = registry.cops().iter().map(|c| c.name()).collect();
    let rubocop_set = rubocop_to_set(&rubocop_output, &covered);

    // 4. Compute set operations
    let matches: HashSet<&Offense> = nitrocop_set.intersection(&rubocop_set).collect();
    let fps: HashSet<&Offense> = nitrocop_set.difference(&rubocop_set).collect();
    let fns: HashSet<&Offense> = rubocop_set.difference(&nitrocop_set).collect();
    let total = nitrocop_set.union(&rubocop_set).count();
    let match_rate = if total == 0 {
        100.0
    } else {
        matches.len() as f64 / total as f64 * 100.0
    };

    // 5. Per-cop breakdown
    let mut per_cop: BTreeMap<String, CopStats> = BTreeMap::new();
    for (_, _, cop) in &matches {
        per_cop.entry(cop.clone()).or_default().matches += 1;
    }
    for (_, _, cop) in &fps {
        per_cop.entry(cop.clone()).or_default().fp += 1;
    }
    for (_, _, cop) in &fns {
        per_cop.entry(cop.clone()).or_default().fn_ += 1;
    }

    Ok(VerifyResult {
        nitrocop_count: nitrocop_set.len(),
        rubocop_count: rubocop_set.len(),
        matches: matches.len(),
        false_positives: fps.len(),
        false_negatives: fns.len(),
        match_rate,
        per_cop,
    })
}

// ---------- Subprocess ----------

fn run_rubocop(args: &Args) -> Result<String> {
    let parts: Vec<&str> = args.rubocop_cmd.split_whitespace().collect();
    if parts.is_empty() {
        anyhow::bail!("--rubocop-cmd is empty");
    }

    let program = parts[0];

    // Check if the program is available
    let which_result = Command::new("which").arg(program).output();
    match which_result {
        Ok(output) if !output.status.success() => {
            anyhow::bail!(
                "Could not find '{}'. Is Ruby/Bundler installed and in PATH?\n\
                 Hint: --verify requires RuboCop to be installed in the target project.\n\
                 Run `bundle install` in the target directory first.",
                program
            );
        }
        Err(_) => {
            anyhow::bail!(
                "Could not find '{}'. Is Ruby/Bundler installed and in PATH?",
                program
            );
        }
        _ => {}
    }

    let mut cmd = Command::new(program);
    for part in &parts[1..] {
        cmd.arg(part);
    }
    cmd.arg("--format").arg("json").arg("--no-color");
    for path in &args.paths {
        cmd.arg(path);
    }

    eprintln!("Running RuboCop...");
    let output = cmd
        .output()
        .with_context(|| format!("Failed to execute '{}'", args.rubocop_cmd))?;

    // RuboCop exits 0 (clean) or 1 (offenses found) — both are fine.
    // Exit code 2+ means an error.
    let code = output.status.code().unwrap_or(127);
    if code >= 2 {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "RuboCop exited with code {} (expected 0 or 1).\nstderr: {}",
            code,
            stderr.trim()
        );
    }

    let stdout = String::from_utf8(output.stdout).context("RuboCop output was not valid UTF-8")?;
    Ok(stdout)
}

// ---------- Output ----------

pub fn print_text(result: &VerifyResult) {
    println!("nitrocop verify:");
    println!("  nitrocop: {} offenses", result.nitrocop_count);
    println!("  rubocop:  {} offenses", result.rubocop_count);
    println!("  matches:  {} ({:.1}%)", result.matches, result.match_rate);
    println!("  FP:       {} (nitrocop-only)", result.false_positives);
    println!("  FN:       {} (rubocop-only)", result.false_negatives);

    // Per-cop diffs (only cops with FP or FN, sorted by total diffs descending)
    let mut diffs: Vec<(&String, &CopStats)> = result
        .per_cop
        .iter()
        .filter(|(_, s)| s.fp > 0 || s.fn_ > 0)
        .collect();
    diffs.sort_by(|a, b| (b.1.fp + b.1.fn_).cmp(&(a.1.fp + a.1.fn_)));

    if !diffs.is_empty() {
        println!();
        println!("  Per-cop diffs (top 20):");
        println!("  {:<40} {:>4} {:>4}", "Cop", "FP", "FN");
        for (cop, stats) in diffs.iter().take(20) {
            println!("  {:<40} {:>4} {:>4}", cop, stats.fp, stats.fn_);
        }
    }
}

pub fn print_json(result: &VerifyResult) {
    let json = serde_json::to_string_pretty(result).expect("Failed to serialize verify result");
    println!("{json}");
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_strips_dot_slash() {
        assert_eq!(normalize_path("./foo/bar.rb"), "foo/bar.rb");
        assert_eq!(normalize_path("foo/bar.rb"), "foo/bar.rb");
        assert_eq!(normalize_path("./bar.rb"), "bar.rb");
        assert_eq!(normalize_path("bar.rb"), "bar.rb");
    }

    #[test]
    fn rubocop_json_parsing() {
        let json = r#"{
            "files": [
                {
                    "path": "app/models/user.rb",
                    "offenses": [
                        {
                            "cop_name": "Style/StringLiterals",
                            "message": "Prefer single quotes",
                            "location": { "start_line": 5, "start_column": 1, "last_line": 5, "last_column": 10, "length": 10 }
                        },
                        {
                            "cop_name": "Layout/TrailingWhitespace",
                            "message": "Trailing whitespace",
                            "location": { "start_line": 10, "start_column": 1, "last_line": 10, "last_column": 5, "length": 5 }
                        }
                    ]
                },
                {
                    "path": "lib/helper.rb",
                    "offenses": []
                }
            ]
        }"#;

        let output: RubocopOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.files.len(), 2);
        assert_eq!(output.files[0].offenses.len(), 2);
        assert_eq!(output.files[0].offenses[0].cop_name, "Style/StringLiterals");
        assert_eq!(output.files[0].offenses[0].location.start_line, 5);
        assert_eq!(output.files[1].offenses.len(), 0);
    }

    #[test]
    fn rubocop_to_set_filters_covered_cops() {
        let json = r#"{
            "files": [
                {
                    "path": "test.rb",
                    "offenses": [
                        {
                            "cop_name": "CoveredCop",
                            "message": "msg",
                            "location": { "start_line": 1, "start_column": 1, "last_line": 1, "last_column": 1, "length": 1 }
                        },
                        {
                            "cop_name": "UncoveredCop",
                            "message": "msg",
                            "location": { "start_line": 2, "start_column": 1, "last_line": 2, "last_column": 1, "length": 1 }
                        }
                    ]
                }
            ]
        }"#;

        let output: RubocopOutput = serde_json::from_str(json).unwrap();
        let covered: HashSet<&str> = ["CoveredCop"].into_iter().collect();
        let set = rubocop_to_set(&output, &covered);
        assert_eq!(set.len(), 1);
        assert!(set.contains(&("test.rb".to_string(), 1, "CoveredCop".to_string())));
    }

    #[test]
    fn diagnostics_to_set_normalizes_paths() {
        let diags = vec![Diagnostic {
            path: "./foo/bar.rb".to_string(),
            location: crate::diagnostic::Location { line: 3, column: 0 },
            severity: crate::diagnostic::Severity::Convention,
            cop_name: "Style/Test".to_string(),
            message: "msg".to_string(),
            corrected: false,
        }];
        let set = diagnostics_to_set(&diags);
        assert_eq!(set.len(), 1);
        assert!(set.contains(&("foo/bar.rb".to_string(), 3, "Style/Test".to_string())));
    }
}
