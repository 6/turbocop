//! Benchmark rblint vs rubocop on real-world codebases.
//!
//! Usage:
//!   cargo run --release --bin bench_rblint          # full run (bench + conform + report)
//!   cargo run --release --bin bench_rblint -- bench  # timing only
//!   cargo run --release --bin bench_rblint -- conform # conformance only
//!   cargo run --release --bin bench_rblint -- report  # regenerate results.md from cached data

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use clap::Parser;

// --- CLI ---

#[derive(Parser)]
#[command(about = "Benchmark rblint vs rubocop. Writes results to bench/results.md.")]
struct Args {
    /// Subcommand: bench, conform, report, or omit for all
    #[arg(default_value = "all")]
    mode: String,

    /// Number of hyperfine runs per benchmark
    #[arg(long, default_value_t = 5)]
    runs: u32,

    /// Hyperfine warmup runs
    #[arg(long, default_value_t = 2)]
    warmup: u32,

    /// Output markdown file path (relative to project root)
    #[arg(long)]
    output: Option<PathBuf>,
}

// --- Repo config ---

struct BenchRepo {
    name: &'static str,
    url: &'static str,
    tag: &'static str,
}

static REPOS: &[BenchRepo] = &[
    BenchRepo {
        name: "mastodon",
        url: "https://github.com/mastodon/mastodon.git",
        tag: "v4.3.4",
    },
    BenchRepo {
        name: "discourse",
        url: "https://github.com/discourse/discourse.git",
        tag: "v3.4.3",
    },
    BenchRepo {
        name: "rails",
        url: "https://github.com/rails/rails.git",
        tag: "v8.1.2",
    },
    BenchRepo {
        name: "rubocop",
        url: "https://github.com/rubocop/rubocop.git",
        tag: "v1.84.2",
    },
    BenchRepo {
        name: "chatwoot",
        url: "https://github.com/chatwoot/chatwoot.git",
        tag: "v4.10.1",
    },
    BenchRepo {
        name: "errbit",
        url: "https://github.com/errbit/errbit.git",
        tag: "v0.10.7",
    },
    BenchRepo {
        name: "activeadmin",
        url: "https://github.com/activeadmin/activeadmin.git",
        tag: "v3.4.0",
    },
    BenchRepo {
        name: "good_job",
        url: "https://github.com/bensheldon/good_job.git",
        tag: "v4.13.3",
    },
    BenchRepo {
        name: "docuseal",
        url: "https://github.com/docusealco/docuseal.git",
        tag: "2.3.4",
    },
    BenchRepo {
        name: "rubygems.org",
        url: "https://github.com/rubygems/rubygems.org.git",
        tag: "master",
    },
    BenchRepo {
        name: "doorkeeper",
        url: "https://github.com/doorkeeper-gem/doorkeeper.git",
        tag: "v5.8.2",
    },
    BenchRepo {
        name: "fat_free_crm",
        url: "https://github.com/fatfreecrm/fat_free_crm.git",
        tag: "v0.25.0",
    },
];

// --- Helpers ---

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn bench_dir() -> PathBuf {
    project_root().join("bench")
}

fn repos_dir() -> PathBuf {
    bench_dir().join("repos")
}

fn results_dir() -> PathBuf {
    bench_dir().join("results")
}

fn rblint_binary() -> PathBuf {
    project_root().join("target/release/rblint")
}

fn shell_output(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn has_command(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn count_rb_files(dir: &Path) -> usize {
    let mut count = 0;
    fn walk(dir: &Path, count: &mut usize) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().unwrap_or_default();
                if name != "vendor" && name != "node_modules" && name != ".git" {
                    walk(&path, count);
                }
            } else if path.extension().is_some_and(|e| e == "rb") {
                *count += 1;
            }
        }
    }
    walk(dir, &mut count);
    count
}

fn format_time(seconds: f64) -> String {
    if seconds >= 1.0 {
        format!("{seconds:.2}s")
    } else {
        let ms = seconds * 1000.0;
        format!("{ms:.0}ms")
    }
}

fn format_speedup(slow: f64, fast: f64) -> String {
    if fast <= 0.0 {
        return "-".to_string();
    }
    format!("{:.1}x", slow / fast)
}

// --- Setup ---

fn setup_repos() {
    let repos = repos_dir();
    fs::create_dir_all(&repos).unwrap();

    for repo in REPOS {
        let repo_path = repos.join(repo.name);
        if !repo_path.exists() {
            eprintln!("Cloning {} at {}...", repo.name, repo.tag);
            let status = Command::new("git")
                .args([
                    "clone",
                    "--depth",
                    "1",
                    "--branch",
                    repo.tag,
                    repo.url,
                    repo_path.to_str().unwrap(),
                ])
                .status()
                .expect("failed to run git");
            if !status.success() {
                eprintln!("  Failed to clone {}", repo.name);
                continue;
            }
        } else {
            eprintln!("{} already cloned.", repo.name);
        }

        eprintln!("Installing {} bundle...", repo.name);
        let status = Command::new("bundle")
            .args(["install", "--jobs", "4"])
            .current_dir(&repo_path)
            .status();

        match status {
            Ok(s) if s.success() => eprintln!("  Bundle install OK."),
            _ => {
                eprintln!("  Bundle install failed. Trying with --without production...");
                let retry = Command::new("bundle")
                    .args(["install", "--jobs", "4", "--without", "production"])
                    .current_dir(&repo_path)
                    .status();
                match retry {
                    Ok(s) if s.success() => eprintln!("  Bundle install OK (without production)."),
                    _ => eprintln!(
                        "  WARNING: bundle install failed for {}. rubocop may not work.",
                        repo.name
                    ),
                }
            }
        }
    }
}

// --- Build ---

fn build_rblint() {
    eprintln!("Building rblint (release)...");
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--bin",
            "rblint",
            "--manifest-path",
            project_root().join("Cargo.toml").to_str().unwrap(),
        ])
        .status()
        .expect("failed to run cargo build");
    assert!(status.success(), "cargo build --release failed");
}

// --- Init lockfiles ---

fn init_lockfiles() {
    let rblint = rblint_binary();
    if !rblint.exists() {
        eprintln!("rblint binary not found. Build first.");
        return;
    }

    for repo in REPOS {
        let repo_dir = repos_dir().join(repo.name);
        if !repo_dir.exists() {
            continue;
        }

        eprintln!("Generating .rblint.cache for {}...", repo.name);
        let start = Instant::now();
        let output = Command::new(rblint.as_os_str())
            .args(["--init", repo_dir.to_str().unwrap()])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .expect("failed to run rblint --init");

        if output.status.success() {
            eprintln!(
                "  OK ({:.1}s)",
                start.elapsed().as_secs_f64()
            );
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("  Failed: {}", stderr.trim());
        }
    }
}

// --- Bench ---

#[derive(serde::Deserialize)]
struct HyperfineOutput {
    results: Vec<HyperfineResult>,
}

#[derive(serde::Deserialize)]
struct HyperfineResult {
    command: String,
    mean: f64,
    stddev: f64,
    median: f64,
    min: f64,
    max: f64,
}

struct BenchResult {
    rblint: HyperfineResult,
    rubocop: HyperfineResult,
    rb_count: usize,
}

fn run_bench(args: &Args) -> HashMap<String, BenchResult> {
    let rblint = rblint_binary();
    let results_path = results_dir();
    fs::create_dir_all(&results_path).unwrap();

    if !has_command("hyperfine") {
        eprintln!("Error: hyperfine not found. Install via: mise install");
        std::process::exit(1);
    }

    let mut bench_results = HashMap::new();

    for repo in REPOS {
        let repo_dir = repos_dir().join(repo.name);
        if !repo_dir.exists() {
            eprintln!("Repo {} not found. Run setup first.", repo.name);
            continue;
        }

        let rb_count = count_rb_files(&repo_dir);
        eprintln!("\n=== Benchmarking {} ({} .rb files) ===", repo.name, rb_count);

        let json_file = results_path.join(format!("{}-bench.json", repo.name));
        let rblint_cmd = format!(
            "{} {} --no-color",
            rblint.display(),
            repo_dir.display()
        );
        let rubocop_cmd = format!(
            "cd {} && bundle exec rubocop --no-color",
            repo_dir.display()
        );

        let status = Command::new("hyperfine")
            .args([
                "--warmup",
                &args.warmup.to_string(),
                "--runs",
                &args.runs.to_string(),
                "--ignore-failure",
                "--export-json",
                json_file.to_str().unwrap(),
                "--command-name",
                "rblint",
                &rblint_cmd,
                "--command-name",
                "rubocop",
                &rubocop_cmd,
            ])
            .status()
            .expect("failed to run hyperfine");

        if !status.success() {
            eprintln!("  hyperfine failed for {}", repo.name);
            continue;
        }

        let json_content = fs::read_to_string(&json_file).unwrap();
        let parsed: HyperfineOutput = serde_json::from_str(&json_content).unwrap();

        let rblint_result = parsed
            .results
            .into_iter()
            .find(|r| r.command == "rblint")
            .unwrap();
        let rubocop_result_json = fs::read_to_string(&json_file).unwrap();
        let parsed2: HyperfineOutput = serde_json::from_str(&rubocop_result_json).unwrap();
        let rubocop_result = parsed2
            .results
            .into_iter()
            .find(|r| r.command == "rubocop")
            .unwrap();

        bench_results.insert(
            repo.name.to_string(),
            BenchResult {
                rblint: rblint_result,
                rubocop: rubocop_result,
                rb_count,
            },
        );
    }

    bench_results
}

// --- Conformance ---

#[derive(serde::Deserialize)]
struct RblintOutput {
    offenses: Vec<RblintOffense>,
}

#[derive(serde::Deserialize)]
struct RblintOffense {
    path: String,
    line: usize,
    cop_name: String,
}

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

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
struct CopStats {
    matches: usize,
    fp: usize,
    #[serde(rename = "fn")]
    fn_: usize,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ConformResult {
    rblint_count: usize,
    rubocop_count: usize,
    matches: usize,
    false_positives: usize,
    false_negatives: usize,
    match_rate: f64,
    per_cop: BTreeMap<String, CopStats>,
}

fn get_covered_cops() -> HashSet<String> {
    let rblint = rblint_binary();
    let output = Command::new(rblint.as_os_str())
        .arg("--list-cops")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .expect("failed to run rblint --list-cops");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Detect `TargetRubyVersion` from a repo's `.rubocop.yml`.
/// Returns the version as a float (e.g. 2.6, 3.1) or None if not specified.
fn detect_target_ruby_version(repo_dir: &Path) -> Option<f64> {
    let yml = repo_dir.join(".rubocop.yml");
    let content = fs::read_to_string(&yml).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("TargetRubyVersion") {
            // Parse "TargetRubyVersion: 2.6" or "TargetRubyVersion: '4.0'"
            let val = trimmed.split(':').nth(1)?.trim();
            let val = val.trim_matches(|c| c == '\'' || c == '"');
            return val.parse::<f64>().ok();
        }
    }
    None
}

/// Build the set of cops to exclude from conformance for a specific repo.
/// `Lint/Syntax` is excluded for repos targeting Ruby < 3.0 because Prism
/// always parses modern Ruby and cannot detect parser-version-specific syntax
/// errors (e.g. `...` under Ruby 2.6).
fn per_repo_excluded_cops(repo_dir: &Path) -> HashSet<String> {
    let mut excluded = HashSet::new();
    if let Some(ver) = detect_target_ruby_version(repo_dir) {
        if ver < 3.0 {
            eprintln!(
                "  TargetRubyVersion={ver} (< 3.0) — excluding Lint/Syntax from conformance"
            );
            excluded.insert("Lint/Syntax".to_string());
        }
    }
    excluded
}

fn run_conform() -> HashMap<String, ConformResult> {
    let rblint = rblint_binary();
    let results_path = results_dir();
    fs::create_dir_all(&results_path).unwrap();

    let covered = get_covered_cops();
    eprintln!("{} cops covered by rblint", covered.len());

    let mut conform_results = HashMap::new();

    for repo in REPOS {
        let repo_dir = repos_dir().join(repo.name);
        if !repo_dir.exists() {
            eprintln!("Repo {} not found. Run setup first.", repo.name);
            continue;
        }

        eprintln!("\n=== Conformance: {} ===", repo.name);

        // Run rblint in JSON mode
        eprintln!("  Running rblint...");
        let rblint_json_file = results_path.join(format!("{}-rblint.json", repo.name));
        let start = Instant::now();
        let rblint_out = Command::new(rblint.as_os_str())
            .args([
                repo_dir.to_str().unwrap(),
                "--format",
                "json",
                "--no-color",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .expect("failed to run rblint");
        fs::write(&rblint_json_file, &rblint_out.stdout).unwrap();
        eprintln!("  rblint done in {:.1}s", start.elapsed().as_secs_f64());

        // Run rubocop in JSON mode
        eprintln!("  Running rubocop...");
        let rubocop_json_file = results_path.join(format!("{}-rubocop.json", repo.name));
        let start = Instant::now();
        let rubocop_out = Command::new("bundle")
            .args(["exec", "rubocop", "--format", "json", "--no-color"])
            .current_dir(&repo_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .expect("failed to run rubocop");
        fs::write(&rubocop_json_file, &rubocop_out.stdout).unwrap();
        eprintln!(
            "  rubocop done in {:.1}s",
            start.elapsed().as_secs_f64()
        );

        // Parse and compare
        let repo_prefix = format!("{}/", repo_dir.display());

        let rblint_data: RblintOutput =
            match serde_json::from_slice(&rblint_out.stdout) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("  Failed to parse rblint JSON: {e}");
                    continue;
                }
            };

        let rubocop_data: RubocopOutput = match serde_json::from_slice(&rubocop_out.stdout) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("  Failed to parse rubocop JSON: {e}");
                continue;
            }
        };

        // Per-repo cop exclusions (e.g. Lint/Syntax for old-Ruby repos)
        let repo_excluded = per_repo_excluded_cops(&repo_dir);

        type Offense = (String, usize, String); // (path, line, cop_name)

        let rblint_set: HashSet<Offense> = rblint_data
            .offenses
            .iter()
            .filter(|o| !repo_excluded.contains(&o.cop_name))
            .map(|o| {
                let path = o.path.strip_prefix(&repo_prefix).unwrap_or(&o.path);
                // Strip leading "./" if present (rblint outputs ./path when run with ".")
                let path = path.strip_prefix("./").unwrap_or(path);
                (path.to_string(), o.line, o.cop_name.clone())
            })
            .collect();

        let rubocop_set: HashSet<Offense> = rubocop_data
            .files
            .iter()
            .flat_map(|f| {
                f.offenses.iter().filter_map(|o| {
                    if covered.contains(&o.cop_name) && !repo_excluded.contains(&o.cop_name) {
                        Some((f.path.clone(), o.location.start_line, o.cop_name.clone()))
                    } else {
                        None
                    }
                })
            })
            .collect();

        let matches: HashSet<&Offense> = rblint_set.intersection(&rubocop_set).collect();
        let fps: HashSet<&Offense> = rblint_set.difference(&rubocop_set).collect();
        let fns: HashSet<&Offense> = rubocop_set.difference(&rblint_set).collect();
        let total = rblint_set.union(&rubocop_set).count();
        let match_rate = if total == 0 {
            100.0
        } else {
            matches.len() as f64 / total as f64 * 100.0
        };

        // Per-cop breakdown
        let mut per_cop: BTreeMap<String, CopStats> = BTreeMap::new();
        for (_, _, cop) in matches.iter() {
            per_cop.entry(cop.clone()).or_default().matches += 1;
        }
        for (_, _, cop) in fps.iter() {
            per_cop.entry(cop.clone()).or_default().fp += 1;
        }
        for (_, _, cop) in fns.iter() {
            per_cop.entry(cop.clone()).or_default().fn_ += 1;
        }

        eprintln!("  rblint: {} offenses", rblint_set.len());
        eprintln!(
            "  rubocop: {} offenses (filtered to {} covered cops)",
            rubocop_set.len(),
            covered.len()
        );
        eprintln!("  matches: {}", matches.len());
        eprintln!("  FP (rblint only): {}", fps.len());
        eprintln!("  FN (rubocop only): {}", fns.len());
        eprintln!("  match rate: {:.1}%", match_rate);

        conform_results.insert(
            repo.name.to_string(),
            ConformResult {
                rblint_count: rblint_set.len(),
                rubocop_count: rubocop_set.len(),
                matches: matches.len(),
                false_positives: fps.len(),
                false_negatives: fns.len(),
                match_rate,
                per_cop,
            },
        );
    }

    conform_results
}

// --- Report generation ---

fn format_elapsed(secs: f64) -> String {
    let total_secs = secs as u64;
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    if minutes > 0 {
        format!("{}m {:02}s", minutes, seconds)
    } else {
        format!("{:.0}s", secs)
    }
}

fn generate_report(
    bench: &HashMap<String, BenchResult>,
    conform: &HashMap<String, ConformResult>,
    args: &Args,
    total_elapsed: Option<f64>,
) -> String {
    let platform = shell_output("uname", &["-sm"]);
    let date = shell_output("date", &["-u", "+%Y-%m-%d %H:%M UTC"]);

    let covered_count = if rblint_binary().exists() {
        get_covered_cops().len()
    } else {
        0
    };

    let mut md = String::new();
    writeln!(md, "# rblint Benchmark & Conformance Results").unwrap();
    writeln!(md).unwrap();
    writeln!(
        md,
        "> Auto-generated by `cargo run --release --bin bench_rblint`. Do not edit manually."
    )
    .unwrap();
    writeln!(md, "> Last updated: {date} on `{platform}`").unwrap();
    writeln!(md).unwrap();
    if covered_count > 0 {
        writeln!(md, "**rblint cops:** {covered_count}").unwrap();
    }
    writeln!(
        md,
        "**Benchmark config:** {} runs, {} warmup",
        args.runs, args.warmup
    )
    .unwrap();
    if let Some(elapsed) = total_elapsed {
        writeln!(md, "**Total benchmark time:** {}", format_elapsed(elapsed)).unwrap();
    }
    writeln!(md).unwrap();

    // --- Performance table ---
    if !bench.is_empty() {
        writeln!(md, "## Performance").unwrap();
        writeln!(md).unwrap();
        writeln!(
            md,
            "Median of {} runs via [hyperfine](https://github.com/sharkdp/hyperfine). rblint has no result cache; rubocop uses its built-in file cache (warm after hyperfine warmup runs).",
            args.runs
        )
        .unwrap();
        writeln!(md).unwrap();
        writeln!(
            md,
            "| Repo | .rb files | rblint (no cache) | rubocop (cached) | Speedup |"
        )
        .unwrap();
        writeln!(md, "|------|----------:|------------------:|-----------------:|--------:|").unwrap();

        for repo in REPOS {
            if let Some(r) = bench.get(repo.name) {
                let speedup = format_speedup(r.rubocop.median, r.rblint.median);
                writeln!(
                    md,
                    "| {} | {} | **{}** | {} | **{}** |",
                    repo.name,
                    r.rb_count,
                    format_time(r.rblint.median),
                    format_time(r.rubocop.median),
                    speedup,
                )
                .unwrap();
            }
        }

        writeln!(md).unwrap();

        // Detailed stats (collapsible)
        writeln!(
            md,
            "<details>\n<summary>Detailed timing (mean \u{00b1} stddev, min\u{2026}max)</summary>"
        )
        .unwrap();
        writeln!(md).unwrap();

        for repo in REPOS {
            if let Some(r) = bench.get(repo.name) {
                writeln!(md, "### {}", repo.name).unwrap();
                writeln!(md).unwrap();
                writeln!(md, "| Tool | Mean | Stddev | Min | Max | Median |").unwrap();
                writeln!(md, "|------|-----:|-------:|----:|----:|-------:|").unwrap();

                for (name, result) in [("rblint", &r.rblint), ("rubocop", &r.rubocop)] {
                    let bold = name == "rblint";
                    let fmt = |v: f64| -> String {
                        let s = format_time(v);
                        if bold {
                            format!("**{s}**")
                        } else {
                            s
                        }
                    };
                    writeln!(
                        md,
                        "| {}{}{} | {} | {} | {} | {} | {} |",
                        if bold { "**" } else { "" },
                        name,
                        if bold { "**" } else { "" },
                        fmt(result.mean),
                        format_time(result.stddev),
                        fmt(result.min),
                        fmt(result.max),
                        fmt(result.median),
                    )
                    .unwrap();
                }

                writeln!(md).unwrap();
            }
        }

        writeln!(md, "</details>").unwrap();
        writeln!(md).unwrap();
    }

    // --- Conformance table ---
    if !conform.is_empty() {
        writeln!(md, "## Conformance").unwrap();
        writeln!(md).unwrap();
        writeln!(
            md,
            "Location-level comparison: file + line + cop_name. Only cops implemented by rblint ({covered_count}) are compared."
        )
        .unwrap();
        writeln!(md).unwrap();
        writeln!(
            md,
            "| Repo | rblint | rubocop | Matches | FP (rblint only) | FN (rubocop only) | Match rate |"
        )
        .unwrap();
        writeln!(
            md,
            "|------|-------:|--------:|--------:|-----------------:|------------------:|-----------:|"
        )
        .unwrap();

        for repo in REPOS {
            if let Some(c) = conform.get(repo.name) {
                writeln!(
                    md,
                    "| {} | {} | {} | {} | {} | {} | **{:.1}%** |",
                    repo.name,
                    c.rblint_count,
                    c.rubocop_count,
                    c.matches,
                    c.false_positives,
                    c.false_negatives,
                    c.match_rate,
                )
                .unwrap();
            }
        }

        writeln!(md).unwrap();

        // Per-cop divergence tables
        for repo in REPOS {
            if let Some(c) = conform.get(repo.name) {
                let mut divergent: Vec<(&String, &CopStats)> = c
                    .per_cop
                    .iter()
                    .filter(|(_, s)| s.fp > 0 || s.fn_ > 0)
                    .collect();
                divergent.sort_by_key(|(_, s)| std::cmp::Reverse(s.fp + s.fn_));

                if divergent.is_empty() {
                    writeln!(md, "**{}:** All cops match perfectly!", repo.name).unwrap();
                    writeln!(md).unwrap();
                    continue;
                }

                let shown = divergent.len().min(30);
                writeln!(
                    md,
                    "<details>\n<summary>Divergent cops \u{2014} {} ({} of {} shown)</summary>",
                    repo.name,
                    shown,
                    divergent.len()
                )
                .unwrap();
                writeln!(md).unwrap();
                writeln!(md, "| Cop | Matches | FP | FN |").unwrap();
                writeln!(md, "|-----|--------:|---:|---:|").unwrap();

                for (cop, stats) in divergent.iter().take(30) {
                    writeln!(
                        md,
                        "| {} | {} | {} | {} |",
                        cop, stats.matches, stats.fp, stats.fn_
                    )
                    .unwrap();
                }

                writeln!(md).unwrap();
                writeln!(md, "</details>").unwrap();
                writeln!(md).unwrap();
            }
        }
    }

    md
}

// --- Load cached bench results from hyperfine JSON files ---

fn load_cached_bench() -> HashMap<String, BenchResult> {
    let mut results = HashMap::new();
    for repo in REPOS {
        let json_file = results_dir().join(format!("{}-bench.json", repo.name));
        if !json_file.exists() {
            continue;
        }
        let content = fs::read_to_string(&json_file).unwrap();
        let parsed: HyperfineOutput = match serde_json::from_str(&content) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let rblint_result = parsed.results.iter().find(|r| r.command == "rblint");
        let rubocop_result = parsed.results.iter().find(|r| r.command == "rubocop");

        if let (Some(rb), Some(rc)) = (rblint_result, rubocop_result) {
            let repo_dir = repos_dir().join(repo.name);
            let rb_count = if repo_dir.exists() {
                count_rb_files(&repo_dir)
            } else {
                0
            };
            results.insert(
                repo.name.to_string(),
                BenchResult {
                    rblint: HyperfineResult {
                        command: rb.command.clone(),
                        mean: rb.mean,
                        stddev: rb.stddev,
                        median: rb.median,
                        min: rb.min,
                        max: rb.max,
                    },
                    rubocop: HyperfineResult {
                        command: rc.command.clone(),
                        mean: rc.mean,
                        stddev: rc.stddev,
                        median: rc.median,
                        min: rc.min,
                        max: rc.max,
                    },
                    rb_count,
                },
            );
        }
    }
    results
}

// --- Main ---

fn main() {
    let args = Args::parse();
    let output_path = args
        .output
        .clone()
        .unwrap_or_else(|| project_root().join("bench/results.md"));

    match args.mode.as_str() {
        "setup" => {
            setup_repos();
        }
        "bench" => {
            let start = Instant::now();
            build_rblint();
            init_lockfiles();
            let bench = run_bench(&args);
            let elapsed = start.elapsed().as_secs_f64();
            let md = generate_report(&bench, &HashMap::new(), &args, Some(elapsed));
            fs::write(&output_path, &md).unwrap();
            eprintln!("\nWrote {}", output_path.display());
        }
        "conform" => {
            let start = Instant::now();
            build_rblint();
            init_lockfiles();
            let conform = run_conform();
            // Write structured JSON for coverage_table to consume
            let json_path = project_root().join("bench/conform.json");
            let json = serde_json::to_string_pretty(&conform).unwrap();
            fs::write(&json_path, &json).unwrap();
            eprintln!("\nWrote {}", json_path.display());
            // Also write human-readable markdown
            let bench = load_cached_bench();
            let elapsed = start.elapsed().as_secs_f64();
            let md = generate_report(&bench, &conform, &args, Some(elapsed));
            fs::write(&output_path, &md).unwrap();
            eprintln!("Wrote {}", output_path.display());
        }
        "report" => {
            let bench = load_cached_bench();
            // For conformance, we'd need to re-parse the JSON files.
            // For now, just regenerate from bench data.
            let md = generate_report(&bench, &HashMap::new(), &args, None);
            fs::write(&output_path, &md).unwrap();
            eprintln!("\nWrote {}", output_path.display());
        }
        "all" => {
            let start = Instant::now();
            setup_repos();
            build_rblint();
            init_lockfiles();
            let bench = run_bench(&args);
            let conform = run_conform();
            let json_path = project_root().join("bench/conform.json");
            let json = serde_json::to_string_pretty(&conform).unwrap();
            fs::write(&json_path, &json).unwrap();
            eprintln!("\nWrote {}", json_path.display());
            let elapsed = start.elapsed().as_secs_f64();
            let md = generate_report(&bench, &conform, &args, Some(elapsed));
            fs::write(&output_path, &md).unwrap();
            eprintln!("Wrote {}", output_path.display());
        }
        other => {
            eprintln!("Unknown mode: {other}. Use: setup, bench, conform, report, or all.");
            std::process::exit(1);
        }
    }
}
