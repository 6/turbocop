use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;

use rayon::prelude::*;
use ruby_prism::Visit;

use crate::cli::Args;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::registry::CopRegistry;
use crate::cop::walker::BatchedCopWalker;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Thread-safe phase timing counters (nanoseconds) for profiling.
struct PhaseTimers {
    file_io_ns: AtomicU64,
    parse_ns: AtomicU64,
    codemap_ns: AtomicU64,
    cop_exec_ns: AtomicU64,
    cop_filter_ns: AtomicU64,
    cop_ast_ns: AtomicU64,
    disable_ns: AtomicU64,
}

impl PhaseTimers {
    fn new() -> Self {
        Self {
            file_io_ns: AtomicU64::new(0),
            parse_ns: AtomicU64::new(0),
            codemap_ns: AtomicU64::new(0),
            cop_exec_ns: AtomicU64::new(0),
            cop_filter_ns: AtomicU64::new(0),
            cop_ast_ns: AtomicU64::new(0),
            disable_ns: AtomicU64::new(0),
        }
    }

    fn print_summary(&self, total: std::time::Duration, file_count: usize) {
        let file_io = std::time::Duration::from_nanos(self.file_io_ns.load(Ordering::Relaxed));
        let parse = std::time::Duration::from_nanos(self.parse_ns.load(Ordering::Relaxed));
        let codemap = std::time::Duration::from_nanos(self.codemap_ns.load(Ordering::Relaxed));
        let cop_exec = std::time::Duration::from_nanos(self.cop_exec_ns.load(Ordering::Relaxed));
        let disable = std::time::Duration::from_nanos(self.disable_ns.load(Ordering::Relaxed));
        let accounted = file_io + parse + codemap + cop_exec + disable;

        eprintln!("debug: --- linter phase breakdown ({file_count} files) ---");
        eprintln!("debug:   file I/O:       {file_io:.0?} (cumulative across threads)");
        eprintln!("debug:   prism parse:    {parse:.0?}");
        eprintln!("debug:   codemap build:  {codemap:.0?}");
        let cop_filter = std::time::Duration::from_nanos(self.cop_filter_ns.load(Ordering::Relaxed));
        let cop_ast = std::time::Duration::from_nanos(self.cop_ast_ns.load(Ordering::Relaxed));
        eprintln!("debug:   cop execution:  {cop_exec:.0?}");
        eprintln!("debug:     filter+config:  {cop_filter:.0?}");
        eprintln!("debug:     AST walk:       {cop_ast:.0?}");
        eprintln!("debug:   disable filter: {disable:.0?}");
        eprintln!("debug:   accounted:      {accounted:.0?} (sum of per-thread time)");
        eprintln!("debug:   wall clock:     {total:.0?}");
    }
}

/// Renamed cops from vendor/rubocop/config/obsoletion.yml.
/// Maps old cop name -> new cop name (e.g., "Naming/PredicateName" -> "Naming/PredicatePrefix").
static RENAMED_COPS: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    parse_renamed_cops(include_str!(
        "../vendor/rubocop/config/obsoletion.yml"
    ))
});

/// Parse the `renamed:` section from obsoletion.yml content.
///
/// The YAML format supports two styles:
///   - Simple:   `OldName: NewName`
///   - Extended:  `OldName:\n  new_name: NewName\n  severity: warning`
fn parse_renamed_cops(yaml_content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();

    let raw: serde_yml::Value = match serde_yml::from_str(yaml_content) {
        Ok(v) => v,
        Err(_) => return map,
    };

    let Some(renamed) = raw.get("renamed") else {
        return map;
    };
    let Some(mapping) = renamed.as_mapping() else {
        return map;
    };

    for (key, value) in mapping {
        let Some(old_name) = key.as_str() else {
            continue;
        };

        let new_name = if let Some(s) = value.as_str() {
            // Simple format: OldName: NewName
            s.to_string()
        } else if let Some(inner_map) = value.as_mapping() {
            // Extended format: OldName: { new_name: NewName, severity: ... }
            inner_map
                .get(&serde_yml::Value::String("new_name".to_string()))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default()
        } else {
            continue;
        };

        if !new_name.is_empty() {
            map.insert(old_name.to_string(), new_name);
        }
    }

    map
}

pub struct LintResult {
    pub diagnostics: Vec<Diagnostic>,
    pub file_count: usize,
}

/// Lint a single SourceFile (already loaded into memory). Used for --stdin mode.
pub fn lint_source(
    source: &SourceFile,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    args: &Args,
) -> LintResult {
    let cop_filters = config.build_cop_filters(registry);
    let base_configs = config.precompute_cop_configs(registry);
    let has_dir_overrides = config.has_dir_overrides();
    let diagnostics = lint_source_inner(
        source,
        config,
        registry,
        args,
        &cop_filters,
        &base_configs,
        has_dir_overrides,
        None,
    );
    let mut sorted = diagnostics;
    sorted.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    LintResult {
        diagnostics: sorted,
        file_count: 1,
    }
}

pub fn run_linter(
    files: &[std::path::PathBuf],
    config: &ResolvedConfig,
    registry: &CopRegistry,
    args: &Args,
) -> LintResult {
    let wall_start = std::time::Instant::now();
    // Build cop filters once before the parallel loop
    let cop_filters = config.build_cop_filters(registry);
    // Pre-compute base cop configs once (avoids HashMap clone per cop per file)
    let base_configs = config.precompute_cop_configs(registry);
    let has_dir_overrides = config.has_dir_overrides();

    let timers = if args.debug {
        Some(PhaseTimers::new())
    } else {
        None
    };

    let diagnostics: Vec<Diagnostic> = files
        .par_iter()
        .flat_map(|path| {
            lint_file(
                path,
                config,
                registry,
                args,
                &cop_filters,
                &base_configs,
                has_dir_overrides,
                timers.as_ref(),
            )
        })
        .collect();

    let mut sorted = diagnostics;
    sorted.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));

    if let Some(ref t) = timers {
        t.print_summary(wall_start.elapsed(), files.len());
    }

    // Per-cop timing: enabled by RBLINT_COP_PROFILE=1
    if std::env::var("RBLINT_COP_PROFILE").is_ok() {
        use std::sync::Mutex;
        let cop_timings: Vec<Mutex<(u64, u64, u64)>> = (0..registry.cops().len())
            .map(|_| Mutex::new((0u64, 0u64, 0u64)))
            .collect();
        // Re-run single-threaded for profiling
        for path in files {
            if cop_filters.is_globally_excluded(path) {
                continue;
            }
            let source = match SourceFile::from_path(path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let parse_result = crate::parse::parse_source(source.as_bytes());
            let code_map = CodeMap::from_parse_result(source.as_bytes(), &parse_result);
            for (i, cop) in registry.cops().iter().enumerate() {
                if !cop_filters.is_cop_match(i, &source.path) {
                    continue;
                }
                let cop_config = &base_configs[i];
                let t0 = std::time::Instant::now();
                let mut d = Vec::new();
                cop.check_lines(&source, cop_config, &mut d);
                let lines_ns = t0.elapsed().as_nanos() as u64;
                let t1 = std::time::Instant::now();
                cop.check_source(&source, &parse_result, &code_map, cop_config, &mut d);
                let source_ns = t1.elapsed().as_nanos() as u64;
                let t2 = std::time::Instant::now();
                // check_node via single-cop walker
                if !cop.interested_node_types().is_empty() || cop.name().contains('/') {
                    let ast_cops: Vec<(&dyn Cop, &CopConfig)> = vec![(&**cop, cop_config)];
                    let mut walker = BatchedCopWalker::new(ast_cops, &source, &parse_result);
                    walker.visit(&parse_result.node());
                }
                let ast_ns = t2.elapsed().as_nanos() as u64;
                let mut m = cop_timings[i].lock().unwrap();
                m.0 += lines_ns;
                m.1 += source_ns;
                m.2 += ast_ns;
            }
        }
        let mut entries: Vec<(String, u64, u64, u64)> = registry
            .cops()
            .iter()
            .enumerate()
            .map(|(i, cop)| {
                let m = cop_timings[i].lock().unwrap();
                (cop.name().to_string(), m.0, m.1, m.2)
            })
            .filter(|(_, l, s, a)| *l + *s + *a > 0)
            .collect();
        entries.sort_by(|a, b| (b.1 + b.2 + b.3).cmp(&(a.1 + a.2 + a.3)));
        eprintln!("\n=== Per-cop timing (top 30) ===");
        eprintln!("{:<50} {:>10} {:>10} {:>10} {:>10}", "Cop", "lines", "source", "ast", "total");
        for (name, l, s, a) in entries.iter().take(30) {
            let lms = *l as f64 / 1_000_000.0;
            let sms = *s as f64 / 1_000_000.0;
            let ams = *a as f64 / 1_000_000.0;
            let total = lms + sms + ams;
            eprintln!("{:<50} {:>9.1}ms {:>9.1}ms {:>9.1}ms {:>9.1}ms", name, lms, sms, ams, total);
        }
        let total_all: u64 = entries.iter().map(|(_, l, s, a)| l + s + a).sum();
        eprintln!("{:<50} {:>10} {:>10} {:>10} {:>9.1}ms", "TOTAL", "", "", "", total_all as f64 / 1_000_000.0);
    }

    LintResult {
        diagnostics: sorted,
        file_count: files.len(),
    }
}

fn lint_file(
    path: &Path,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    args: &Args,
    cop_filters: &CopFilterSet,
    base_configs: &[CopConfig],
    has_dir_overrides: bool,
    timers: Option<&PhaseTimers>,
) -> Vec<Diagnostic> {
    // Check global excludes once per file
    if cop_filters.is_globally_excluded(path) {
        return Vec::new();
    }

    let io_start = std::time::Instant::now();
    let source = match SourceFile::from_path(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e:#}");
            return Vec::new();
        }
    };
    if let Some(t) = timers {
        t.file_io_ns
            .fetch_add(io_start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }

    lint_source_inner(
        &source,
        config,
        registry,
        args,
        cop_filters,
        base_configs,
        has_dir_overrides,
        timers,
    )
}

/// Name of the redundant cop disable directive cop.
const REDUNDANT_DISABLE_COP: &str = "Lint/RedundantCopDisableDirective";

/// Determine if a disable directive should be flagged as redundant.
///
/// Returns true if the directive IS redundant (should be reported), false if
/// we should skip it.
///
/// The logic is conservative to avoid false positives:
///   - "all" or department-only directives: never flag (too broad to check)
///   - Known cop that is explicitly disabled (Enabled: false): flag as redundant
///   - Known cop that is enabled: don't flag (rblint may have detection gaps)
///   - Renamed cop (per obsoletion.yml) whose new name IS in the registry:
///     flag as redundant (the old name is obsolete)
///   - Cop NOT in the registry but known from gem config (has Include/Exclude):
///     flag as redundant if the file is excluded by those patterns
///   - Completely unknown cop: never flag (could be custom/project-local)
fn is_directive_redundant(
    cop_name: &str,
    registry: &CopRegistry,
    cop_filters: &CopFilterSet,
    config: &ResolvedConfig,
    path: &Path,
) -> bool {
    // "all" is a wildcard — never flag (too broad to determine redundancy)
    if cop_name == "all" {
        return false;
    }

    // Department-only name (no '/') — never flag (too broad to check)
    if !cop_name.contains('/') {
        return false;
    }

    // Fully qualified cop name — check if it's in the registry
    let cop_entry = registry
        .cops()
        .iter()
        .enumerate()
        .find(|(_, c)| c.name() == cop_name);

    if let Some((idx, _)) = cop_entry {
        // Cop IS in the registry.
        let filter = cop_filters.cop_filter(idx);
        if !filter.is_enabled() {
            // Cop is explicitly disabled — the disable directive is redundant.
            true
        } else {
            // Cop is enabled — don't flag even if excluded by Include/Exclude.
            // The Include/Exclude matching uses relative paths which may not
            // resolve correctly for all file paths, and rblint may have
            // detection gaps vs. RuboCop. Conservative approach: only flag
            // explicitly disabled cops.
            false
        }
    } else {
        // Cop is NOT in the registry. Check if it's a renamed cop whose new
        // name IS in the registry and is enabled. RuboCop treats disable
        // directives for renamed cops as redundant since the old name no
        // longer exists.
        if let Some(new_name) = RENAMED_COPS.get(cop_name) {
            let new_cop_entry = registry
                .cops()
                .iter()
                .enumerate()
                .find(|(_, c)| c.name() == new_name.as_str());

            if let Some((_idx, _)) = new_cop_entry {
                // The renamed-to cop IS in the registry.
                // Regardless of enabled/disabled state, a disable for the old
                // (renamed) name is always redundant — the old cop no longer exists.
                return true;
            }
        }

        // Not a renamed cop (or renamed-to cop is also not in registry).
        // Check if it's known from gem config (has Include/Exclude patterns).
        // For example, Discourse/* cops from rubocop-discourse have Include
        // patterns that limit them to spec files — a disable directive in a
        // non-spec file is redundant. Similarly, cops excluded from certain
        // paths via Exclude patterns (e.g., **/app/controllers/**) are
        // redundant to disable in those files.
        let cop_config = config.cop_config(cop_name);
        if !cop_config.include.is_empty() || !cop_config.exclude.is_empty() {
            if !cop_filters.is_path_matched_by_cop_config(&cop_config, path) {
                return true;
            }
        }

        // Unknown cop with no Include/Exclude info, or file IS matched —
        // don't flag. Could be a custom cop, project-local cop, or the
        // directive is genuinely needed.
        false
    }
}

fn lint_source_inner(
    source: &SourceFile,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    args: &Args,
    cop_filters: &CopFilterSet,
    base_configs: &[CopConfig],
    has_dir_overrides: bool,
    timers: Option<&PhaseTimers>,
) -> Vec<Diagnostic> {
    // Parse on this thread (ParseResult is !Send)
    let parse_start = std::time::Instant::now();
    let parse_result = crate::parse::parse_source(source.as_bytes());
    if let Some(t) = timers {
        t.parse_ns
            .fetch_add(parse_start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }

    let codemap_start = std::time::Instant::now();
    let code_map = CodeMap::from_parse_result(source.as_bytes(), &parse_result);
    if let Some(t) = timers {
        t.codemap_ns
            .fetch_add(codemap_start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }

    let mut diagnostics = Vec::new();

    let cop_start = std::time::Instant::now();
    let filter_start = std::time::Instant::now();

    // Collect enabled cops and their configs, run line/source checks eagerly.
    // AST checks are deferred to a single batched walk below.
    // We use references to pre-computed base configs to avoid cloning.
    // override_configs stores the rare per-file overrides (only when dir_overrides exist).
    // We collect cop indices and build ast_cops after the loop to satisfy the borrow checker
    // (pushing to override_configs invalidates references, so we defer reference creation).
    let mut ast_cop_indices: Vec<(usize, Option<usize>)> = Vec::new();
    let mut override_configs: Vec<CopConfig> = Vec::new();

    for (i, cop) in registry.cops().iter().enumerate() {
        let name = cop.name();

        // Skip RedundantCopDisableDirective in the normal cop loop — it's handled
        // in post-processing below.
        if name == REDUNDANT_DISABLE_COP {
            continue;
        }

        // Filter by --only / --except
        if !args.only.is_empty() && !args.only.iter().any(|o| o == name) {
            continue;
        }
        if args.except.iter().any(|e| e == name) {
            continue;
        }

        // Use pre-compiled cop filter (checks enabled state + include/exclude globs).
        // is_cop_match relativizes path against config_dir so relative patterns
        // (e.g., `lib/mastodon/cli/*.rb`) work when running from outside the project root.
        if !cop_filters.is_cop_match(i, &source.path) {
            continue;
        }

        // Use pre-computed base config; only apply dir overrides when they exist.
        let override_idx = if has_dir_overrides {
            config
                .apply_dir_override(&base_configs[i], name, &source.path)
                .map(|merged| {
                    let idx = override_configs.len();
                    override_configs.push(merged);
                    idx
                })
        } else {
            None
        };

        let cop_config = match override_idx {
            Some(idx) => &override_configs[idx],
            None => &base_configs[i],
        };

        // Line-based checks
        cop.check_lines(source, cop_config, &mut diagnostics);

        // Source-based checks (raw byte scanning with CodeMap)
        cop.check_source(source, &parse_result, &code_map, cop_config, &mut diagnostics);

        ast_cop_indices.push((i, override_idx));
    }

    if let Some(t) = timers {
        t.cop_filter_ns
            .fetch_add(filter_start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }

    // Build ast_cops from indices (override_configs is now stable — no more pushes).
    let ast_start = std::time::Instant::now();
    if !ast_cop_indices.is_empty() {
        let ast_cops: Vec<(&dyn Cop, &CopConfig)> = ast_cop_indices
            .iter()
            .map(|&(i, override_idx)| {
                let cop: &dyn Cop = &*registry.cops()[i];
                let cfg = match override_idx {
                    Some(idx) => &override_configs[idx],
                    None => &base_configs[i],
                };
                (cop, cfg)
            })
            .collect();
        let mut walker = BatchedCopWalker::new(ast_cops, source, &parse_result);
        walker.visit(&parse_result.node());
        diagnostics.extend(walker.diagnostics);
    }
    if let Some(t) = timers {
        t.cop_ast_ns
            .fetch_add(ast_start.elapsed().as_nanos() as u64, Ordering::Relaxed);
        t.cop_exec_ns
            .fetch_add(cop_start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }

    // Filter out diagnostics suppressed by inline disable comments,
    // and detect redundant disable directives.
    let disable_start = std::time::Instant::now();
    let mut disabled =
        crate::parse::directives::DisabledRanges::from_comments(source, &parse_result);

    if !disabled.is_empty() {
        // Use check_and_mark_used to both filter diagnostics and track which
        // directives actually suppressed something.
        diagnostics.retain(|d| !disabled.check_and_mark_used(&d.cop_name, d.location.line));
    }

    // Generate Lint/RedundantCopDisableDirective offenses for unused directives.
    //
    // Flag a directive as redundant when:
    //   - The referenced cop is in the registry AND explicitly disabled (Enabled: false)
    //   - The referenced cop is in the registry, enabled, but excluded from this
    //     file by Include/Exclude patterns
    //   - The referenced cop is a renamed cop (old name is obsolete)
    //   - The referenced cop is from a gem config with Include/Exclude patterns
    //     that exclude this file
    //
    // Only run when:
    // 1. There are disable directives to check
    // 2. --only is empty (running all cops) — when --only filters cops, unused
    //    directives are expected since filtered cops don't generate diagnostics
    // 3. The cop itself is enabled in config
    // 4. The cop is not in --except
    if disabled.has_directives()
        && args.only.is_empty()
        && !args.except.iter().any(|e| e == REDUNDANT_DISABLE_COP)
    {
        // Check if the RedundantCopDisableDirective cop itself is enabled
        let cop_enabled = registry
            .cops()
            .iter()
            .enumerate()
            .find(|(_, c)| c.name() == REDUNDANT_DISABLE_COP)
            .is_some_and(|(idx, _)| cop_filters.is_cop_match(idx, &source.path));

        if cop_enabled {
            for directive in disabled.unused_directives() {
                if !is_directive_redundant(
                    &directive.cop_name,
                    registry,
                    cop_filters,
                    config,
                    &source.path,
                ) {
                    continue;
                }

                let message =
                    format!("Unnecessary disabling of `{}`.", directive.cop_name);
                diagnostics.push(Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location {
                        line: directive.line,
                        column: directive.column,
                    },
                    severity: Severity::Warning,
                    cop_name: REDUNDANT_DISABLE_COP.to_string(),
                    message,
                });
            }
        }
    }
    if let Some(t) = timers {
        t.disable_ns
            .fetch_add(disable_start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }

    diagnostics
}
