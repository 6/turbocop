use crate::cop::shared::method_dispatch_predicates;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for unnecessary `require` statements.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=159, FN=1,224 on the March 10, 2026 run.
///
/// The prior implementation diverged from the pinned RuboCop version by treating
/// `require 'pp'` as redundant and by omitting Ruby 4.0's redundant `pathname`.
/// That produced most observed FPs on scripts that explicitly load `pp`, while
/// leaving most Ruby 4.0 `require 'pathname'` offenses undetected.
///
/// Fix: match RuboCop's feature list exactly for the pinned vendor version:
/// `enumerator`, `thread`, `rational`, `complex`, `ruby2_keywords`, `fiber`,
/// `set`, and `pathname` at Ruby 4.0+.
///
/// Post-fix corpus rerun: actual offenses increased from 895 to 1,648 against
/// a RuboCop expected total of 1,960, eliminating the `pp` FP bucket and
/// recovering most `pathname` misses. Remaining divergence is mostly FN, with
/// one likely extra repo-level offense outside jruby's file-drop-noise repo.
pub struct RedundantRequireStatement;

/// Features that are always redundant (Ruby 2.0+, well below any supported version).
const ALWAYS_REDUNDANT: &[&[u8]] = &[b"enumerator"];

/// Features redundant since Ruby 2.1+.
const RUBY_21_REDUNDANT: &[&[u8]] = &[b"thread"];

/// Features redundant since Ruby 2.2+.
const RUBY_22_REDUNDANT: &[&[u8]] = &[b"rational", b"complex"];

/// Features redundant since Ruby 2.7+.
const RUBY_27_REDUNDANT: &[&[u8]] = &[b"ruby2_keywords"];

/// Features redundant since Ruby 3.1+.
const RUBY_31_REDUNDANT: &[&[u8]] = &[b"fiber"];

/// Features redundant since Ruby 3.2+.
const RUBY_32_REDUNDANT: &[&[u8]] = &[b"set"];

/// Features redundant since Ruby 4.0+.
const RUBY_40_REDUNDANT: &[&[u8]] = &[b"pathname"];

/// Get the target Ruby version from cop config, defaulting to 2.7
/// (matching RuboCop's default when no version is specified).
fn target_ruby_version(config: &CopConfig) -> f64 {
    config
        .options
        .get("TargetRubyVersion")
        .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
        .unwrap_or(2.7)
}

/// Check if a feature is redundant given the target Ruby version.
fn is_redundant_feature(feature: &[u8], ruby_version: f64) -> bool {
    if ALWAYS_REDUNDANT.contains(&feature) {
        return true;
    }
    if ruby_version >= 2.1 && RUBY_21_REDUNDANT.contains(&feature) {
        return true;
    }
    if ruby_version >= 2.2 && RUBY_22_REDUNDANT.contains(&feature) {
        return true;
    }
    if ruby_version >= 2.7 && RUBY_27_REDUNDANT.contains(&feature) {
        return true;
    }
    if ruby_version >= 3.1 && RUBY_31_REDUNDANT.contains(&feature) {
        return true;
    }
    if ruby_version >= 3.2 && RUBY_32_REDUNDANT.contains(&feature) {
        return true;
    }
    if ruby_version >= 4.0 && RUBY_40_REDUNDANT.contains(&feature) {
        return true;
    }
    false
}

/// Visitor that finds redundant require statements and collects diagnostics.
struct RequireVisitor<'a, 'src, 'pr> {
    cop: &'a RedundantRequireStatement,
    source: &'src SourceFile,
    ruby_version: f64,
    diagnostics: Vec<Diagnostic>,
    _phantom: std::marker::PhantomData<&'pr ()>,
}

impl<'a, 'src, 'pr> Visit<'pr> for RequireVisitor<'a, 'src, 'pr> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if method_dispatch_predicates::is_command(node, b"require") {
            if let Some(arguments) = node.arguments() {
                let args = arguments.arguments();
                if args.len() == 1 {
                    if let Some(first_arg) = args.iter().next() {
                        if let Some(string_node) = first_arg.as_string_node() {
                            let feature = string_node.unescaped();
                            if is_redundant_feature(feature, self.ruby_version) {
                                let loc = node.location();
                                let (line, column) =
                                    self.source.offset_to_line_col(loc.start_offset());
                                self.diagnostics.push(self.cop.diagnostic(
                                    self.source,
                                    line,
                                    column,
                                    "Remove unnecessary `require` statement.".to_string(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Continue visiting children (require could be nested in conditionals)
        ruby_prism::visit_call_node(self, node);
    }
}

impl Cop for RedundantRequireStatement {
    fn name(&self) -> &'static str {
        "Lint/RedundantRequireStatement"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let ruby_ver = target_ruby_version(config);

        let mut visitor = RequireVisitor {
            cop: self,
            source,
            ruby_version: ruby_ver,
            diagnostics: Vec::new(),
            _phantom: std::marker::PhantomData,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::cop::CopConfig;
    use crate::testutil::assert_cop_offenses_full_with_config;

    crate::cop_fixture_tests!(
        RedundantRequireStatement,
        "cops/lint/redundant_require_statement"
    );

    #[test]
    fn pathname_is_redundant_on_ruby_40() {
        let config = CopConfig {
            options: HashMap::from([(
                "TargetRubyVersion".into(),
                serde_yml::Value::Number(serde_yml::value::Number::from(4.0_f64)),
            )]),
            ..CopConfig::default()
        };

        let fixture = b"require 'pathname'\n^^^^^^^^^^^^^^^^^ Lint/RedundantRequireStatement: Remove unnecessary `require` statement.\n";
        assert_cop_offenses_full_with_config(&RedundantRequireStatement, fixture, config);
    }
}
