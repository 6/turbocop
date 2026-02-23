use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use rayon::prelude::*;
use ruby_prism::Visit;

use crate::cache::ResultCache;
use crate::cli::Args;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::registry::CopRegistry;
use crate::cop::tiers::{SkipSummary, TierMap};
use crate::cop::walker::BatchedCopWalker;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::fs::DiscoveredFiles;
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
        let cop_filter =
            std::time::Duration::from_nanos(self.cop_filter_ns.load(Ordering::Relaxed));
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
static RENAMED_COPS: LazyLock<HashMap<String, String>> =
    LazyLock::new(|| parse_renamed_cops(include_str!("../vendor/rubocop/config/obsoletion.yml")));

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
                .get(serde_yml::Value::String("new_name".to_string()))
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
    pub corrected_count: usize,
    pub skip_summary: SkipSummary,
}

/// Lint a single SourceFile (already loaded into memory). Used for --stdin mode.
pub fn lint_source(
    source: &SourceFile,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    args: &Args,
    tier_map: &TierMap,
    allowlist: &crate::cop::autocorrect_allowlist::AutocorrectAllowlist,
) -> LintResult {
    let cop_filters = config.build_cop_filters(registry, tier_map, args.preview);
    let base_configs = config.precompute_cop_configs(registry);
    let has_dir_overrides = config.has_dir_overrides();
    let (diagnostics, _corrected_bytes, corrected_count) = lint_source_inner(
        source,
        config,
        registry,
        args,
        &cop_filters,
        &base_configs,
        has_dir_overrides,
        None,
        allowlist,
    );
    let mut sorted = diagnostics;
    sorted.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    let skip_summary = config.compute_skip_summary(registry, tier_map, args.preview);
    LintResult {
        diagnostics: sorted,
        file_count: 1,
        corrected_count,
        skip_summary,
    }
}

pub fn run_linter(
    discovered: &DiscoveredFiles,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    args: &Args,
    tier_map: &TierMap,
    allowlist: &crate::cop::autocorrect_allowlist::AutocorrectAllowlist,
) -> LintResult {
    let files = &discovered.files;
    let wall_start = std::time::Instant::now();

    // Initialize schema (db/schema.rb) for schema-aware cops
    crate::schema::init(config.config_dir());

    // Build cop filters once before the parallel loop
    let cop_filters = config.build_cop_filters(registry, tier_map, args.preview);
    // Pre-compute base cop configs once (avoids HashMap clone per cop per file)
    let base_configs = config.precompute_cop_configs(registry);
    let has_dir_overrides = config.has_dir_overrides();

    // Result cache: enabled by default, disable with --cache false or autocorrect
    let cache_enabled = args.cache == "true"
        && args.stdin.is_none()
        && args.autocorrect_mode() == crate::cli::AutocorrectMode::Off;
    let cache = if cache_enabled {
        let c = ResultCache::new(env!("CARGO_PKG_VERSION"), &base_configs, args);
        if args.debug {
            eprintln!("debug: result cache enabled");
        }
        c
    } else {
        if args.debug && args.cache != "true" {
            eprintln!("debug: result cache disabled (--cache false)");
        }
        ResultCache::disabled()
    };

    let timers = if args.debug {
        Some(PhaseTimers::new())
    } else {
        None
    };

    let cache_stat_hits = std::sync::atomic::AtomicUsize::new(0);
    let cache_content_hits = std::sync::atomic::AtomicUsize::new(0);
    let cache_misses = std::sync::atomic::AtomicUsize::new(0);
    let found_offense = AtomicBool::new(false);
    let total_corrected = std::sync::atomic::AtomicUsize::new(0);

    let diagnostics: Vec<Diagnostic> = files
        .par_iter()
        .flat_map(|path| {
            // --fail-fast: skip remaining files once an offense is found
            if args.fail_fast && found_offense.load(Ordering::Relaxed) {
                return Vec::new();
            }
            let result = lint_file(
                path,
                config,
                registry,
                args,
                &cop_filters,
                &base_configs,
                has_dir_overrides,
                timers.as_ref(),
                &cache,
                &cache_stat_hits,
                &cache_content_hits,
                &cache_misses,
                &discovered.explicit,
                &total_corrected,
                allowlist,
            );
            if args.fail_fast && !result.is_empty() {
                found_offense.store(true, Ordering::Relaxed);
            }
            result
        })
        .collect();

    let mut sorted = diagnostics;
    sorted.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));

    if let Some(ref t) = timers {
        t.print_summary(wall_start.elapsed(), files.len());
    }

    if args.debug && cache.is_enabled() {
        let stat_hits = cache_stat_hits.load(std::sync::atomic::Ordering::Relaxed);
        let content_hits = cache_content_hits.load(std::sync::atomic::Ordering::Relaxed);
        let misses = cache_misses.load(std::sync::atomic::Ordering::Relaxed);
        eprintln!(
            "debug: cache stat_hits: {stat_hits}, content_hits: {content_hits}, misses: {misses}"
        );
    }

    // Flush in-memory cache to disk, then run eviction (best-effort)
    if cache.is_enabled() {
        cache.flush();
        cache.evict(50);
    }

    // Per-cop timing: enabled by TURBOCOP_COP_PROFILE=1
    if std::env::var("TURBOCOP_COP_PROFILE").is_ok() {
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
                cop.check_lines(&source, cop_config, &mut d, None);
                let lines_ns = t0.elapsed().as_nanos() as u64;
                let t1 = std::time::Instant::now();
                cop.check_source(&source, &parse_result, &code_map, cop_config, &mut d, None);
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
        eprintln!(
            "{:<50} {:>10} {:>10} {:>10} {:>10}",
            "Cop", "lines", "source", "ast", "total"
        );
        for (name, l, s, a) in entries.iter().take(30) {
            let lms = *l as f64 / 1_000_000.0;
            let sms = *s as f64 / 1_000_000.0;
            let ams = *a as f64 / 1_000_000.0;
            let total = lms + sms + ams;
            eprintln!(
                "{:<50} {:>9.1}ms {:>9.1}ms {:>9.1}ms {:>9.1}ms",
                name, lms, sms, ams, total
            );
        }
        let total_all: u64 = entries.iter().map(|(_, l, s, a)| l + s + a).sum();
        eprintln!(
            "{:<50} {:>10} {:>10} {:>10} {:>9.1}ms",
            "TOTAL",
            "",
            "",
            "",
            total_all as f64 / 1_000_000.0
        );
    }

    let corrected_count = total_corrected.load(std::sync::atomic::Ordering::Relaxed);
    let skip_summary = config.compute_skip_summary(registry, tier_map, args.preview);
    LintResult {
        diagnostics: sorted,
        file_count: files.len(),
        corrected_count,
        skip_summary,
    }
}

#[allow(clippy::too_many_arguments)] // orchestration entry point threading shared state
fn lint_file(
    path: &Path,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    args: &Args,
    cop_filters: &CopFilterSet,
    base_configs: &[CopConfig],
    has_dir_overrides: bool,
    timers: Option<&PhaseTimers>,
    cache: &ResultCache,
    cache_stat_hits: &std::sync::atomic::AtomicUsize,
    cache_content_hits: &std::sync::atomic::AtomicUsize,
    cache_misses: &std::sync::atomic::AtomicUsize,
    explicit_files: &HashSet<std::path::PathBuf>,
    total_corrected: &std::sync::atomic::AtomicUsize,
    allowlist: &crate::cop::autocorrect_allowlist::AutocorrectAllowlist,
) -> Vec<Diagnostic> {
    use crate::cache::CacheLookup;

    // Check global excludes once per file.
    // Explicitly-passed files bypass AllCops.Exclude (matching RuboCop default)
    // unless --force-exclusion is set.
    if cop_filters.is_globally_excluded(path) {
        let is_explicit = explicit_files.contains(path)
            || path
                .canonicalize()
                .ok()
                .is_some_and(|c| explicit_files.contains(&c));
        if args.force_exclusion || !is_explicit {
            return Vec::new();
        }
    }

    // Tier 1: stat check (mtime + size) — no file read needed
    if cache.is_enabled() {
        if let CacheLookup::StatHit(cached) = cache.get_by_stat(path) {
            cache_stat_hits.fetch_add(1, Ordering::Relaxed);
            return cached;
        }
    }

    // File read needed (either cache disabled, stat miss, or no entry)
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

    // Tier 2: content hash check — file was read, mtime didn't match
    if cache.is_enabled() {
        if let CacheLookup::ContentHit(cached) = cache.get_by_content(path, source.as_bytes()) {
            cache_content_hits.fetch_add(1, Ordering::Relaxed);
            return cached;
        }
        cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    let (result, corrected_bytes, corrected_count) = lint_source_inner(
        &source,
        config,
        registry,
        args,
        cop_filters,
        base_configs,
        has_dir_overrides,
        timers,
        allowlist,
    );
    if corrected_count > 0 {
        total_corrected.fetch_add(corrected_count, Ordering::Relaxed);
    }

    // Write corrected bytes to disk if autocorrect produced changes
    if let Some(bytes) = corrected_bytes {
        if let Err(e) = std::fs::write(path, &bytes) {
            eprintln!(
                "error: failed to write corrected file {}: {e}",
                path.display()
            );
        }
    }

    // Store result in cache
    if cache.is_enabled() {
        cache.put(path, source.as_bytes(), &result);
    }

    result
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
///   - Known cop that is enabled: don't flag (turbocop may have detection gaps)
///   - Renamed cop (per obsoletion.yml) whose new name IS in the registry:
///     flag as redundant (the old name is obsolete)
///   - Cop NOT in the registry but known from gem config (has Include/Exclude):
///     flag as redundant if the file is excluded by those patterns
///   - Completely unknown cop: never flag (could be custom/project-local)
fn is_directive_redundant(
    cop_name: &str,
    registry: &CopRegistry,
    cop_filters: &CopFilterSet,
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

    // Self-referential: disabling RedundantCopDisableDirective itself is
    // legitimate (used in RuboCop's own source, for example).
    if cop_name == REDUNDANT_DISABLE_COP {
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
            return true;
        }
        // Cop is enabled. Check if it's explicitly excluded from this file
        // by Exclude patterns (e.g., Lint/UselessMethodDefinition excluded
        // from app/controllers/**). Only check Exclude, NOT Include — Include
        // mismatches can arise from sub-config directory path issues and are
        // not reliable indicators of redundancy.
        if cop_filters.is_cop_excluded(idx, path) {
            return true;
        }
        // Cop is enabled and not explicitly excluded — don't flag.
        // Conservative: even if the cop didn't execute (Include mismatch),
        // sub-config path resolution issues could cause false positives.
        false
    } else {
        // Cop is NOT in the registry. Check if it's a renamed cop whose new
        // name IS in the registry and is enabled. RuboCop treats disable
        // directives for renamed cops as redundant since the old name no
        // longer exists.
        if RENAMED_COPS.contains_key(cop_name) {
            // The cop was renamed. RuboCop flags disable directives for
            // renamed cops as redundant (with "Did you mean <new name>?").
            return true;
        }

        // Not a renamed cop (or renamed-to cop is also not in registry).
        // Don't flag unknown cops as redundant — they could be custom cops,
        // project-local cops, or from plugins we don't implement. RuboCop
        // only flags directives as redundant when the cop didn't fire any
        // offenses; it doesn't check Include/Exclude patterns for redundancy.
        false
    }
}

/// Validate that corrected bytes are still valid Ruby by re-parsing with Prism.
/// Returns `None` (discarding corrections) if parse errors are found.
fn validate_corrected_bytes(
    original_bytes: &[u8],
    current_bytes: Vec<u8>,
    path: &std::path::Path,
) -> Option<Vec<u8>> {
    if current_bytes == original_bytes {
        return None;
    }
    // Scope the parse_result so its borrow of current_bytes is dropped before we return.
    let has_errors = {
        let parse_result = crate::parse::parse_source(&current_bytes);
        parse_result.errors().count() > 0
    };
    if has_errors {
        eprintln!(
            "warning: autocorrect produced invalid syntax for {}, skipping corrections",
            path.display()
        );
        return None;
    }
    Some(current_bytes)
}

/// Returns (diagnostics, corrected_bytes, corrected_count).
/// corrected_count is the total number of offenses corrected across all iterations.
#[allow(clippy::too_many_arguments)] // internal lint pipeline threading shared state
fn lint_source_inner(
    source: &SourceFile,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    args: &Args,
    cop_filters: &CopFilterSet,
    base_configs: &[CopConfig],
    has_dir_overrides: bool,
    timers: Option<&PhaseTimers>,
    allowlist: &crate::cop::autocorrect_allowlist::AutocorrectAllowlist,
) -> (Vec<Diagnostic>, Option<Vec<u8>>, usize) {
    let autocorrect_mode = args.autocorrect_mode();

    if autocorrect_mode == crate::cli::AutocorrectMode::Off {
        // Fast path: no autocorrect, run once
        let (diags, _) = lint_source_once(
            source,
            config,
            registry,
            args,
            cop_filters,
            base_configs,
            has_dir_overrides,
            timers,
            autocorrect_mode,
            allowlist,
        );
        return (diags, None, 0);
    }

    // Autocorrect iteration loop
    let original_bytes = source.as_bytes();
    let mut current_bytes = original_bytes.to_vec();
    let path = source.path.clone();
    let mut corrected_diags: Vec<Diagnostic> = Vec::new();

    const MAX_ITERATIONS: usize = 200;

    for _iteration in 0..MAX_ITERATIONS {
        let iter_source = SourceFile::from_vec(path.clone(), current_bytes.clone());
        let (diags, corrections) = lint_source_once(
            &iter_source,
            config,
            registry,
            args,
            cop_filters,
            base_configs,
            has_dir_overrides,
            timers,
            autocorrect_mode,
            allowlist,
        );

        if corrections.is_empty() {
            // Converged — no more corrections. Merge corrected diagnostics from
            // earlier iterations with the remaining diagnostics from this pass.
            let mut all_diags = corrected_diags;
            all_diags.extend(diags);
            let total_corrected = all_diags.iter().filter(|d| d.corrected).count();
            let corrected_bytes = validate_corrected_bytes(original_bytes, current_bytes, &path);
            return (all_diags, corrected_bytes, total_corrected);
        }

        // Collect corrected diagnostics from this iteration
        corrected_diags.extend(diags.into_iter().filter(|d| d.corrected));

        let correction_set = crate::correction::CorrectionSet::from_vec(corrections);
        let new_bytes = correction_set.apply(&current_bytes);

        if new_bytes == current_bytes {
            // Source unchanged despite corrections — bail to avoid infinite loop.
            let total_corrected = corrected_diags.iter().filter(|d| d.corrected).count();
            return (corrected_diags, None, total_corrected);
        }

        current_bytes = new_bytes;
    }

    // Hit max iterations — run one final pass without corrections to get clean diagnostics
    let final_source = SourceFile::from_vec(path.clone(), current_bytes.clone());
    let (diags, _) = lint_source_once(
        &final_source,
        config,
        registry,
        args,
        cop_filters,
        base_configs,
        has_dir_overrides,
        timers,
        crate::cli::AutocorrectMode::Off,
        allowlist,
    );
    let mut all_diags = corrected_diags;
    all_diags.extend(diags);
    let total_corrected = all_diags.iter().filter(|d| d.corrected).count();
    let corrected_bytes = validate_corrected_bytes(original_bytes, current_bytes, &path);
    (all_diags, corrected_bytes, total_corrected)
}

/// Run all enabled cops once on a source file. Returns (diagnostics, corrections).
#[allow(clippy::too_many_arguments)] // internal lint pipeline threading shared state
fn lint_source_once(
    source: &SourceFile,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    args: &Args,
    cop_filters: &CopFilterSet,
    base_configs: &[CopConfig],
    has_dir_overrides: bool,
    timers: Option<&PhaseTimers>,
    autocorrect_mode: crate::cli::AutocorrectMode,
    allowlist: &crate::cop::autocorrect_allowlist::AutocorrectAllowlist,
) -> (Vec<Diagnostic>, Vec<crate::correction::Correction>) {
    // Parse on this thread (ParseResult is !Send)
    let parse_start = std::time::Instant::now();
    let parse_result = crate::parse::parse_source(source.as_bytes());
    if let Some(t) = timers {
        t.parse_ns
            .fetch_add(parse_start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }

    // Skip cops on files with parse errors — the AST from error recovery is
    // unreliable and produces false positives (e.g., Procfile.spec parsed as Ruby).
    // This matches RuboCop's behavior of only reporting Lint/Syntax on such files.
    if parse_result.errors().count() > 0 {
        if let Some(t) = timers {
            t.codemap_ns.fetch_add(0, Ordering::Relaxed);
        }
        return (Vec::new(), Vec::new());
    }

    let codemap_start = std::time::Instant::now();
    let code_map = CodeMap::from_parse_result(source.as_bytes(), &parse_result);
    if let Some(t) = timers {
        t.codemap_ns
            .fetch_add(codemap_start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }

    let mut diagnostics = Vec::new();
    let mut corrections: Vec<crate::correction::Correction> = Vec::new();

    let cop_start = std::time::Instant::now();
    let filter_start = std::time::Instant::now();

    let mut ast_cop_indices: Vec<(usize, Option<usize>)> = Vec::new();
    let mut override_configs: Vec<CopConfig> = Vec::new();

    let override_dir = if has_dir_overrides {
        config.find_override_dir_for_file(&source.path)
    } else {
        None
    };

    let cops = registry.cops();
    let has_only = !args.only.is_empty();

    // Pass 1: Universal cops
    for &i in cop_filters.universal_cop_indices() {
        let cop = &cops[i];
        let name = cop.name();
        if name == REDUNDANT_DISABLE_COP {
            continue;
        }
        if has_only && !args.only.iter().any(|o| o == name) {
            continue;
        }
        if args.except.iter().any(|e| e == name) {
            continue;
        }

        let override_idx = override_dir.and_then(|dir| {
            ResolvedConfig::apply_override_from_dir(&base_configs[i], name, dir).map(|merged| {
                let idx = override_configs.len();
                override_configs.push(merged);
                idx
            })
        });
        let cop_config = match override_idx {
            Some(idx) => &override_configs[idx],
            None => &base_configs[i],
        };

        let should_correct = autocorrect_mode != crate::cli::AutocorrectMode::Off
            && cop.supports_autocorrect()
            && cop_config.should_autocorrect(autocorrect_mode)
            && (autocorrect_mode == crate::cli::AutocorrectMode::All || allowlist.contains(name));

        if should_correct {
            cop.check_lines(source, cop_config, &mut diagnostics, Some(&mut corrections));
            cop.check_source(
                source,
                &parse_result,
                &code_map,
                cop_config,
                &mut diagnostics,
                Some(&mut corrections),
            );
        } else {
            cop.check_lines(source, cop_config, &mut diagnostics, None);
            cop.check_source(
                source,
                &parse_result,
                &code_map,
                cop_config,
                &mut diagnostics,
                None,
            );
        }
        ast_cop_indices.push((i, override_idx));
    }

    // Pass 2: Pattern cops
    for &i in cop_filters.pattern_cop_indices() {
        let cop = &cops[i];
        let name = cop.name();
        if name == REDUNDANT_DISABLE_COP {
            continue;
        }
        if has_only && !args.only.iter().any(|o| o == name) {
            continue;
        }
        if args.except.iter().any(|e| e == name) {
            continue;
        }

        if !cop_filters.is_cop_match(i, &source.path) {
            continue;
        }

        let override_idx = override_dir.and_then(|dir| {
            ResolvedConfig::apply_override_from_dir(&base_configs[i], name, dir).map(|merged| {
                let idx = override_configs.len();
                override_configs.push(merged);
                idx
            })
        });
        let cop_config = match override_idx {
            Some(idx) => &override_configs[idx],
            None => &base_configs[i],
        };

        let should_correct = autocorrect_mode != crate::cli::AutocorrectMode::Off
            && cop.supports_autocorrect()
            && cop_config.should_autocorrect(autocorrect_mode)
            && (autocorrect_mode == crate::cli::AutocorrectMode::All || allowlist.contains(name));

        if should_correct {
            cop.check_lines(source, cop_config, &mut diagnostics, Some(&mut corrections));
            cop.check_source(
                source,
                &parse_result,
                &code_map,
                cop_config,
                &mut diagnostics,
                Some(&mut corrections),
            );
        } else {
            cop.check_lines(source, cop_config, &mut diagnostics, None);
            cop.check_source(
                source,
                &parse_result,
                &code_map,
                cop_config,
                &mut diagnostics,
                None,
            );
        }
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
        if autocorrect_mode != crate::cli::AutocorrectMode::Off {
            walker = walker.with_corrections();
        }
        walker.visit(&parse_result.node());
        let (walker_diags, walker_corrections) = walker.into_results();
        diagnostics.extend(walker_diags);
        if let Some(wc) = walker_corrections {
            if autocorrect_mode == crate::cli::AutocorrectMode::Safe {
                corrections.extend(wc.into_iter().filter(|c| allowlist.contains(c.cop_name)));
            } else {
                corrections.extend(wc);
            }
        }
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

    if !args.ignore_disable_comments && !disabled.is_empty() {
        diagnostics.retain(|d| !disabled.check_and_mark_used(&d.cop_name, d.location.line));
    }

    if !args.ignore_disable_comments
        && disabled.has_directives()
        && args.only.is_empty()
        && !args.except.iter().any(|e| e == REDUNDANT_DISABLE_COP)
    {
        let cop_enabled = registry
            .cops()
            .iter()
            .enumerate()
            .find(|(_, c)| c.name() == REDUNDANT_DISABLE_COP)
            .is_some_and(|(idx, _)| cop_filters.is_cop_match(idx, &source.path));

        if cop_enabled {
            for directive in disabled.unused_directives() {
                if !is_directive_redundant(&directive.cop_name, registry, cop_filters, &source.path)
                {
                    continue;
                }

                let message = format!("Unnecessary disabling of `{}`.", directive.cop_name);
                diagnostics.push(Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location {
                        line: directive.line,
                        column: directive.column,
                    },
                    severity: Severity::Warning,
                    cop_name: REDUNDANT_DISABLE_COP.to_string(),
                    message,
                    corrected: false,
                });
            }
        }
    }
    if let Some(t) = timers {
        t.disable_ns
            .fetch_add(disable_start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }

    (diagnostics, corrections)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // --- validate_corrected_bytes ---

    #[test]
    fn validate_corrected_bytes_rejects_invalid_syntax() {
        let original = b"puts 'hello'";
        let invalid = b"def (".to_vec();
        let result = validate_corrected_bytes(original, invalid, &PathBuf::from("test.rb"));
        assert!(result.is_none(), "should reject invalid Ruby syntax");
    }

    #[test]
    fn validate_corrected_bytes_accepts_valid_syntax() {
        let original = b"puts 'hello'";
        let valid = b"puts 'world'".to_vec();
        let result = validate_corrected_bytes(original, valid, &PathBuf::from("test.rb"));
        assert!(result.is_some(), "should accept valid Ruby syntax");
        assert_eq!(result.unwrap(), b"puts 'world'");
    }

    #[test]
    fn validate_corrected_bytes_returns_none_when_unchanged() {
        let original = b"puts 'hello'";
        let unchanged = b"puts 'hello'".to_vec();
        let result = validate_corrected_bytes(original, unchanged, &PathBuf::from("test.rb"));
        assert!(result.is_none(), "should return None when source unchanged");
    }

    // --- parse_renamed_cops ---

    #[test]
    fn parse_renamed_cops_simple_format() {
        let yaml = "renamed:\n  Old/Cop: New/Cop\n  Another/Old: Another/New\n";
        let map = parse_renamed_cops(yaml);
        assert_eq!(map.get("Old/Cop").unwrap(), "New/Cop");
        assert_eq!(map.get("Another/Old").unwrap(), "Another/New");
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn parse_renamed_cops_extended_format() {
        let yaml = "renamed:\n  Naming/PredicateName:\n    new_name: Naming/PredicatePrefix\n    severity: warning\n";
        let map = parse_renamed_cops(yaml);
        assert_eq!(
            map.get("Naming/PredicateName").unwrap(),
            "Naming/PredicatePrefix"
        );
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn parse_renamed_cops_mixed_formats() {
        let yaml = "\
renamed:
  Simple/Rename: Target/Cop
  Extended/Rename:
    new_name: Target/Extended
    severity: warning
  Another/Simple: Another/Target
";
        let map = parse_renamed_cops(yaml);
        assert_eq!(map.get("Simple/Rename").unwrap(), "Target/Cop");
        assert_eq!(map.get("Extended/Rename").unwrap(), "Target/Extended");
        assert_eq!(map.get("Another/Simple").unwrap(), "Another/Target");
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn parse_renamed_cops_missing_renamed_section() {
        let yaml = "removed:\n  Some/Cop: true\n";
        let map = parse_renamed_cops(yaml);
        assert!(map.is_empty());
    }

    #[test]
    fn parse_renamed_cops_empty_yaml() {
        let map = parse_renamed_cops("");
        assert!(map.is_empty());
    }

    #[test]
    fn parse_renamed_cops_invalid_yaml() {
        let map = parse_renamed_cops("{{invalid yaml::");
        assert!(map.is_empty());
    }

    #[test]
    fn parse_renamed_cops_extended_missing_new_name_key() {
        // Extended format but without the new_name key — should be skipped
        let yaml = "renamed:\n  Bad/Entry:\n    severity: warning\n";
        let map = parse_renamed_cops(yaml);
        assert!(
            map.is_empty(),
            "entry with empty new_name should be skipped"
        );
    }

    #[test]
    fn parse_renamed_cops_real_obsoletion_yml() {
        // Verify the actual vendor file parses correctly
        let map = parse_renamed_cops(include_str!("../vendor/rubocop/config/obsoletion.yml"));
        // Spot-check a few known renames
        assert_eq!(map.get("Layout/Tab").unwrap(), "Layout/IndentationStyle");
        assert_eq!(map.get("Lint/Eval").unwrap(), "Security/Eval");
        assert_eq!(
            map.get("Naming/PredicateName").unwrap(),
            "Naming/PredicatePrefix"
        );
        assert_eq!(
            map.get("Style/PredicateName").unwrap(),
            "Naming/PredicatePrefix"
        );
        // Should have a reasonable number of entries
        assert!(
            map.len() >= 30,
            "expected at least 30 renames, got {}",
            map.len()
        );
    }

    // --- PhaseTimers ---

    #[test]
    fn phase_timers_initializes_to_zero() {
        let t = PhaseTimers::new();
        assert_eq!(t.file_io_ns.load(Ordering::Relaxed), 0);
        assert_eq!(t.parse_ns.load(Ordering::Relaxed), 0);
        assert_eq!(t.codemap_ns.load(Ordering::Relaxed), 0);
        assert_eq!(t.cop_exec_ns.load(Ordering::Relaxed), 0);
        assert_eq!(t.cop_filter_ns.load(Ordering::Relaxed), 0);
        assert_eq!(t.cop_ast_ns.load(Ordering::Relaxed), 0);
        assert_eq!(t.disable_ns.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn phase_timers_accumulates_across_fetches() {
        let t = PhaseTimers::new();
        t.parse_ns.fetch_add(100, Ordering::Relaxed);
        t.parse_ns.fetch_add(200, Ordering::Relaxed);
        assert_eq!(t.parse_ns.load(Ordering::Relaxed), 300);

        t.cop_exec_ns.fetch_add(500, Ordering::Relaxed);
        t.file_io_ns.fetch_add(50, Ordering::Relaxed);
        assert_eq!(t.cop_exec_ns.load(Ordering::Relaxed), 500);
        assert_eq!(t.file_io_ns.load(Ordering::Relaxed), 50);
    }

    // --- RENAMED_COPS static ---

    #[test]
    fn renamed_cops_static_is_populated() {
        // The LazyLock should parse the real obsoletion.yml on first access
        assert!(!RENAMED_COPS.is_empty(), "RENAMED_COPS should not be empty");
        assert!(RENAMED_COPS.contains_key("Layout/Tab"));
    }
}
