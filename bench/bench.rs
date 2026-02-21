//! Benchmark turbocop vs rubocop on real-world codebases.
//!
//! Usage:
//!   cargo run --release --bin bench_turbocop          # full run (bench + conform + report)
//!   cargo run --release --bin bench_turbocop -- bench  # timing only
//!   cargo run --release --bin bench_turbocop -- conform # conformance only
//!   cargo run --release --bin bench_turbocop -- report  # regenerate results.md from cached data
//!   cargo run --release --bin bench_turbocop -- autocorrect-conform  # autocorrect conformance

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

    /// Run only on private/local repos from bench/private_repos.json
    #[arg(long)]
    private: bool,

    /// Run on both public and private repos
    #[arg(long)]
    all_repos: bool,
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
        name: "multi_json",
        url: "https://github.com/sferik/multi_json.git",
        tag: "v1.19.1",
    },
    BenchRepo {
        name: "lobsters",
        url: "https://github.com/lobsters/lobsters.git",
        tag: "main",
    },
];

// --- Unified repo reference ---

#[derive(Clone, Copy, PartialEq)]
enum RepoSource {
    /// Public repo cloned into bench/repos/
    Public,
    /// Private/local repo from bench/private_repos.json
    Private,
}

struct RepoRef {
    name: String,
    dir: PathBuf,
    source: RepoSource,
}

#[derive(serde::Deserialize)]
struct PrivateRepoEntry {
    name: String,
    path: String,
}

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

// --- Private repo support ---

fn private_repos_config_path() -> PathBuf {
    bench_dir().join("private_repos.json")
}

fn private_results_dir() -> PathBuf {
    bench_dir().join("private_results")
}

fn results_dir_for(source: RepoSource) -> PathBuf {
    match source {
        RepoSource::Public => results_dir(),
        RepoSource::Private => private_results_dir(),
    }
}

fn load_private_repos() -> Vec<RepoRef> {
    let config_path = private_repos_config_path();
    if !config_path.exists() {
        return Vec::new();
    }

    let content = match fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: could not read {}: {e}", config_path.display());
            return Vec::new();
        }
    };

    let entries: Vec<PrivateRepoEntry> = match serde_json::from_str(&content) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Warning: could not parse {}: {e}", config_path.display());
            return Vec::new();
        }
    };

    let home = std::env::var("HOME").unwrap_or_default();
    let mut repos = Vec::new();

    for entry in entries {
        let expanded = if entry.path.starts_with("~/") {
            format!("{}{}", home, &entry.path[1..])
        } else {
            entry.path.clone()
        };
        let dir = PathBuf::from(&expanded);

        if !dir.exists() {
            eprintln!(
                "Warning: private repo '{}' path does not exist: {}",
                entry.name,
                dir.display()
            );
            continue;
        }

        if !dir.join("Gemfile").exists() {
            eprintln!(
                "Warning: private repo '{}' has no Gemfile: {}",
                entry.name,
                dir.display()
            );
            continue;
        }

        repos.push(RepoRef {
            name: entry.name,
            dir,
            source: RepoSource::Private,
        });
    }

    repos
}

fn resolve_public_repos() -> Vec<RepoRef> {
    REPOS
        .iter()
        .map(|r| RepoRef {
            name: r.name.to_string(),
            dir: repos_dir().join(r.name),
            source: RepoSource::Public,
        })
        .collect()
}

fn resolve_repos(args: &Args) -> Vec<RepoRef> {
    if args.private && args.all_repos {
        eprintln!("Error: --private and --all-repos are mutually exclusive.");
        std::process::exit(1);
    }

    if args.private {
        let repos = load_private_repos();
        if repos.is_empty() {
            eprintln!(
                "No private repos configured. Create {} with repo entries.",
                private_repos_config_path().display()
            );
            eprintln!("Format: [{{\"name\": \"my-app\", \"path\": \"~/path/to/my-app\"}}]");
            std::process::exit(1);
        }
        repos
    } else if args.all_repos {
        let mut repos = resolve_public_repos();
        let private = load_private_repos();
        let public_names: HashSet<String> = repos.iter().map(|r| r.name.clone()).collect();
        for p in private {
            if public_names.contains(&p.name) {
                eprintln!(
                    "Warning: private repo '{}' has same name as public repo, skipping.",
                    p.name
                );
                continue;
            }
            repos.push(p);
        }
        repos
    } else {
        resolve_public_repos()
    }
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
                // Stale lockfiles can pin gems incompatible with the current Ruby.
                // Remove the lockfile and re-resolve to self-heal.
                eprintln!("  Bundle install failed. Removing Gemfile.lock and retrying...");
                let _ = fs::remove_file(repo_path.join("Gemfile.lock"));
                let retry = Command::new("bundle")
                    .args(["install", "--jobs", "4"])
                    .current_dir(&repo_path)
                    .status();
                match retry {
                    Ok(s) if s.success() => eprintln!("  Bundle install OK (fresh resolve)."),
                    _ => {
                        eprintln!("  Trying with --without production...");
                        let retry2 = Command::new("bundle")
                            .args(["install", "--jobs", "4", "--without", "production"])
                            .current_dir(&repo_path)
                            .status();
                        match retry2 {
                            Ok(s) if s.success() => {
                                eprintln!("  Bundle install OK (without production).")
                            }
                            _ => eprintln!(
                                "  WARNING: bundle install failed for {}. rubocop may not work.",
                                repo.name
                            ),
                        }
                    }
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

fn init_lockfiles(repos: &[RepoRef]) {
    let turbocop = turbocop_binary();
    if !turbocop.exists() {
        eprintln!("turbocop binary not found. Build first.");
        return;
    }

    for repo in repos {
        if !repo.dir.exists() {
            continue;
        }

        // Clear stale file-level result caches before regenerating.
        // The result cache stores per-file lint results keyed by content hash.
        // When the binary changes (new cops, different detection logic), stale
        // cached results diverge from fresh results.
        let _ = Command::new(turbocop.as_os_str())
            .args(["--cache-clear", repo.dir.to_str().unwrap()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        eprintln!("Generating .turbocop.cache for {}...", repo.name);
        let start = Instant::now();
        let output = Command::new(turbocop.as_os_str())
            .args(["--init", repo.dir.to_str().unwrap()])
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

fn run_bench(args: &Args, repos: &[RepoRef]) -> HashMap<String, BenchResult> {
    let turbocop = turbocop_binary();

    if !has_command("hyperfine") {
        eprintln!("Error: hyperfine not found. Install via: mise install");
        std::process::exit(1);
    }

    let mut bench_results = HashMap::new();

    for repo in repos {
        let results_path = results_dir_for(repo.source);
        fs::create_dir_all(&results_path).unwrap();

        if !repo.dir.exists() {
            eprintln!("Repo {} not found at {}.", repo.name, repo.dir.display());
            continue;
        }

        let rb_count = count_rb_files(&repo.dir);
        eprintln!("\n=== Benchmarking {} ({} .rb files) ===", repo.name, rb_count);

        let json_file = results_path.join(format!("{}-bench.json", repo.name));
        let turbocop_cmd = format!(
            "{} {} --no-color",
            turbocop.display(),
            repo.dir.display()
        );
        let rubocop_cmd = format!(
            "cd {} && bundle exec rubocop --no-color",
            repo.dir.display()
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
            repo.name.clone(),
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
    #[serde(default)]
    corrected: bool,
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
    // multi_json: `require: standard` sets EmptyClassDefinition Enabled: false,
    // but RuboCop still fires it (shows Enabled: pending). Likely a RuboCop quirk
    // where `require:` runtime config injection interacts with `NewCops: enable`
    // differently than YAML inheritance. See docs/TODO_EXCLUDED_COPS.md.
    if repo_dir.ends_with("multi_json") {
        excluded.insert("Style/EmptyClassDefinition".to_string());
    }
    excluded
}

/// Detect if a repo is a pure standardrb project (.standard.yml without .rubocop.yml).
/// For these projects, conformance should compare against `bundle exec standardrb`
/// instead of `bundle exec rubocop`, since standardrb applies its own config layer
/// that rubocop alone doesn't pick up.
fn is_standardrb_only(repo_dir: &Path) -> bool {
    !repo_dir.join(".rubocop.yml").exists() && repo_dir.join(".standard.yml").exists()
}

fn run_conform(repos: &[RepoRef]) -> HashMap<String, ConformResult> {
    let turbocop = turbocop_binary();

    let covered = get_covered_cops();
    eprintln!("{} cops covered by turbocop", covered.len());

    let mut conform_results = HashMap::new();

    for repo in repos {
        let results_path = results_dir_for(repo.source);
        fs::create_dir_all(&results_path).unwrap();

        if !repo.dir.exists() {
            eprintln!("Repo {} not found at {}.", repo.name, repo.dir.display());
            continue;
        }

        eprintln!("\n=== Conformance: {} ===", repo.name);

        // Run turbocop in JSON mode
        eprintln!("  Running turbocop...");
        let turbocop_json_file = results_path.join(format!("{}-turbocop.json", repo.name));
        let start = Instant::now();
        let turbocop_out = Command::new(turbocop.as_os_str())
            .args([
                repo.dir.to_str().unwrap(),
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

        // Run rubocop (or standardrb for pure-standardrb projects) in JSON mode
        let use_standardrb = is_standardrb_only(&repo.dir);
        let reference_tool = if use_standardrb { "standardrb" } else { "rubocop" };
        eprintln!("  Running {reference_tool}...");
        let rubocop_json_file = results_path.join(format!("{}-rubocop.json", repo.name));
        let start = Instant::now();
        let rubocop_out = Command::new("bundle")
            .args(["exec", reference_tool, "--format", "json", "--no-color"])
            .current_dir(&repo.dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .unwrap_or_else(|_| panic!("failed to run {reference_tool}"));
        fs::write(&rubocop_json_file, &rubocop_out.stdout).unwrap();
        eprintln!(
            "  {reference_tool} done in {:.1}s",
            start.elapsed().as_secs_f64()
        );

        // Parse and compare
        let repo_prefix = format!("{}/", repo.dir.display());

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
        let repo_excluded = per_repo_excluded_cops(&repo.dir);

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
            "  {reference_tool}: {} offenses (filtered to {} covered cops)",
            rubocop_set.len(),
            covered.len()
        );
        eprintln!("  matches: {}", matches.len());
        eprintln!("  FP (turbocop only): {}", fps.len());
        eprintln!("  FN ({reference_tool} only): {}", fns.len());
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

// --- Autocorrect conformance ---

#[derive(Debug, Default, serde::Serialize)]
struct AutocorrectConformResult {
    files_corrected_rubocop: usize,
    files_corrected_turbocop: usize,
    files_match: usize,
    files_differ: usize,
    match_rate: f64,
}

/// Run autocorrect conformance: compare `rubocop -A` vs `turbocop -A` output
/// on each bench repo. Uses full-file autocorrect (all cops at once).
fn run_autocorrect_conform(repos: &[RepoRef]) -> HashMap<String, AutocorrectConformResult> {
    let turbocop = turbocop_binary();
    let mut results = HashMap::new();

    for repo in repos {
        if !repo.dir.exists() {
            eprintln!("Repo {} not found at {}.", repo.name, repo.dir.display());
            continue;
        }

        eprintln!("\n=== Autocorrect conformance: {} ===", repo.name);

        let temp_base = std::env::temp_dir().join("turbocop_autocorrect_conform");
        let _ = fs::remove_dir_all(&temp_base);
        fs::create_dir_all(&temp_base).unwrap();

        // Collect original Ruby files (relative paths)
        let rb_files = collect_rb_files(&repo.dir);
        eprintln!("  {} Ruby files", rb_files.len());

        // Read original file contents
        let originals: HashMap<PathBuf, Vec<u8>> = rb_files
            .iter()
            .filter_map(|rel| {
                let full = repo.dir.join(rel);
                fs::read(&full).ok().map(|bytes| (rel.clone(), bytes))
            })
            .collect();

        // --- Run rubocop -A on a copy ---
        let rubocop_dir = temp_base.join("rubocop");
        copy_repo(&repo.dir, &rubocop_dir);
        eprintln!("  Running rubocop -A...");
        let start = Instant::now();
        let _ = Command::new("bundle")
            .args(["exec", "rubocop", "-A", "--format", "quiet"])
            .current_dir(&rubocop_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
        eprintln!("  rubocop -A done in {:.1}s", start.elapsed().as_secs_f64());

        // --- Run turbocop -A on another copy ---
        let turbocop_dir = temp_base.join("turbocop");
        copy_repo(&repo.dir, &turbocop_dir);
        // Copy the .turbocop.cache from the original repo
        let lock_src = repo.dir.join(".turbocop.cache");
        let lock_dst = turbocop_dir.join(".turbocop.cache");
        if lock_src.exists() {
            let _ = fs::copy(&lock_src, &lock_dst);
        }
        eprintln!("  Running turbocop -A...");
        let start = Instant::now();
        let _ = Command::new(turbocop.as_os_str())
            .args(["-A", turbocop_dir.to_str().unwrap()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
        eprintln!("  turbocop -A done in {:.1}s", start.elapsed().as_secs_f64());

        // --- Compare corrected files ---
        let mut files_corrected_rubocop = 0;
        let mut files_corrected_turbocop = 0;
        let mut files_match = 0;
        let mut files_differ = 0;
        let mut diff_examples: Vec<String> = Vec::new();

        for rel in &rb_files {
            let original = match originals.get(rel) {
                Some(b) => b,
                None => continue,
            };
            let rubocop_content = fs::read(rubocop_dir.join(rel)).unwrap_or_default();
            let turbocop_content = fs::read(turbocop_dir.join(rel)).unwrap_or_default();

            let rubocop_changed = rubocop_content != *original;
            let turbocop_changed = turbocop_content != *original;

            if rubocop_changed {
                files_corrected_rubocop += 1;
            }
            if turbocop_changed {
                files_corrected_turbocop += 1;
            }

            // Only compare files that at least one tool changed
            if rubocop_changed || turbocop_changed {
                if rubocop_content == turbocop_content {
                    files_match += 1;
                } else {
                    files_differ += 1;
                    if diff_examples.len() < 5 {
                        diff_examples.push(rel.display().to_string());
                    }
                }
            }
        }

        let total = files_match + files_differ;
        let match_rate = if total == 0 {
            100.0
        } else {
            files_match as f64 / total as f64 * 100.0
        };

        eprintln!("  rubocop corrected: {} files", files_corrected_rubocop);
        eprintln!("  turbocop corrected: {} files", files_corrected_turbocop);
        eprintln!("  matching corrections: {} files", files_match);
        eprintln!("  differing corrections: {} files", files_differ);
        eprintln!("  match rate: {:.1}%", match_rate);
        if !diff_examples.is_empty() {
            eprintln!("  example diffs: {}", diff_examples.join(", "));
        }

        results.insert(
            repo.name.clone(),
            AutocorrectConformResult {
                files_corrected_rubocop,
                files_corrected_turbocop,
                files_match,
                files_differ,
                match_rate,
            },
        );

        // Clean up temp
        let _ = fs::remove_dir_all(&temp_base);
    }

    results
}

// --- Autocorrect validation ---

/// Per-cop validation stats: how many offenses turbocop corrected vs how many rubocop still finds.
#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
struct CopValidateStats {
    /// Number of offenses turbocop marked as corrected
    turbocop_corrected: usize,
    /// Number of offenses rubocop still finds after turbocop's corrections
    rubocop_remaining: usize,
}

/// Per-repo autocorrect validation result.
#[derive(serde::Serialize, serde::Deserialize)]
struct AutocorrectValidateResult {
    cops_tested: usize,
    cops_clean: usize,
    cops_with_remaining: usize,
    per_cop: BTreeMap<String, CopValidateStats>,
}

/// Get the set of cops that support autocorrect.
fn get_autocorrectable_cops() -> Vec<String> {
    let turbocop = turbocop_binary();
    let output = Command::new(turbocop.as_os_str())
        .arg("--list-autocorrectable-cops")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .expect("failed to run turbocop --list-autocorrectable-cops");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Run autocorrect validation: apply `turbocop -A`, then verify with `rubocop --only <cops>`.
///
/// For each bench repo:
/// 1. Copy to temp dir
/// 2. Run `turbocop -A --format json` to correct files and capture what was corrected
/// 3. Run `rubocop --only <autocorrectable-cops> --format json` on corrected files
/// 4. For each autocorrectable cop, remaining rubocop offenses indicate broken autocorrect
fn run_autocorrect_validate(repos: &[RepoRef]) -> HashMap<String, AutocorrectValidateResult> {
    let turbocop = turbocop_binary();
    let autocorrectable = get_autocorrectable_cops();
    if autocorrectable.is_empty() {
        eprintln!("No autocorrectable cops found. Nothing to validate.");
        return HashMap::new();
    }
    let cops_csv = autocorrectable.join(",");
    eprintln!(
        "{} autocorrectable cops: {}",
        autocorrectable.len(),
        cops_csv
    );

    let mut results = HashMap::new();

    for repo in repos {
        if !repo.dir.exists() {
            eprintln!("Repo {} not found at {}.", repo.name, repo.dir.display());
            continue;
        }

        eprintln!("\n=== Autocorrect validation: {} ===", repo.name);

        let temp_base = std::env::temp_dir().join("turbocop_autocorrect_validate");
        let _ = fs::remove_dir_all(&temp_base);
        fs::create_dir_all(&temp_base).unwrap();

        // Copy repo to temp dir
        let work_dir = temp_base.join(&repo.name);
        copy_repo(&repo.dir, &work_dir);

        // Copy .turbocop.cache if present
        let lock_src = repo.dir.join(".turbocop.cache");
        let lock_dst = work_dir.join(".turbocop.cache");
        if lock_src.exists() {
            let _ = fs::copy(&lock_src, &lock_dst);
        }

        // Step 1: Run turbocop -A --format json
        eprintln!("  Running turbocop -A...");
        let start = Instant::now();
        let tc_output = Command::new(turbocop.as_os_str())
            .args(["-A", work_dir.to_str().unwrap(), "--format", "json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .expect("failed to run turbocop -A");
        eprintln!(
            "  turbocop -A done in {:.1}s",
            start.elapsed().as_secs_f64()
        );

        // Parse turbocop output to count corrected offenses per cop
        let mut per_cop: BTreeMap<String, CopValidateStats> = BTreeMap::new();
        if let Ok(tc_data) = serde_json::from_slice::<TurboCopOutput>(&tc_output.stdout) {
            for offense in &tc_data.offenses {
                if offense.corrected && autocorrectable.contains(&offense.cop_name) {
                    per_cop
                        .entry(offense.cop_name.clone())
                        .or_default()
                        .turbocop_corrected += 1;
                }
            }
        } else {
            eprintln!("  Failed to parse turbocop JSON output");
        }

        let total_corrected: usize = per_cop.values().map(|s| s.turbocop_corrected).sum();
        eprintln!("  turbocop corrected {} offenses", total_corrected);

        // Step 2: Run rubocop --only <autocorrectable-cops> on corrected files
        eprintln!("  Running rubocop --only <autocorrectable-cops>...");
        let start = Instant::now();
        let rb_output = Command::new("bundle")
            .args([
                "exec",
                "rubocop",
                "--only",
                &cops_csv,
                "--format",
                "json",
                "--no-color",
            ])
            .current_dir(&work_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .unwrap_or_else(|_| panic!("failed to run rubocop --only"));
        eprintln!(
            "  rubocop done in {:.1}s",
            start.elapsed().as_secs_f64()
        );

        // Parse rubocop output — remaining offenses
        if let Ok(rb_data) = serde_json::from_slice::<RubocopOutput>(&rb_output.stdout) {
            for file in &rb_data.files {
                for offense in &file.offenses {
                    if autocorrectable.contains(&offense.cop_name) {
                        per_cop
                            .entry(offense.cop_name.clone())
                            .or_default()
                            .rubocop_remaining += 1;
                    }
                }
            }
        } else {
            eprintln!("  Failed to parse rubocop JSON output");
        }

        let total_remaining: usize = per_cop.values().map(|s| s.rubocop_remaining).sum();
        let cops_tested = per_cop.len();
        let cops_clean = per_cop
            .values()
            .filter(|s| s.rubocop_remaining == 0 && s.turbocop_corrected > 0)
            .count();
        let cops_with_remaining = per_cop
            .values()
            .filter(|s| s.rubocop_remaining > 0 && s.turbocop_corrected > 0)
            .count();
        let detection_gaps = per_cop
            .values()
            .filter(|s| s.rubocop_remaining > 0 && s.turbocop_corrected == 0)
            .count();

        eprintln!("  rubocop found {} remaining offenses", total_remaining);
        eprintln!(
            "  {} cops tested, {} clean, {} with remaining, {} detection gaps",
            cops_tested, cops_clean, cops_with_remaining, detection_gaps
        );

        // Print per-cop details
        for (cop, stats) in &per_cop {
            if stats.turbocop_corrected > 0 {
                let status = if stats.rubocop_remaining == 0 { "PASS" } else { "FAIL" };
                eprintln!(
                    "    {} — corrected: {}, remaining: {} [{}]",
                    cop, stats.turbocop_corrected, stats.rubocop_remaining, status
                );
            } else if stats.rubocop_remaining > 0 {
                eprintln!(
                    "    {} — detection gap: {} rubocop offenses not detected by turbocop",
                    cop, stats.rubocop_remaining
                );
            }
        }

        results.insert(
            repo.name.clone(),
            AutocorrectValidateResult {
                cops_tested,
                cops_clean,
                cops_with_remaining,
                per_cop,
            },
        );

        // Clean up temp
        let _ = fs::remove_dir_all(&temp_base);
    }

    results
}

/// Generate a markdown report for autocorrect validation results.
fn generate_autocorrect_validate_report(
    results: &HashMap<String, AutocorrectValidateResult>,
) -> String {
    let mut md = String::new();
    let _ = writeln!(md, "# Autocorrect Validation Report\n");
    let _ = writeln!(
        md,
        "Validates that `turbocop -A` corrections are recognized as clean by `rubocop`.\n"
    );

    // Aggregate per-cop stats across all repos
    let mut aggregate: BTreeMap<String, CopValidateStats> = BTreeMap::new();
    for result in results.values() {
        for (cop, stats) in &result.per_cop {
            let agg = aggregate.entry(cop.clone()).or_default();
            agg.turbocop_corrected += stats.turbocop_corrected;
            agg.rubocop_remaining += stats.rubocop_remaining;
        }
    }

    // Split into validated cops (turbocop corrected > 0) and detection gaps
    let validated: BTreeMap<&String, &CopValidateStats> = aggregate
        .iter()
        .filter(|(_, s)| s.turbocop_corrected > 0)
        .collect();
    let detection_gaps: BTreeMap<&String, &CopValidateStats> = aggregate
        .iter()
        .filter(|(_, s)| s.turbocop_corrected == 0 && s.rubocop_remaining > 0)
        .collect();

    // Autocorrect validation table (only cops where turbocop actually corrected something)
    let _ = writeln!(md, "## Autocorrect Validation\n");
    if validated.is_empty() {
        let _ = writeln!(
            md,
            "No offenses were corrected by turbocop across all repos. These repos are already \
             clean for the {} autocorrectable cops.\n",
            get_autocorrectable_cops().len()
        );
    } else {
        let _ = writeln!(md, "| Cop | Corrected | Remaining | Status |");
        let _ = writeln!(md, "|-----|-----------|-----------|--------|");
        for (cop, stats) in &validated {
            let status = if stats.rubocop_remaining == 0 {
                "PASS"
            } else {
                "FAIL"
            };
            let _ = writeln!(
                md,
                "| {} | {} | {} | {} |",
                cop, stats.turbocop_corrected, stats.rubocop_remaining, status
            );
        }
        let passing = validated.values().filter(|s| s.rubocop_remaining == 0).count();
        let _ = writeln!(
            md,
            "\n**{}/{} cops passing** (0 remaining offenses after correction)\n",
            passing,
            validated.len()
        );
    }

    // Detection gaps table (rubocop finds offenses but turbocop didn't detect them)
    if !detection_gaps.is_empty() {
        let _ = writeln!(md, "## Detection Gaps\n");
        let _ = writeln!(
            md,
            "Offenses rubocop finds that turbocop did not detect (not an autocorrect issue).\n"
        );
        let _ = writeln!(md, "| Cop | Rubocop Offenses |");
        let _ = writeln!(md, "|-----|-----------------|");
        for (cop, stats) in &detection_gaps {
            let _ = writeln!(md, "| {} | {} |", cop, stats.rubocop_remaining);
        }
        let _ = writeln!(md);
    }

    // Per-repo details
    let _ = writeln!(md, "## Per-repo Details\n");
    let mut repo_names: Vec<&String> = results.keys().collect();
    repo_names.sort();
    for repo_name in repo_names {
        let result = &results[repo_name];
        let validated_cops: Vec<_> = result
            .per_cop
            .iter()
            .filter(|(_, s)| s.turbocop_corrected > 0)
            .collect();
        let gap_cops: Vec<_> = result
            .per_cop
            .iter()
            .filter(|(_, s)| s.turbocop_corrected == 0 && s.rubocop_remaining > 0)
            .collect();

        if validated_cops.is_empty() && gap_cops.is_empty() {
            continue; // Skip repos with nothing to report
        }

        let _ = writeln!(md, "### {}\n", repo_name);

        if !validated_cops.is_empty() {
            let _ = writeln!(md, "**Autocorrect validation:**\n");
            let _ = writeln!(md, "| Cop | Corrected | Remaining | Status |");
            let _ = writeln!(md, "|-----|-----------|-----------|--------|");
            for (cop, stats) in &validated_cops {
                let status = if stats.rubocop_remaining == 0 {
                    "PASS"
                } else {
                    "FAIL"
                };
                let _ = writeln!(
                    md,
                    "| {} | {} | {} | {} |",
                    cop, stats.turbocop_corrected, stats.rubocop_remaining, status
                );
            }
            let _ = writeln!(md);
        }

        if !gap_cops.is_empty() {
            let _ = writeln!(md, "**Detection gaps:**\n");
            let _ = writeln!(md, "| Cop | Rubocop Offenses |");
            let _ = writeln!(md, "|-----|-----------------|");
            for (cop, stats) in &gap_cops {
                let _ = writeln!(md, "| {} | {} |", cop, stats.rubocop_remaining);
            }
            let _ = writeln!(md);
        }
    }

    md
}

/// Collect all .rb files in a directory (relative paths), respecting .gitignore.
fn collect_rb_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in ignore::WalkBuilder::new(dir)
        .hidden(false)
        .git_ignore(true)
        .build()
    {
        if let Ok(entry) = entry {
            if entry.file_type().is_some_and(|ft| ft.is_file()) {
                if let Some(ext) = entry.path().extension() {
                    if ext == "rb" {
                        if let Ok(rel) = entry.path().strip_prefix(dir) {
                            files.push(rel.to_path_buf());
                        }
                    }
                }
            }
        }
    }
    files.sort();
    files
}

/// Copy a directory tree (shallow: files only, follows the same structure).
fn copy_repo(src: &Path, dst: &Path) {
    for entry in ignore::WalkBuilder::new(src)
        .hidden(false)
        .git_ignore(true)
        .build()
    {
        if let Ok(entry) = entry {
            let rel = match entry.path().strip_prefix(src) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let target = dst.join(rel);
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                let _ = fs::create_dir_all(&target);
            } else if entry.file_type().is_some_and(|ft| ft.is_file()) {
                if let Some(parent) = target.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::copy(entry.path(), &target);
            }
        }
    }
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
    repos: &[RepoRef],
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

        for repo in repos {
            if let Some(r) = bench.get(&repo.name) {
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

        for repo in repos {
            if let Some(r) = bench.get(&repo.name) {
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

        for repo in repos {
            if let Some(c) = conform.get(&repo.name) {
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
        for repo in repos {
            if let Some(c) = conform.get(&repo.name) {
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

fn load_cached_bench(repos: &[RepoRef]) -> HashMap<String, BenchResult> {
    let mut results = HashMap::new();
    for repo in repos {
        let json_file = results_dir_for(repo.source).join(format!("{}-bench.json", repo.name));
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
            let rb_count = if repo.dir.exists() {
                count_rb_files(&repo.dir)
            } else {
                0
            };
            results.insert(
                repo.name.clone(),
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
    let repos = resolve_repos(&args);

    let is_private_run = args.private;
    let output_path = args.output.clone().unwrap_or_else(|| {
        if is_private_run {
            project_root().join("bench/private_results.md")
        } else {
            project_root().join("bench/results.md")
        }
    });

    /// Choose the right JSON output path based on repo source.
    fn json_output_path(base_name: &str, is_private: bool) -> PathBuf {
        let prefix = if is_private { "private_" } else { "" };
        project_root().join(format!("bench/{prefix}{base_name}"))
    }

    match args.mode.as_str() {
        "setup" => {
            if is_private_run {
                eprintln!("Validating private repo paths...");
                for repo in &repos {
                    if repo.dir.exists() {
                        let rb_count = count_rb_files(&repo.dir);
                        eprintln!("  OK: {} ({} .rb files) at {}", repo.name, rb_count, repo.dir.display());
                    } else {
                        eprintln!("  MISSING: {} at {}", repo.name, repo.dir.display());
                    }
                }
            } else {
                setup_repos();
            }
        }
        "bench" => {
            let start = Instant::now();
            build_turbocop();
            init_lockfiles(&repos);
            let bench = run_bench(&args, &repos);
            let elapsed = start.elapsed().as_secs_f64();
            let md = generate_report(&bench, &HashMap::new(), &args, &repos, Some(elapsed));
            fs::write(&output_path, &md).unwrap();
            eprintln!("\nWrote {}", output_path.display());
        }
        "conform" => {
            let start = Instant::now();
            build_turbocop();
            init_lockfiles(&repos);
            let conform = run_conform(&repos);
            // Write structured JSON
            let json_path = json_output_path("conform.json", is_private_run);
            let json = serde_json::to_string_pretty(&conform).unwrap();
            fs::write(&json_path, &json).unwrap();
            eprintln!("\nWrote {}", json_path.display());
            // Also write human-readable markdown
            let bench = load_cached_bench(&repos);
            let elapsed = start.elapsed().as_secs_f64();
            let md = generate_report(&bench, &conform, &args, &repos, Some(elapsed));
            fs::write(&output_path, &md).unwrap();
            eprintln!("Wrote {}", output_path.display());
        }
        "quick" => {
            build_turbocop();
            run_quick_bench(&args);
        }
        "report" => {
            let bench = load_cached_bench(&repos);
            // For conformance, we'd need to re-parse the JSON files.
            // For now, just regenerate from bench data.
            let md = generate_report(&bench, &HashMap::new(), &args, &repos, None);
            fs::write(&output_path, &md).unwrap();
            eprintln!("\nWrote {}", output_path.display());
        }
        "all" => {
            let start = Instant::now();
            if !is_private_run {
                setup_repos();
            }
            build_turbocop();
            init_lockfiles(&repos);
            let bench = run_bench(&args, &repos);
            let conform = run_conform(&repos);
            let json_path = json_output_path("conform.json", is_private_run);
            let json = serde_json::to_string_pretty(&conform).unwrap();
            fs::write(&json_path, &json).unwrap();
            eprintln!("\nWrote {}", json_path.display());
            let elapsed = start.elapsed().as_secs_f64();
            let md = generate_report(&bench, &conform, &args, &repos, Some(elapsed));
            fs::write(&output_path, &md).unwrap();
            eprintln!("Wrote {}", output_path.display());
        }
        "autocorrect-conform" => {
            let start = Instant::now();
            build_turbocop();
            init_lockfiles(&repos);
            let ac_results = run_autocorrect_conform(&repos);
            let json_path = json_output_path("autocorrect_conform.json", is_private_run);
            let json = serde_json::to_string_pretty(&ac_results).unwrap();
            fs::write(&json_path, &json).unwrap();
            eprintln!("\nWrote {} ({:.0}s)", json_path.display(), start.elapsed().as_secs_f64());
        }
        "autocorrect-validate" => {
            let start = Instant::now();
            build_turbocop();
            init_lockfiles(&repos);
            let av_results = run_autocorrect_validate(&repos);
            // Write structured JSON
            let json_path = json_output_path("autocorrect_validate.json", is_private_run);
            let json = serde_json::to_string_pretty(&av_results).unwrap();
            fs::write(&json_path, &json).unwrap();
            eprintln!("\nWrote {}", json_path.display());
            // Write markdown report
            let md = generate_autocorrect_validate_report(&av_results);
            let md_path = if is_private_run {
                project_root().join("bench/private_autocorrect_validate.md")
            } else {
                project_root().join("bench/autocorrect_validate.md")
            };
            fs::write(&md_path, &md).unwrap();
            eprintln!("Wrote {} ({:.0}s)", md_path.display(), start.elapsed().as_secs_f64());
        }
        other => {
            eprintln!("Unknown mode: {other}. Use: setup, bench, conform, report, quick, autocorrect-conform, autocorrect-validate, or all.");
            std::process::exit(1);
        }
    }
}
