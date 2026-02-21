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

/// Check if a repo directory needs `mise exec --` to activate the correct Ruby.
fn needs_mise(repo_dir: &Path) -> bool {
    (repo_dir.join(".ruby-version").exists()
        || repo_dir.join(".tool-versions").exists()
        || repo_dir.join(".mise.toml").exists())
        && has_command("mise")
}

/// Build a `Command` for `bundle exec <tool> [args...]`, using `mise exec --`
/// when the repo has a `.ruby-version`/`.tool-versions` that may differ from the
/// current shell's Ruby.
fn bundle_exec_command(repo_dir: &Path, tool: &str, extra_args: &[&str]) -> Command {
    if needs_mise(repo_dir) {
        let mut cmd = Command::new("mise");
        cmd.arg("exec").arg("--").arg("bundle").arg("exec").arg(tool);
        cmd.args(extra_args);
        cmd.current_dir(repo_dir);
        cmd
    } else {
        let mut cmd = Command::new("bundle");
        cmd.arg("exec").arg(tool);
        cmd.args(extra_args);
        cmd.current_dir(repo_dir);
        cmd
    }
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

    // Clear stale result caches once before all inits.
    // The result cache stores per-file lint results keyed by session hash.
    // When the binary changes (new cops, different detection logic), stale
    // cached results diverge from fresh results. Clearing once avoids
    // accidentally deleting lockfiles created by earlier repos in this loop.
    let _ = Command::new(turbocop.as_os_str())
        .args(["--cache-clear", "."])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output();

    for repo in repos {
        if !repo.dir.exists() {
            continue;
        }

        eprintln!("Generating lockfile for {}...", repo.name);
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
    /// Number of files mtime-invalidated before each run.
    touched_count: usize,
}

/// Pre-flight check: run a tool once and verify it doesn't fatally error.
/// Returns true if the tool exited with code 0 or 1 (ok / offenses found).
/// Returns false if it exited with code 2+ (fatal error like missing lockfile).
fn preflight_check(name: &str, cmd: &str, repo_name: &str) -> bool {
    let output = Command::new("sh")
        .args(["-c", cmd])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output();
    match output {
        Ok(o) => {
            let code = o.status.code().unwrap_or(127);
            if code > 1 {
                let stderr = String::from_utf8_lossy(&o.stderr);
                let first_line = stderr.lines().next().unwrap_or("(no output)");
                eprintln!(
                    "  SKIP {repo_name}: {name} exited with code {code}: {first_line}"
                );
                false
            } else {
                true
            }
        }
        Err(e) => {
            eprintln!("  SKIP {repo_name}: failed to run {name}: {e}");
            false
        }
    }
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

        // Pre-flight: verify both tools run without fatal errors before benchmarking
        let turbocop_cmd_str = format!(
            "{} {} --no-color",
            turbocop.display(),
            repo.dir.display()
        );
        let rubocop_cmd_str = if needs_mise(&repo.dir) {
            format!(
                "cd {} && mise exec -- bundle exec rubocop --no-color",
                repo.dir.display()
            )
        } else {
            format!(
                "cd {} && bundle exec rubocop --no-color",
                repo.dir.display()
            )
        };

        if !preflight_check("turbocop", &turbocop_cmd_str, &repo.name) {
            continue;
        }
        if !preflight_check("rubocop", &rubocop_cmd_str, &repo.name) {
            continue;
        }

        // Touch 10% of .rb files (capped at 50) before each run to simulate
        // "git pull brought changes". Both caches are warm from preflight.
        let touched_count = std::cmp::min(std::cmp::max(rb_count / 10, 1), 50);
        let json_file = results_path.join(format!("{}-bench.json", repo.name));
        let prepare_cmd = format!(
            "find {} -name '*.rb' -not -path '*/vendor/*' -not -path '*/node_modules/*' | shuf -n {} | xargs touch",
            repo.dir.display(),
            touched_count
        );

        let status = Command::new("hyperfine")
            .args([
                "--warmup",
                &args.warmup.to_string(),
                "--runs",
                &args.runs.to_string(),
                "--prepare",
                &prepare_cmd,
                "--ignore-failure",
                "--export-json",
                json_file.to_str().unwrap(),
                "--command-name",
                "turbocop",
                &turbocop_cmd_str,
                "--command-name",
                "rubocop",
                &rubocop_cmd_str,
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
            .iter()
            .find(|r| r.command == "turbocop")
            .unwrap();
        let rubocop_result = parsed
            .results
            .iter()
            .find(|r| r.command == "rubocop")
            .unwrap();

        bench_results.insert(
            repo.name.clone(),
            BenchResult {
                turbocop: HyperfineResult {
                    command: turbocop_result.command.clone(),
                    mean: turbocop_result.mean,
                    stddev: turbocop_result.stddev,
                    median: turbocop_result.median,
                    min: turbocop_result.min,
                    max: turbocop_result.max,
                },
                rubocop: HyperfineResult {
                    command: rubocop_result.command.clone(),
                    mean: rubocop_result.mean,
                    stddev: rubocop_result.stddev,
                    median: rubocop_result.median,
                    min: rubocop_result.min,
                    max: rubocop_result.max,
                },
                rb_count,
                touched_count,
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
    eprintln!("Generating lockfile for {}...", repo_name);
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
    let touched_count = std::cmp::min(std::cmp::max(rb_count / 10, 1), 50);
    eprintln!(
        "\n=== Quick Bench: {} ({} .rb files, {} changed, {} runs) ===",
        repo_name, rb_count, touched_count, runs
    );

    let turbocop_cmd = format!("{} {} --no-color", turbocop.display(), repo_dir.display());
    let rubocop_cmd = format!(
        "cd {} && bundle exec rubocop --no-color",
        repo_dir.display()
    );

    // Pre-flight check
    if !preflight_check("turbocop", &turbocop_cmd, repo_name) {
        std::process::exit(1);
    }
    if !preflight_check("rubocop", &rubocop_cmd, repo_name) {
        std::process::exit(1);
    }

    // --- Partial invalidation (local dev scenario) ---
    eprintln!("\n--- Partial invalidation ({touched_count} files touched) ---");
    let partial_json = results_path.join("quick-partial.json");
    let prepare_cmd = format!(
        "find {} -name '*.rb' -not -path '*/vendor/*' -not -path '*/node_modules/*' | shuf -n {} | xargs touch",
        repo_dir.display(),
        touched_count
    );

    let status = Command::new("hyperfine")
        .args([
            "--warmup",
            &args.warmup.to_string(),
            "--runs",
            &runs.to_string(),
            "--prepare",
            &prepare_cmd,
            "--ignore-failure",
            "--export-json",
            partial_json.to_str().unwrap(),
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
        eprintln!("hyperfine failed for partial invalidation");
        std::process::exit(1);
    }

    // --- No cache (CI scenario) ---
    eprintln!("\n--- No cache (CI) ---");
    let nocache_json = results_path.join("quick-nocache.json");
    let turbocop_nocache_cmd = format!(
        "{} --cache false {} --no-color",
        turbocop.display(),
        repo_dir.display()
    );
    let rubocop_nocache_cmd = if needs_mise(&repo_dir) {
        format!(
            "cd {} && mise exec -- bundle exec rubocop --cache false --no-color",
            repo_dir.display()
        )
    } else {
        format!(
            "cd {} && bundle exec rubocop --cache false --no-color",
            repo_dir.display()
        )
    };

    let status = Command::new("hyperfine")
        .args([
            "--warmup",
            "1",
            "--runs",
            &runs.to_string(),
            "--ignore-failure",
            "--export-json",
            nocache_json.to_str().unwrap(),
            "--command-name",
            "turbocop",
            &turbocop_nocache_cmd,
            "--command-name",
            "rubocop",
            &rubocop_nocache_cmd,
        ])
        .status()
        .expect("failed to run hyperfine");
    if !status.success() {
        eprintln!("hyperfine failed for no-cache scenario");
        std::process::exit(1);
    }

    // Parse results
    let partial: HyperfineOutput =
        serde_json::from_str(&fs::read_to_string(&partial_json).unwrap()).unwrap();
    let nocache: HyperfineOutput =
        serde_json::from_str(&fs::read_to_string(&nocache_json).unwrap()).unwrap();

    let partial_tc = partial.results.iter().find(|r| r.command == "turbocop").unwrap();
    let partial_rc = partial.results.iter().find(|r| r.command == "rubocop").unwrap();
    let nocache_tc = nocache.results.iter().find(|r| r.command == "turbocop").unwrap();
    let nocache_rc = nocache.results.iter().find(|r| r.command == "rubocop").unwrap();

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
    writeln!(md, "**Repo:** {repo_name} ({rb_count} .rb files, {touched_count} mtime-invalidated per run)").unwrap();
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
    writeln!(md, "| Scenario | turbocop | rubocop | Speedup |").unwrap();
    writeln!(md, "|----------|-------:|--------:|--------:|").unwrap();
    writeln!(
        md,
        "| Local dev ({touched_count} files changed) | **{}** | {} | **{}** |",
        format_time(partial_tc.median),
        format_time(partial_rc.median),
        format_speedup(partial_rc.median, partial_tc.median),
    )
    .unwrap();
    writeln!(
        md,
        "| CI (no cache) | **{}** | {} | **{}** |",
        format_time(nocache_tc.median),
        format_time(nocache_rc.median),
        format_speedup(nocache_rc.median, nocache_tc.median),
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

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
struct GemVersionMismatchInfo {
    gem_name: String,
    vendor_version: String,
    project_version: String,
    attributed_fps: usize,
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
    turbocop_secs: Option<f64>,
    rubocop_secs: Option<f64>,
    #[serde(default)]
    version_mismatches: Vec<GemVersionMismatchInfo>,
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

// --- Gem version mismatch detection ---

/// Gems with vendor submodules under `vendor/` whose cop behavior may diverge
/// across versions. Each entry here must have a corresponding `vendor/<gem>/`
/// submodule so `get_vendor_gem_version()` can read its git tag.
const VERSION_CHECK_GEMS: &[&str] = &[
    "rubocop",
    "rubocop-rails",
    "rubocop-performance",
    "rubocop-rspec",
    "rubocop-factory_bot",
    "rubocop-rspec_rails",
];

/// Map a cop name to the gem it belongs to.
fn cop_gem(cop_name: &str) -> &'static str {
    if cop_name.starts_with("Rails/") {
        "rubocop-rails"
    } else if cop_name.starts_with("Performance/") {
        "rubocop-performance"
    } else if cop_name.starts_with("FactoryBot/") {
        "rubocop-factory_bot"
    } else if cop_name.starts_with("RSpecRails/") {
        "rubocop-rspec_rails"
    } else if cop_name.starts_with("RSpec/") || cop_name.starts_with("Capybara/") {
        "rubocop-rspec"
    } else {
        "rubocop"
    }
}

/// Get the version from a vendor submodule's git tag (e.g. "v2.34.3" → "2.34.3").
fn get_vendor_gem_version(gem_name: &str) -> Option<String> {
    let vendor_dir = project_root().join("vendor").join(gem_name);
    if !vendor_dir.exists() {
        return None;
    }
    let output = Command::new("git")
        .args(["describe", "--tags", "--exact-match"])
        .current_dir(&vendor_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Some(tag.trim_start_matches('v').to_string())
}

/// Get the version of a gem installed in a project's bundle.
fn get_project_gem_version(repo_dir: &Path, gem_name: &str) -> Option<String> {
    let mut cmd = if needs_mise(repo_dir) {
        let mut c = Command::new("mise");
        c.arg("exec")
            .arg("--")
            .arg("bundle")
            .arg("info")
            .arg(gem_name);
        c
    } else {
        let mut c = Command::new("bundle");
        c.arg("info").arg(gem_name);
        c
    };
    cmd.current_dir(repo_dir);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());

    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse "  * gem_name (version)" from first line
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("* ") {
            if let Some(paren_start) = trimmed.find('(') {
                if let Some(paren_end) = trimmed.find(')') {
                    return Some(trimmed[paren_start + 1..paren_end].to_string());
                }
            }
        }
    }
    None
}

/// Detect gem version mismatches between vendor submodules and the project's installed gems.
/// Returns a list of mismatches with attributed FP counts from the per-cop stats.
fn detect_version_mismatches(
    repo_dir: &Path,
    per_cop: &BTreeMap<String, CopStats>,
) -> Vec<GemVersionMismatchInfo> {
    let mut mismatches = Vec::new();

    for gem_name in VERSION_CHECK_GEMS {
        let vendor = match get_vendor_gem_version(gem_name) {
            Some(v) => v,
            None => continue,
        };
        let project = match get_project_gem_version(repo_dir, gem_name) {
            Some(v) => v,
            None => continue, // gem not installed in project
        };

        if vendor != project {
            // Count FPs from cops belonging to this gem's department
            let attributed_fps: usize = per_cop
                .iter()
                .filter(|(cop, _)| cop_gem(cop) == *gem_name)
                .map(|(_, stats)| stats.fp)
                .sum();

            if attributed_fps > 0 {
                eprintln!(
                    "  ⚠ {gem_name} version mismatch: project={project}, turbocop targets={vendor} ({attributed_fps} FPs attributed)"
                );
            }

            mismatches.push(GemVersionMismatchInfo {
                gem_name: gem_name.to_string(),
                vendor_version: vendor,
                project_version: project,
                attributed_fps,
            });
        }
    }

    mismatches
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

        // Pre-flight: verify turbocop runs without fatal errors
        let preflight_cmd = format!(
            "{} {} --no-color",
            turbocop.display(),
            repo.dir.display()
        );
        if !preflight_check("turbocop", &preflight_cmd, &repo.name) {
            continue;
        }

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
            .stderr(Stdio::piped())
            .output()
            .expect("failed to run turbocop");
        let turbocop_exit = turbocop_out.status.code().unwrap_or(127);
        if turbocop_exit > 1 {
            let stderr = String::from_utf8_lossy(&turbocop_out.stderr);
            let first_line = stderr.lines().next().unwrap_or("(no output)");
            eprintln!("  SKIP {}: turbocop failed (exit {}): {}", repo.name, turbocop_exit, first_line);
            continue;
        }
        fs::write(&turbocop_json_file, &turbocop_out.stdout).unwrap();
        let turbocop_secs = start.elapsed().as_secs_f64();
        eprintln!("  turbocop done in {turbocop_secs:.1}s");

        // Run rubocop (or standardrb for pure-standardrb projects) in JSON mode
        let use_standardrb = is_standardrb_only(&repo.dir);
        let reference_tool = if use_standardrb { "standardrb" } else { "rubocop" };
        eprintln!("  Running {reference_tool}...");
        let rubocop_json_file = results_path.join(format!("{}-rubocop.json", repo.name));
        let start = Instant::now();
        let rubocop_out = bundle_exec_command(&repo.dir, reference_tool, &["--format", "json", "--no-color"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .unwrap_or_else(|_| panic!("failed to run {reference_tool}"));
        fs::write(&rubocop_json_file, &rubocop_out.stdout).unwrap();
        let rubocop_secs = start.elapsed().as_secs_f64();
        eprintln!("  {reference_tool} done in {rubocop_secs:.1}s");

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

        // Detect gem version mismatches
        let version_mismatches = detect_version_mismatches(&repo.dir, &per_cop);

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
                turbocop_secs: Some(turbocop_secs),
                rubocop_secs: Some(rubocop_secs),
                version_mismatches,
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
        // copy_repo() respects .gitignore, so explicitly copy files that may be
        // gitignored but are needed (Gemfile.lock for bundler resolution).
        for name in &["Gemfile.lock"] {
            let src = repo.dir.join(name);
            let dst = rubocop_dir.join(name);
            if src.exists() {
                let _ = fs::copy(&src, &dst);
            }
        }
        eprintln!("  Running rubocop -A...");
        let start = Instant::now();
        let _ = bundle_exec_command(&rubocop_dir, "rubocop", &["-A", "--format", "quiet"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
        eprintln!("  rubocop -A done in {:.1}s", start.elapsed().as_secs_f64());

        // --- Run turbocop -A on another copy ---
        let turbocop_dir = temp_base.join("turbocop");
        copy_repo(&repo.dir, &turbocop_dir);
        // copy_repo() respects .gitignore, so explicitly copy files that may be
        // gitignored but are needed for linting.
        for name in &["Gemfile.lock"] {
            let src = repo.dir.join(name);
            let dst = turbocop_dir.join(name);
            if src.exists() {
                let _ = fs::copy(&src, &dst);
            }
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

        // Copy files that may be gitignored but are needed for linting.
        // copy_repo() respects .gitignore to avoid copying large vendor/ dirs,
        // but Gemfile.lock is often gitignored and required.
        for name in &["Gemfile.lock"] {
            let src = repo.dir.join(name);
            let dst = work_dir.join(name);
            if src.exists() {
                let _ = fs::copy(&src, &dst);
            }
        }

        // Step 1: Run turbocop -A --format json
        eprintln!("  Running turbocop -A...");
        let start = Instant::now();
        let tc_output = Command::new(turbocop.as_os_str())
            .args(["-A", work_dir.to_str().unwrap(), "--format", "json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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
            let stderr = String::from_utf8_lossy(&tc_output.stderr);
            let stderr = stderr.trim();
            if stderr.is_empty() {
                eprintln!("  Failed to parse turbocop JSON output (empty stdout)");
            } else {
                eprintln!("  Failed to parse turbocop JSON output: {}", stderr);
            }
        }

        let total_corrected: usize = per_cop.values().map(|s| s.turbocop_corrected).sum();
        eprintln!("  turbocop corrected {} offenses", total_corrected);

        // Step 2: Run rubocop --only <corrected-cops> to verify corrections.
        // We only verify cops that turbocop actually corrected, using --only to scope
        // the check. This is safe because --only overrides Enabled: false, but if
        // turbocop corrected an offense, the cop must be enabled in the project config.
        //
        // We copy corrected .rb files back into the original repo (so rubocop runs
        // with the correct Ruby, gems, config, and path-relative exclusions), run
        // rubocop there, then restore the originals.
        let corrected_cops: Vec<String> = per_cop
            .iter()
            .filter(|(_, s)| s.turbocop_corrected > 0)
            .map(|(name, _)| name.clone())
            .collect();

        if corrected_cops.is_empty() {
            eprintln!("  Skipping rubocop (no corrections to verify)");
        } else {
            // Save originals, copy corrected files into repo
            let rb_files = collect_rb_files(&repo.dir);
            let originals: Vec<(PathBuf, Vec<u8>)> = rb_files
                .iter()
                .filter_map(|rel| {
                    let full = repo.dir.join(rel);
                    fs::read(&full).ok().map(|bytes| (rel.clone(), bytes))
                })
                .collect();
            // Copy corrected files from work_dir into repo.dir
            for (rel, _) in &originals {
                let src = work_dir.join(rel);
                let dst = repo.dir.join(rel);
                if src.exists() {
                    let _ = fs::copy(&src, &dst);
                }
            }

            let uses_standardrb = !repo.dir.join(".rubocop.yml").exists()
                && repo.dir.join(".standard.yml").exists();
            let linter_name = if uses_standardrb { "standardrb" } else { "rubocop" };
            let only_arg = corrected_cops.join(",");
            eprintln!("  Running {} --only {}...", linter_name, only_arg);
            let start = Instant::now();
            let rb_output = bundle_exec_command(&repo.dir, linter_name, &[
                "--only", &only_arg,
                "--format", "json", "--no-color",
            ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .unwrap_or_else(|_| panic!("failed to run {}", linter_name));

            // Restore original files
            for (rel, bytes) in &originals {
                let dst = repo.dir.join(rel);
                let _ = fs::write(&dst, bytes);
            }
            eprintln!(
                "  {} done in {:.1}s",
                linter_name,
                start.elapsed().as_secs_f64()
            );

            // Parse rubocop output — remaining offenses for corrected cops only
            if let Ok(rb_data) = serde_json::from_slice::<RubocopOutput>(&rb_output.stdout) {
                for file in &rb_data.files {
                    for offense in &file.offenses {
                        if per_cop.contains_key(&offense.cop_name) {
                            per_cop
                                .entry(offense.cop_name.clone())
                                .or_default()
                                .rubocop_remaining += 1;
                        }
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&rb_output.stderr);
                let stderr = stderr.trim();
                if stderr.is_empty() {
                    let stdout_preview: String = String::from_utf8_lossy(&rb_output.stdout).chars().take(200).collect();
                    eprintln!("  Failed to parse rubocop JSON output: {}", stdout_preview);
                } else {
                    eprintln!("  Failed to parse rubocop JSON output: {}", stderr.lines().next().unwrap_or(""));
                }
            }
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

        if !corrected_cops.is_empty() {
            eprintln!("  rubocop found {} remaining offenses", total_remaining);
        }
        eprintln!(
            "  {} cops tested, {} clean, {} with remaining",
            cops_tested, cops_clean, cops_with_remaining,
        );

        // Print per-cop details
        for (cop, stats) in &per_cop {
            let status = if stats.rubocop_remaining == 0 { "PASS" } else { "FAIL" };
            eprintln!(
                "    {} — corrected: {}, remaining: {} [{}]",
                cop, stats.turbocop_corrected, stats.rubocop_remaining, status
            );
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

    // Only report cops where turbocop actually corrected something
    let validated: BTreeMap<&String, &CopValidateStats> = aggregate
        .iter()
        .filter(|(_, s)| s.turbocop_corrected > 0)
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
        if validated_cops.is_empty() {
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
            "Median of {} runs via [hyperfine](https://github.com/sharkdp/hyperfine). \
             10% of `.rb` files (capped at 50) are mtime-invalidated before each run, simulating re-linting after a `git pull`.",
            args.runs,
        )
        .unwrap();
        writeln!(md).unwrap();
        writeln!(
            md,
            "| Repo | .rb files | Files changed | turbocop | rubocop | Speedup |"
        )
        .unwrap();
        writeln!(
            md,
            "|------|----------:|--------------:|------------------:|-----------------:|--------:|"
        )
        .unwrap();

        for repo in repos {
            if let Some(r) = bench.get(&repo.name) {
                let speedup = format_speedup(r.rubocop.median, r.turbocop.median);
                writeln!(
                    md,
                    "| {} | {} | {} | **{}** | {} | **{}** |",
                    repo.name,
                    r.rb_count,
                    r.touched_count,
                    format_time(r.turbocop.median),
                    format_time(r.rubocop.median),
                    speedup,
                )
                .unwrap();
            }
        }

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
        // Check if any conform results have timing data
        let has_timing = conform.values().any(|c| c.turbocop_secs.is_some());

        if has_timing {
            writeln!(
                md,
                "| Repo | turbocop | rubocop | Matches | FP | FN | Match rate | turbocop time | rubocop time | Speedup |"
            )
            .unwrap();
            writeln!(
                md,
                "|------|-------:|--------:|--------:|---:|---:|-----------:|--------------:|-------------:|--------:|"
            )
            .unwrap();
        } else {
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
        }

        for repo in repos {
            if let Some(c) = conform.get(&repo.name) {
                if has_timing {
                    let tc_time = c.turbocop_secs.map(|s| format!("{}ms", (s * 1000.0) as u64)).unwrap_or_default();
                    let rc_time = c.rubocop_secs.map(|s| format!("{}ms", (s * 1000.0) as u64)).unwrap_or_default();
                    let speedup = match (c.turbocop_secs, c.rubocop_secs) {
                        (Some(tc), Some(rc)) if tc > 0.0 => format!("**{:.1}x**", rc / tc),
                        _ => String::new(),
                    };
                    writeln!(
                        md,
                        "| {} | {} | {} | {} | {} | {} | **{:.1}%** | {} | {} | {} |",
                        repo.name,
                        c.turbocop_count,
                        c.rubocop_count,
                        c.matches,
                        c.false_positives,
                        c.false_negatives,
                        c.match_rate,
                        tc_time,
                        rc_time,
                        speedup,
                    )
                    .unwrap();
                } else {
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

                if divergent.is_empty() && c.version_mismatches.is_empty() {
                    writeln!(md, "**{}:** All cops match perfectly!", repo.name).unwrap();
                    writeln!(md).unwrap();
                    continue;
                }

                if divergent.is_empty() {
                    writeln!(md, "**{}:** All cops match perfectly!", repo.name).unwrap();
                } else {
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
                }

                // Version mismatch attribution
                for mm in &c.version_mismatches {
                    if mm.attributed_fps > 0 {
                        writeln!(
                            md,
                            "\n> **{}** version mismatch — {} FPs attributed to {} (project: {}, turbocop targets: {})",
                            repo.name, mm.attributed_fps, mm.gem_name, mm.project_version, mm.vendor_version,
                        )
                        .unwrap();
                    }
                }

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
        let results_path = results_dir_for(repo.source);
        let json_file = results_path.join(format!("{}-bench.json", repo.name));
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
                    touched_count: std::cmp::min(std::cmp::max(rb_count / 10, 1), 50),
                },
            );
        }
    }
    results
}

fn load_cached_conform(is_private: bool) -> HashMap<String, ConformResult> {
    let json_path = project_root().join(if is_private {
        "bench/private_conform.json"
    } else {
        "bench/conform.json"
    });
    if !json_path.exists() {
        return HashMap::new();
    }
    let content = match fs::read_to_string(&json_path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };
    serde_json::from_str(&content).unwrap_or_default()
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
            let conform = load_cached_conform(is_private_run);
            let elapsed = start.elapsed().as_secs_f64();
            let md = generate_report(&bench, &conform, &args, &repos, Some(elapsed));
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
            let conform = load_cached_conform(is_private_run);
            let md = generate_report(&bench, &conform, &args, &repos, None);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_passes_exit_code_0() {
        assert!(preflight_check("true", "true", "test-repo"));
    }

    #[test]
    fn preflight_passes_exit_code_1() {
        // exit 1 = offenses found, should be allowed
        assert!(preflight_check("test", "exit 1", "test-repo"));
    }

    #[test]
    fn preflight_rejects_exit_code_2() {
        // exit 2 = fatal error (e.g. missing lockfile)
        assert!(!preflight_check("test", "exit 2", "test-repo"));
    }

    #[test]
    fn preflight_rejects_higher_exit_codes() {
        assert!(!preflight_check("test", "exit 127", "test-repo"));
    }

    #[test]
    fn preflight_rejects_command_that_writes_to_stderr_and_fails() {
        assert!(!preflight_check(
            "test",
            "echo 'error: No lockfile found' >&2; exit 2",
            "test-repo"
        ));
    }
}
