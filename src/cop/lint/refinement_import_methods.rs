use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks if `include` or `prepend` is called in a `refine` block.
/// These methods are deprecated and should be replaced with `import_methods`.
///
/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=1, FN=0.
///
/// FP=1: the remaining mismatch is still `ruby-next`'s excluded
/// `spec/core/refinement/fixtures/import.rb`, so the corpus divergence is
/// config/path-filter noise rather than active cop logic. Earlier fixes already
/// aligned the cop's pending default-disabled state and restricted matches to
/// direct `include`/`prepend` children of a `refine` block.
///
/// Additional correctness fix: RuboCop declares `minimum_target_ruby_version 3.1`,
/// so `include`/`prepend` inside `refine` must be ignored when the project
/// targets Ruby 3.0 or below.
/// FN=0: no missing detections were reported for this cop in the corpus run.
///
/// ## Corpus investigation (2026-03-15)
///
/// The remaining FP=1 was not path-filter noise. RuboCop 1.84.2 only flags
/// `include`/`prepend` when it is the sole body node of the `refine` block.
/// If the refine body is a multi-statement `begin`, e.g. `include B` followed
/// by `def foo`, RuboCop does not descend and emits no offense.
///
/// Fix:
/// - Match RuboCop's current behavior by only checking a direct call body or a
///   `StatementsNode` with exactly one statement.
pub struct RefinementImportMethods;

impl Cop for RefinementImportMethods {
    fn name(&self) -> &'static str {
        "Lint/RefinementImportMethods"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_enabled(&self) -> bool {
        false
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
        // RuboCop: minimum_target_ruby_version 3.1
        let ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(2.7);
        if ruby_version < 3.1 {
            return;
        }

        let mut visitor = RefineVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RefineVisitor<'a, 'src> {
    cop: &'a RefinementImportMethods,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for RefineVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        // Check if this is a `refine` call with a block
        if method_name == b"refine" && node.receiver().is_none() {
            if let Some(block) = node.block() {
                // Check direct children of the block body for include/prepend
                if let Some(block_node) = block.as_block_node() {
                    if let Some(body) = block_node.body() {
                        self.check_refine_body(&body);
                    }
                }
            }
        }

        // Continue visiting children for nested refine blocks
        ruby_prism::visit_call_node(self, node);
    }
}

impl RefineVisitor<'_, '_> {
    fn check_refine_body(&mut self, body: &ruby_prism::Node<'_>) {
        let maybe_call = if let Some(call) = body.as_call_node() {
            Some(call)
        } else if let Some(stmts) = body.as_statements_node() {
            let mut iter = stmts.body().iter();
            let first = iter.next();
            if iter.next().is_none() {
                first.and_then(|stmt| stmt.as_call_node())
            } else {
                None
            }
        } else {
            None
        };

        if let Some(call) = maybe_call {
            let name = call.name().as_slice();
            if (name == b"include" || name == b"prepend") && call.receiver().is_none() {
                let msg_loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = self.source.offset_to_line_col(msg_loc.start_offset());
                let method_str = if name == b"include" {
                    "include"
                } else {
                    "prepend"
                };
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!(
                        "Use `import_methods` instead of `{}` because it is deprecated in Ruby 3.1.",
                        method_str
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;

    fn ruby31_config() -> CopConfig {
        let mut config = CopConfig::default();
        config.options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::Value::Number(serde_yml::Number::from(3.1)),
        );
        config
    }

    #[test]
    fn offense_with_ruby31() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &RefinementImportMethods,
            include_bytes!(
                "../../../tests/fixtures/cops/lint/refinement_import_methods/offense.rb"
            ),
            ruby31_config(),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &RefinementImportMethods,
            include_bytes!(
                "../../../tests/fixtures/cops/lint/refinement_import_methods/no_offense.rb"
            ),
            ruby31_config(),
        );
    }

    #[test]
    fn no_offense_below_ruby31() {
        let mut config = CopConfig::default();
        config.options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::Value::Number(serde_yml::Number::from(3.0)),
        );
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &RefinementImportMethods,
            b"refine Foo do\n  include Bar\nend\n",
            config,
        );
    }
}
