use std::path::Path;

use rayon::prelude::*;
use ruby_prism::Visit;

use crate::cli::Args;
use crate::config::ResolvedConfig;
use crate::cop::registry::CopRegistry;
use crate::cop::walker::CopWalker;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct LintResult {
    pub diagnostics: Vec<Diagnostic>,
    pub file_count: usize,
}

pub fn run_linter(
    files: &[std::path::PathBuf],
    config: &ResolvedConfig,
    registry: &CopRegistry,
    args: &Args,
) -> LintResult {
    let diagnostics: Vec<Diagnostic> = files
        .par_iter()
        .flat_map(|path| lint_file(path, config, registry, args))
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
) -> Vec<Diagnostic> {
    let source = match SourceFile::from_path(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e:#}");
            return Vec::new();
        }
    };

    // Parse on this thread (ParseResult is !Send)
    let parse_result = crate::parse::parse_source(source.as_bytes());
    let code_map = CodeMap::from_parse_result(source.as_bytes(), &parse_result);

    let mut diagnostics = Vec::new();

    for cop in registry.cops() {
        let name = cop.name();

        // Filter by --only / --except
        if !args.only.is_empty() && !args.only.iter().any(|o| o == name) {
            continue;
        }
        if args.except.iter().any(|e| e == name) {
            continue;
        }

        // Check config (with cop's default include/exclude patterns)
        if !config.is_cop_enabled(name, path, cop.default_include(), cop.default_exclude()) {
            continue;
        }

        let cop_config = config.cop_config(name);

        // Line-based checks
        diagnostics.extend(cop.check_lines(&source, &cop_config));

        // Source-based checks (raw byte scanning with CodeMap)
        diagnostics.extend(cop.check_source(&source, &parse_result, &code_map, &cop_config));

        // AST-based checks: walk every node
        let mut walker = CopWalker {
            cop: &**cop,
            source: &source,
            parse_result: &parse_result,
            cop_config: &cop_config,
            diagnostics: Vec::new(),
        };
        walker.visit(&parse_result.node());
        diagnostics.extend(walker.diagnostics);
    }

    diagnostics
}
