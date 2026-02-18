use std::path::Path;

use rayon::prelude::*;
use ruby_prism::Visit;

use crate::cli::Args;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::registry::CopRegistry;
use crate::cop::walker::CopWalker;
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

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
    let diagnostics = lint_source_inner(source, config, registry, args, &cop_filters);
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
    // Build cop filters once before the parallel loop
    let cop_filters = config.build_cop_filters(registry);

    let diagnostics: Vec<Diagnostic> = files
        .par_iter()
        .flat_map(|path| lint_file(path, config, registry, args, &cop_filters))
        .collect();

    let mut sorted = diagnostics;
    sorted.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));

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
) -> Vec<Diagnostic> {
    // Check global excludes once per file
    if cop_filters.is_globally_excluded(path) {
        return Vec::new();
    }

    let source = match SourceFile::from_path(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e:#}");
            return Vec::new();
        }
    };

    lint_source_inner(&source, config, registry, args, cop_filters)
}

/// Name of the redundant cop disable directive cop.
const REDUNDANT_DISABLE_COP: &str = "Lint/RedundantCopDisableDirective";

/// Determine if a disable directive should be flagged as redundant.
///
/// Returns true if the directive IS redundant (should be reported), false if
/// we should skip it.
///
/// The logic is ultra-conservative to avoid false positives:
///   - "all" or department-only directives: never flag (too broad to check)
///   - Cop NOT in the registry: never flag (could be a custom cop from a
///     plugin gem, a project-local cop, or a renamed/removed cop)
///   - Known cop that is explicitly disabled (Enabled: false): flag as redundant
///   - Known cop that is enabled: never flag, even if Include/Exclude would
///     exclude it from this file, because rblint's path resolution may differ
///     from RuboCop's (e.g., per-directory .rubocop.yml overrides)
fn is_directive_redundant(
    cop_name: &str,
    registry: &CopRegistry,
    cop_filters: &CopFilterSet,
    _path: &Path, // reserved for future Include/Exclude-based detection
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
        // Only flag when the cop is explicitly disabled (Enabled: false).
        // Do NOT flag when the cop is enabled but fails Include/Exclude matching,
        // because rblint's Include/Exclude resolution may differ from RuboCop's
        // (e.g., per-directory .rubocop.yml overrides, plugin gem path patterns).
        // Also don't flag running cops — rblint might just have a detection gap.
        let filter = cop_filters.cop_filter(idx);
        if !filter.is_enabled() {
            // Cop is explicitly disabled — the disable directive is redundant.
            true
        } else {
            // Cop is enabled (even if Include/Exclude might exclude it) — don't flag.
            false
        }
    } else {
        // Cop is NOT in the registry — never flag.
        // It could be a custom cop from a plugin gem (e.g., InternalAffairs/*,
        // Discourse/*), a project-local custom cop (e.g., Style/MiddleDot),
        // or a renamed/removed cop. We can't distinguish these cases reliably,
        // so we conservatively skip all of them to avoid false positives.
        false
    }
}

fn lint_source_inner(
    source: &SourceFile,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    args: &Args,
    cop_filters: &CopFilterSet,
) -> Vec<Diagnostic> {
    // Parse on this thread (ParseResult is !Send)
    let parse_result = crate::parse::parse_source(source.as_bytes());
    let code_map = CodeMap::from_parse_result(source.as_bytes(), &parse_result);

    let mut diagnostics = Vec::new();

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

        let cop_config = config.cop_config_for_file(name, &source.path);

        // Line-based checks
        diagnostics.extend(cop.check_lines(source, &cop_config));

        // Source-based checks (raw byte scanning with CodeMap)
        diagnostics.extend(cop.check_source(source, &parse_result, &code_map, &cop_config));

        // AST-based checks: walk every node
        let mut walker = CopWalker {
            cop: &**cop,
            source,
            parse_result: &parse_result,
            cop_config: &cop_config,
            diagnostics: Vec::new(),
        };
        walker.visit(&parse_result.node());
        diagnostics.extend(walker.diagnostics);
    }

    // Filter out diagnostics suppressed by inline disable comments,
    // and detect redundant disable directives.
    let mut disabled =
        crate::parse::directives::DisabledRanges::from_comments(source, &parse_result);

    if !disabled.is_empty() {
        // Use check_and_mark_used to both filter diagnostics and track which
        // directives actually suppressed something.
        diagnostics.retain(|d| !disabled.check_and_mark_used(&d.cop_name, d.location.line));
    }

    // Generate Lint/RedundantCopDisableDirective offenses for unused directives.
    //
    // Only flag a directive as redundant when:
    //   - The referenced cop is in the registry AND explicitly disabled (Enabled: false)
    //
    // We intentionally skip:
    //   - Unknown cops (could be custom/plugin/renamed cops)
    //   - Enabled cops (rblint may have detection gaps vs. RuboCop)
    //   - Cops excluded by Include/Exclude (rblint's path resolution may differ)
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

    diagnostics
}
