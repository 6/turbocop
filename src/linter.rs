use std::path::Path;

use rayon::prelude::*;
use ruby_prism::Visit;

use crate::cli::Args;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::registry::CopRegistry;
use crate::cop::walker::CopWalker;
use crate::diagnostic::Diagnostic;
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

    // Filter out diagnostics suppressed by inline disable comments
    let disabled = crate::parse::directives::DisabledRanges::from_comments(source, &parse_result);
    if !disabled.is_empty() {
        diagnostics.retain(|d| !disabled.is_disabled(&d.cop_name, d.location.line));
    }

    diagnostics
}
