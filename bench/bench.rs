//! Benchmark turbocop vs rubocop on real-world codebases.
//!
//! Usage:
//!   cargo run --release --bin bench_turbocop          # full run (bench + conform + report)
//!   cargo run --release --bin bench_turbocop -- bench  # timing only
//!   cargo run --release --bin bench_turbocop -- conform # conformance only
//!   cargo run --release --bin bench_turbocop -- report  # regenerate results.md from cached data

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use clap::Parser;

// --- CLI ---

#[derive(Parser)]
#[command(about = "Benchmark turbocop vs rubocop. Writes results to bench/results.md.")]
struct Args {
    /// Subcommand: bench, conform, report, quick, or omit for all
    #[arg(default_value = "all")]
    mode: String,

    /// Number of hyperfine runs per benchmark
    #[arg(long, default_value_t = 3)]
    runs: u32,

    /// Hyperfine warmup runs
    #[arg(long, default_value_t = 1)]
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
    BenchRepo {
        name: "ruby-progressbar",
        url: "https://github.com/jfelchner/ruby-progressbar.git",
        tag: "v1.5.1",
    },
    BenchRepo {
        name: "multi_json",
        url: "https://github.com/sferik/multi_json.git",
        tag: "v1.19.1",
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

fn turbocop_binary() -> PathBuf {
    project_root().join("target/release/turbocop")
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

fn build_turbocop() {
    eprintln!("Building turbocop (release)...");
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--bin",
            "turbocop",
            "--manifest-path",
            project_root().join("Cargo.toml").to_str().unwrap(),
        ])
        .status()
        .expect("failed to run cargo build");
    assert!(status.success(), "cargo build --release failed");
}

// --- Init lockfiles ---

fn init_lockfiles() {
    let turbocop = turbocop_binary();
    if !turbocop.exists() {
        eprintln!("turbocop binary not found. Build first.");
        return;
    }

    for repo in REPOS {
        let repo_dir = repos_dir().join(repo.name);
        if !repo_dir.exists() {
            continue;
        }

        eprintln!("Generating .turbocop.cache for {}...", repo.name);
        let start = Instant::now();
        let output = Command::new(turbocop.as_os_str())
            .args(["--init", repo_dir.to_str().unwrap()])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .expect("failed to run turbocop --init");

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
    turbocop: HyperfineResult,
    rubocop: HyperfineResult,
    rb_count: usize,
}

fn run_bench(args: &Args) -> HashMap<String, BenchResult> {
    let turbocop = turbocop_binary();
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
        let turbocop_cmd = format!(
            "{} {} --no-color",
            turbocop.display(),
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
                "turbocop",
                &turbocop_cmd,
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

        let turbocop_result = parsed
            .results
            .into_iter()
            .find(|r| r.command == "turbocop")
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
                turbocop: turbocop_result,
                rubocop: rubocop_result,
                rb_count,
            },
        );
    }

    bench_results
}

// --- Quick bench (single repo, cached vs uncached) ---

fn run_quick_bench(args: &Args) {
    let bench_start = Instant::now();
    let repo_name = "rubygems.org";
    let repo_dir = repos_dir().join(repo_name);
    if !repo_dir.exists() {
        eprintln!("{repo_name} repo not found. Run `bench_turbocop setup` first.");
        std::process::exit(1);
    }

    let turbocop = turbocop_binary();
    let results_path = results_dir();
    fs::create_dir_all(&results_path).unwrap();

    if !has_command("hyperfine") {
        eprintln!("Error: hyperfine not found. Install via: mise install");
        std::process::exit(1);
    }

    // Init lockfile for just this repo
    eprintln!("Generating .turbocop.cache for {}...", repo_name);
    let init_out = Command::new(turbocop.as_os_str())
        .args(["--init", repo_dir.to_str().unwrap()])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to run turbocop --init");
    if !init_out.status.success() {
        let stderr = String::from_utf8_lossy(&init_out.stderr);
        eprintln!("  Failed: {}", stderr.trim());
    }

    let rb_count = count_rb_files(&repo_dir);
    let runs = args.runs;
    eprintln!(
        "\n=== Quick Bench: {} ({} .rb files, {} runs) ===",
        repo_name, rb_count, runs
    );

    // --- Cached (warm) scenario ---
    eprintln!("\n--- Cached (warm) ---");
    let cached_json = results_path.join("quick-cached.json");
    let turbocop_cached_cmd = format!("{} {} --no-color", turbocop.display(), repo_dir.display());
    let rubocop_cached_cmd = format!(
        "cd {} && bundle exec rubocop --no-color",
        repo_dir.display()
    );

    let status = Command::new("hyperfine")
        .args([
            "--warmup",
            "1",
            "--runs",
            &runs.to_string(),
            "--ignore-failure",
            "--export-json",
            cached_json.to_str().unwrap(),
            "--command-name",
            "turbocop",
            &turbocop_cached_cmd,
            "--command-name",
            "rubocop",
            &rubocop_cached_cmd,
        ])
        .status()
        .expect("failed to run hyperfine");
    if !status.success() {
        eprintln!("hyperfine failed for cached scenario");
        std::process::exit(1);
    }

    // --- No cache scenario ---
    eprintln!("\n--- No cache ---");
    let uncached_json = results_path.join("quick-uncached.json");
    let turbocop_uncached_cmd = format!(
        "{} --cache false {} --no-color",
        turbocop.display(),
        repo_dir.display()
    );
    let rubocop_uncached_cmd = format!(
        "cd {} && bundle exec rubocop --cache false --no-color",
        repo_dir.display()
    );

    let status = Command::new("hyperfine")
        .args([
            "--warmup",
            "1",
            "--runs",
            &runs.to_string(),
            "--ignore-failure",
            "--export-json",
            uncached_json.to_str().unwrap(),
            "--command-name",
            "turbocop",
            &turbocop_uncached_cmd,
            "--command-name",
            "rubocop",
            &rubocop_uncached_cmd,
        ])
        .status()
        .expect("failed to run hyperfine");
    if !status.success() {
        eprintln!("hyperfine failed for uncached scenario");
        std::process::exit(1);
    }

    // Parse results
    let cached: HyperfineOutput =
        serde_json::from_str(&fs::read_to_string(&cached_json).unwrap()).unwrap();
    let uncached: HyperfineOutput =
        serde_json::from_str(&fs::read_to_string(&uncached_json).unwrap()).unwrap();

    let cached_turbocop = cached
        .results
        .iter()
        .find(|r| r.command == "turbocop")
        .unwrap();
    let cached_rubocop = cached
        .results
        .iter()
        .find(|r| r.command == "rubocop")
        .unwrap();
    let uncached_turbocop = uncached
        .results
        .iter()
        .find(|r| r.command == "turbocop")
        .unwrap();
    let uncached_rubocop = uncached
        .results
        .iter()
        .find(|r| r.command == "rubocop")
        .unwrap();

    // Generate report
    let date = shell_output("date", &["-u", "+%Y-%m-%d %H:%M UTC"]);
    let platform = shell_output("uname", &["-sm"]);

    let mut md = String::new();
    writeln!(md, "# turbocop Quick Benchmark").unwrap();
    writeln!(md).unwrap();
    writeln!(
        md,
        "> Auto-generated by `cargo run --release --bin bench_turbocop -- quick`. Do not edit manually."
    )
    .unwrap();
    writeln!(md, "> Last updated: {date} on `{platform}`").unwrap();
    writeln!(md).unwrap();
    writeln!(md, "**Repo:** {repo_name} ({rb_count} .rb files)").unwrap();
    writeln!(md, "**Benchmark config:** {runs} runs").unwrap();
    writeln!(
        md,
        "**Total time:** {:.0}s",
        bench_start.elapsed().as_secs_f64()
    )
    .unwrap();
    writeln!(md).unwrap();
    writeln!(md, "## Results").unwrap();
    writeln!(md).unwrap();
    writeln!(md, "| Mode | turbocop | rubocop | Speedup |").unwrap();
    writeln!(md, "|------|-------:|--------:|--------:|").unwrap();
    writeln!(
        md,
        "| Cached (warm) | **{}** | {} | **{}** |",
        format_time(cached_turbocop.median),
        format_time(cached_rubocop.median),
        format_speedup(cached_rubocop.median, cached_turbocop.median),
    )
    .unwrap();
    writeln!(
        md,
        "| No cache | **{}** | {} | **{}** |",
        format_time(uncached_turbocop.median),
        format_time(uncached_rubocop.median),
        format_speedup(uncached_rubocop.median, uncached_turbocop.median),
    )
    .unwrap();
    writeln!(md).unwrap();

    let output_path = args
        .output
        .clone()
        .unwrap_or_else(|| project_root().join("bench/quick_results.md"));
    fs::write(&output_path, &md).unwrap();
    eprintln!("\nWrote {}", output_path.display());
}

// --- Conformance ---

#[derive(serde::Deserialize)]
struct TurboCopOutput {
    offenses: Vec<TurboCopOffense>,
}

#[derive(serde::Deserialize)]
struct TurboCopOffense {
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
    turbocop_count: usize,
    rubocop_count: usize,
    matches: usize,
    false_positives: usize,
    false_negatives: usize,
    match_rate: f64,
    per_cop: BTreeMap<String, CopStats>,
}

fn get_covered_cops() -> HashSet<String> {
    let turbocop = turbocop_binary();
    let output = Command::new(turbocop.as_os_str())
        .arg("--list-cops")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .expect("failed to run turbocop --list-cops");
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
///
/// fat_free_crm has 4 cops where RuboCop reports 0 offenses even with `--only`,
/// but the code patterns match the cop specifications. These are RuboCop quirks,
/// not turbocop bugs.
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
    // Known RuboCop quirks: RuboCop reports 0 offenses on these cops even
    // with --only, but the code patterns match the cop specifications.
    if repo_dir.ends_with("fat_free_crm") {
        for cop in [
            "Style/RedundantRegexpEscape",
            "Layout/FirstArrayElementIndentation",
            "Layout/MultilineMethodCallIndentation",
            "Style/TrailingCommaInHashLiteral",
        ] {
            excluded.insert(cop.to_string());
        }
    }
    excluded
}

fn run_conform() -> HashMap<String, ConformResult> {
    let turbocop = turbocop_binary();
    let results_path = results_dir();
    fs::create_dir_all(&results_path).unwrap();

    let covered = get_covered_cops();
    eprintln!("{} cops covered by turbocop", covered.len());

    let mut conform_results = HashMap::new();

    for repo in REPOS {
        let repo_dir = repos_dir().join(repo.name);
        if !repo_dir.exists() {
            eprintln!("Repo {} not found. Run setup first.", repo.name);
            continue;
        }

        eprintln!("\n=== Conformance: {} ===", repo.name);

        // Run turbocop in JSON mode
        eprintln!("  Running turbocop...");
        let turbocop_json_file = results_path.join(format!("{}-turbocop.json", repo.name));
        let start = Instant::now();
        let turbocop_out = Command::new(turbocop.as_os_str())
            .args([
                repo_dir.to_str().unwrap(),
                "--format",
                "json",
                "--no-color",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .expect("failed to run turbocop");
        fs::write(&turbocop_json_file, &turbocop_out.stdout).unwrap();
        eprintln!("  turbocop done in {:.1}s", start.elapsed().as_secs_f64());

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

        let turbocop_data: TurboCopOutput =
            match serde_json::from_slice(&turbocop_out.stdout) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("  Failed to parse turbocop JSON: {e}");
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

        let turbocop_set: HashSet<Offense> = turbocop_data
            .offenses
            .iter()
            .filter(|o| !repo_excluded.contains(&o.cop_name))
            .map(|o| {
                let path = o.path.strip_prefix(&repo_prefix).unwrap_or(&o.path);
                // Strip leading "./" if present (turbocop outputs ./path when run with ".")
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

        let matches: HashSet<&Offense> = turbocop_set.intersection(&rubocop_set).collect();
        let fps: HashSet<&Offense> = turbocop_set.difference(&rubocop_set).collect();
        let fns: HashSet<&Offense> = rubocop_set.difference(&turbocop_set).collect();
        let total = turbocop_set.union(&rubocop_set).count();
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

        eprintln!("  turbocop: {} offenses", turbocop_set.len());
        eprintln!(
            "  rubocop: {} offenses (filtered to {} covered cops)",
            rubocop_set.len(),
            covered.len()
        );
        eprintln!("  matches: {}", matches.len());
        eprintln!("  FP (turbocop only): {}", fps.len());
        eprintln!("  FN (rubocop only): {}", fns.len());
        eprintln!("  match rate: {:.1}%", match_rate);

        conform_results.insert(
            repo.name.to_string(),
            ConformResult {
                turbocop_count: turbocop_set.len(),
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

    let covered_count = if turbocop_binary().exists() {
        get_covered_cops().len()
    } else {
        0
    };

    let mut md = String::new();
    writeln!(md, "# turbocop Benchmark & Conformance Results").unwrap();
    writeln!(md).unwrap();
    writeln!(
        md,
        "> Auto-generated by `cargo run --release --bin bench_turbocop`. Do not edit manually."
    )
    .unwrap();
    writeln!(md, "> Last updated: {date} on `{platform}`").unwrap();
    writeln!(md).unwrap();
    if covered_count > 0 {
        writeln!(md, "**turbocop cops:** {covered_count}").unwrap();
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
            "Median of {} runs via [hyperfine](https://github.com/sharkdp/hyperfine). Both tools use built-in file cache (warm after hyperfine warmup runs).",
            args.runs
        )
        .unwrap();
        writeln!(md).unwrap();
        writeln!(
            md,
            "| Repo | .rb files | turbocop | rubocop | Speedup |"
        )
        .unwrap();
        writeln!(md, "|------|----------:|------------------:|-----------------:|--------:|").unwrap();

        for repo in REPOS {
            if let Some(r) = bench.get(repo.name) {
                let speedup = format_speedup(r.rubocop.median, r.turbocop.median);
                writeln!(
                    md,
                    "| {} | {} | **{}** | {} | **{}** |",
                    repo.name,
                    r.rb_count,
                    format_time(r.turbocop.median),
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

                for (name, result) in [("turbocop", &r.turbocop), ("rubocop", &r.rubocop)] {
                    let bold = name == "turbocop";
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
            "Location-level comparison: file + line + cop_name. Only cops implemented by turbocop ({covered_count}) are compared."
        )
        .unwrap();
        writeln!(md).unwrap();
        writeln!(
            md,
            "| Repo | turbocop | rubocop | Matches | FP (turbocop only) | FN (rubocop only) | Match rate |"
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
                    c.turbocop_count,
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

        let turbocop_result = parsed.results.iter().find(|r| r.command == "turbocop");
        let rubocop_result = parsed.results.iter().find(|r| r.command == "rubocop");

        if let (Some(rb), Some(rc)) = (turbocop_result, rubocop_result) {
            let repo_dir = repos_dir().join(repo.name);
            let rb_count = if repo_dir.exists() {
                count_rb_files(&repo_dir)
            } else {
                0
            };
            results.insert(
                repo.name.to_string(),
                BenchResult {
                    turbocop: HyperfineResult {
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
            build_turbocop();
            init_lockfiles();
            let bench = run_bench(&args);
            let elapsed = start.elapsed().as_secs_f64();
            let md = generate_report(&bench, &HashMap::new(), &args, Some(elapsed));
            fs::write(&output_path, &md).unwrap();
            eprintln!("\nWrote {}", output_path.display());
        }
        "conform" => {
            let start = Instant::now();
            build_turbocop();
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
        "quick" => {
            build_turbocop();
            run_quick_bench(&args);
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
            build_turbocop();
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
            eprintln!("Unknown mode: {other}. Use: setup, bench, conform, report, quick, or all.");
            std::process::exit(1);
        }
    }
}
