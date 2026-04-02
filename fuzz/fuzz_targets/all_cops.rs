#![no_main]
use libfuzzer_sys::fuzz_target;

use std::path::PathBuf;
use std::sync::LazyLock;

use nitrocop::cop::registry::CopRegistry;
use nitrocop::cop::walker::BatchedCopWalker;
use nitrocop::cop::{Cop, CopConfig};
use nitrocop::diagnostic::Diagnostic;
use nitrocop::parse::codemap::CodeMap;
use nitrocop::parse::source::SourceFile;
use ruby_prism::Visit;

static REGISTRY: LazyLock<CopRegistry> = LazyLock::new(CopRegistry::default_registry);
static DEFAULT_CONFIG: LazyLock<CopConfig> = LazyLock::new(CopConfig::default);

// Run every registered cop on arbitrary input through all three phases:
// check_lines, check_source, and check_node (AST walk). This catches panics
// from unexpected AST shapes, string handling, and source-level scanning in
// any cop — not just the hand-picked ones.
fuzz_target!(|data: &str| {
    let source = SourceFile::from_string(PathBuf::from("fuzz_input.rb"), data.to_string());
    let parse_result = nitrocop::parse::parse_source(source.as_bytes());
    let code_map = CodeMap::from_parse_result(source.as_bytes(), &parse_result);

    let cops = REGISTRY.cops();
    let mut diagnostics = Vec::<Diagnostic>::new();

    let mut ast_cops: Vec<(&dyn Cop, &CopConfig)> = Vec::new();

    for cop in cops.iter() {
        // Phase 1: line-based checks
        cop.check_lines(&source, &DEFAULT_CONFIG, &mut diagnostics, None);

        // Phase 2: source-based checks
        cop.check_source(
            &source,
            &parse_result,
            &code_map,
            &DEFAULT_CONFIG,
            &mut diagnostics,
            None,
        );

        ast_cops.push((&**cop, &DEFAULT_CONFIG));
    }

    // Phase 3: AST walk (check_node for all cops in one pass)
    let mut walker = BatchedCopWalker::new(ast_cops, &source, &parse_result);
    walker.visit(&parse_result.node());
    let (walker_diags, _) = walker.into_results();
    diagnostics.extend(walker_diags);
});
