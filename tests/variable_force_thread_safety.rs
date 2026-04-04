//! Stress test for VariableForce cop thread safety.
//!
//! Verifies that all VF consumer cops produce deterministic results when
//! a single shared cop instance processes files in parallel via rayon.
//! A TOCTOU race (e.g., Mutex per-file state overwritten by another thread)
//! would cause non-deterministic diagnostic counts, caught here as flaky
//! assertion failures.

use std::fs;

use rayon::prelude::*;

use nitrocop::cop::registry::CopRegistry;
use nitrocop::cop::{Cop, CopConfig};
use nitrocop::diagnostic::Diagnostic;
use nitrocop::parse::codemap::CodeMap;
use nitrocop::parse::source::SourceFile;

/// Run check_source + VariableForce engine on a cop (the two phases where
/// TOCTOU races manifest). Skips check_lines and check_node since those
/// don't interact with VF state.
fn run_cop_vf(cop: &dyn Cop, source_bytes: &[u8]) -> Vec<Diagnostic> {
    let config = CopConfig::default();
    let source = SourceFile::from_vec("test.rb".into(), source_bytes.to_vec());
    let parse_result = nitrocop::parse::parse_source(source.as_bytes());
    let code_map = CodeMap::from_parse_result(source.as_bytes(), &parse_result);

    let mut diagnostics = Vec::new();

    cop.check_source(
        &source,
        &parse_result,
        &code_map,
        &config,
        &mut diagnostics,
        None,
    );

    if let Some(consumer) = cop.as_variable_force_consumer() {
        let registered = nitrocop::cop::variable_force::engine::RegisteredConsumer {
            consumer,
            config: &config,
        };
        let consumers = [registered];
        let mut engine = nitrocop::cop::variable_force::engine::Engine::new(&source, &consumers);
        engine.run(&parse_result);
        diagnostics.extend(engine.into_diagnostics());
    }

    diagnostics
}

/// Convert CamelCase cop name to snake_case.
fn camel_to_snake(name: &str) -> String {
    let mut result = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

/// Map a cop name like "Style/InfiniteLoop" to its fixture path.
fn cop_fixture_path(name: &str) -> Option<String> {
    let (dept, cop) = name.split_once('/')?;
    Some(format!(
        "tests/fixtures/cops/{}/{}/offense.rb",
        dept.to_lowercase(),
        camel_to_snake(cop),
    ))
}

#[test]
fn variable_force_cops_parallel_determinism() {
    let registry = CopRegistry::default_registry();
    let mut tested = 0;

    for cop in registry.cops() {
        if cop.as_variable_force_consumer().is_none() {
            continue;
        }

        let fixture_path = match cop_fixture_path(cop.name()) {
            Some(p) => p,
            None => continue,
        };
        let fixture = match fs::read(&fixture_path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };

        // Establish single-threaded baseline
        let expected = run_cop_vf(cop.as_ref(), &fixture).len();
        if expected == 0 {
            continue;
        }

        // Run 50 parallel copies across 10 rounds to catch races
        for round in 0..10 {
            let results: Vec<usize> = (0..50)
                .into_par_iter()
                .map(|_| run_cop_vf(cop.as_ref(), &fixture).len())
                .collect();

            for (i, &count) in results.iter().enumerate() {
                assert_eq!(
                    count,
                    expected,
                    "TOCTOU race detected in {}! round {round} iter {i}: \
                     expected {expected} diagnostics, got {count}. \
                     The cop likely uses Mutex per-file state instead of thread_local!.",
                    cop.name(),
                );
            }
        }

        tested += 1;
    }

    // Sanity check: we should have tested at least a few cops
    assert!(
        tested >= 5,
        "Expected to test at least 5 VF consumer cops, only found {tested}"
    );
}
