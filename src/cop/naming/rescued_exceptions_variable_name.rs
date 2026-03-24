use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-11)
///
/// Corpus oracle reported FP=0, FN=2. Both FNs from jruby
/// (`test/jruby/test_local_jump_error.rb:7` and `:15`), `rescue LocalJumpError => lje`.
///
/// **Root cause: CI file discovery issue, NOT a cop logic bug.**
/// The file `test_local_jump_error.rb` is completely skipped by nitrocop in CI —
/// ALL 17 offenses from ALL cops (including Style/FrozenStringLiteralComment,
/// Layout/TrailingWhitespace, etc.) are missing, not just RescuedExceptionsVariableName.
/// The file is valid ASCII Ruby (689 bytes, no BOM, no special encoding).
/// The `ignore` crate walker finds it locally; 156 other test/jruby/ files are
/// processed in CI. No .gitignore pattern matches it. The FN=2 is consistent
/// across 3 corpus oracle runs. `check-cop.py --rerun` locally shows 0 FN.
///
/// Added test coverage for: method-body rescue (no explicit begin),
/// underscore-prefixed variables (`_exc` -> `_e`), multiple rescue clauses in
/// same begin block, writer method rescue targets (`storage.exception`).
pub struct RescuedExceptionsVariableName;

impl Cop for RescuedExceptionsVariableName {
    fn name(&self) -> &'static str {
        "Naming/RescuedExceptionsVariableName"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let preferred = config.get_str("PreferredName", "e");
        let mut visitor = RescuedVarVisitor {
            cop: self,
            source,
            preferred,
            diagnostics: Vec::new(),
            correction_data: Vec::new(),
            rescue_depth: 0,
        };
        visitor.visit(&parse_result.node());

        if let Some(ref mut corr) = corrections {
            for (diag, data) in visitor
                .diagnostics
                .iter()
                .zip(visitor.correction_data.iter())
            {
                let mut d = diag.clone();
                if let Some(data) = data {
                    // Replace the variable declaration
                    corr.push(crate::correction::Correction {
                        start: data.var_start,
                        end: data.var_end,
                        replacement: data.preferred_name.clone(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    // Replace all usages of the old variable name in the rescue body
                    for (usage_start, usage_end) in &data.body_usages {
                        corr.push(crate::correction::Correction {
                            start: *usage_start,
                            end: *usage_end,
                            replacement: data.preferred_name.clone(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                    }
                    d.corrected = true;
                }
                diagnostics.push(d);
            }
        } else {
            diagnostics.extend(visitor.diagnostics);
        }
    }
}

/// Data for generating corrections for a single rescue variable rename.
struct CorrectionData {
    var_start: usize,
    var_end: usize,
    preferred_name: String,
    /// (start, end) byte offsets for all usages of the old variable in the rescue body.
    body_usages: Vec<(usize, usize)>,
}

struct RescuedVarVisitor<'a, 'src> {
    cop: &'a RescuedExceptionsVariableName,
    source: &'src SourceFile,
    preferred: &'a str,
    diagnostics: Vec<Diagnostic>,
    correction_data: Vec<Option<CorrectionData>>,
    rescue_depth: usize,
}

impl<'pr> Visit<'pr> for RescuedVarVisitor<'_, '_> {
    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        // Only check top-level rescues (not nested). RuboCop skips nested
        // rescues because renaming the inner variable could shadow the outer.
        if self.rescue_depth == 0 {
            self.check_rescue(node);
        }

        // Increment depth for body and descendant traversal
        self.rescue_depth += 1;
        ruby_prism::visit_rescue_node(self, node);
        self.rescue_depth -= 1;
    }
}

impl<'a, 'src> RescuedVarVisitor<'a, 'src> {
    fn check_rescue(&mut self, rescue_node: &ruby_prism::RescueNode<'_>) {
        if let Some(reference) = rescue_node.reference() {
            // Extract variable name and location from any target node type
            let var_info = self.extract_variable_info(&reference);
            if let Some((var_str, start_offset)) = var_info {
                // Accept both "e" and "_e" (underscore-prefixed preferred name)
                let underscore_preferred = format!("_{}", self.preferred);
                if var_str != self.preferred && var_str != underscore_preferred {
                    // Determine the preferred name for the diagnostic message
                    let preferred_for_var = if var_str.starts_with('_') {
                        &underscore_preferred
                    } else {
                        self.preferred
                    };
                    // Shadow check always uses the plain preferred name (e.g., "e"),
                    // matching RuboCop's behavior where shadowed_variable_name? checks
                    // lvar reads against the base preferred name regardless of underscore prefix.
                    if self.preferred_name_shadowed(rescue_node, self.preferred) {
                        // Don't flag — renaming would shadow an existing variable
                    } else {
                        let (line, column) = self.source.offset_to_line_col(start_offset);
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            format!(
                                "Use `{}` instead of `{}` for rescued exceptions.",
                                preferred_for_var, var_str,
                            ),
                        ));
                        // Collect correction data for local variables only
                        let is_local = !var_str.starts_with('@')
                            && !var_str.starts_with('$')
                            && !var_str.contains("::")
                            && var_str
                                .chars()
                                .next()
                                .is_some_and(|c| c.is_ascii_lowercase() || c == '_');
                        if is_local {
                            let var_end = start_offset + var_str.len();
                            let mut body_usages = Vec::new();
                            if let Some(body) = rescue_node.statements() {
                                let mut usage_finder = UsageFinder {
                                    var_name: var_str.as_bytes(),
                                    usages: Vec::new(),
                                };
                                usage_finder.visit_statements_node(&body);
                                body_usages = usage_finder.usages;
                            }
                            self.correction_data.push(Some(CorrectionData {
                                var_start: start_offset,
                                var_end,
                                preferred_name: preferred_for_var.to_string(),
                                body_usages,
                            }));
                        } else {
                            self.correction_data.push(None);
                        }
                    }
                }
            }
        }

        // Check subsequent rescue clauses in the same chain (they're at the same depth)
        if let Some(subsequent) = rescue_node.subsequent() {
            self.check_rescue(&subsequent);
        }
    }

    /// Extract the variable name string and start offset from any target node type.
    /// Returns None for unsupported node types (e.g., call nodes like `storage.exception`).
    fn extract_variable_info(&self, reference: &ruby_prism::Node<'_>) -> Option<(String, usize)> {
        if let Some(node) = reference.as_local_variable_target_node() {
            let name = std::str::from_utf8(node.name().as_slice())
                .unwrap_or("")
                .to_string();
            Some((name, node.location().start_offset()))
        } else if let Some(node) = reference.as_instance_variable_target_node() {
            let name = std::str::from_utf8(node.name().as_slice())
                .unwrap_or("")
                .to_string();
            Some((name, node.location().start_offset()))
        } else if let Some(node) = reference.as_class_variable_target_node() {
            let name = std::str::from_utf8(node.name().as_slice())
                .unwrap_or("")
                .to_string();
            Some((name, node.location().start_offset()))
        } else if let Some(node) = reference.as_global_variable_target_node() {
            let name = std::str::from_utf8(node.name().as_slice())
                .unwrap_or("")
                .to_string();
            Some((name, node.location().start_offset()))
        } else if let Some(node) = reference.as_constant_target_node() {
            let name = std::str::from_utf8(node.name().as_slice())
                .unwrap_or("")
                .to_string();
            Some((name, node.location().start_offset()))
        } else if let Some(node) = reference.as_constant_path_target_node() {
            // Qualified constant paths like M::E or ::E2
            let name = std::str::from_utf8(node.location().as_slice())
                .unwrap_or("")
                .to_string();
            Some((name, node.location().start_offset()))
        } else {
            None
        }
    }

    /// Check if the preferred name appears as a local variable READ
    /// anywhere in the rescue body. This matches RuboCop's `shadowed_variable_name?`,
    /// which only checks `:lvar` (read) nodes. Writes (`lvasgn`) do not count as
    /// shadowing — e.g., `e = error` in the body should not prevent flagging.
    fn preferred_name_shadowed(
        &self,
        rescue_node: &ruby_prism::RescueNode<'_>,
        preferred: &str,
    ) -> bool {
        let preferred_bytes = preferred.as_bytes();
        if let Some(body) = rescue_node.statements() {
            let mut checker = ShadowChecker {
                preferred: preferred_bytes,
                found: false,
            };
            checker.visit_statements_node(&body);
            checker.found
        } else {
            false
        }
    }
}

/// Visitor that finds all local variable reads/writes of a given name in the body.
/// Used to collect byte ranges for renaming.
struct UsageFinder<'a> {
    var_name: &'a [u8],
    usages: Vec<(usize, usize)>,
}

impl<'pr> Visit<'pr> for UsageFinder<'_> {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if node.name().as_slice() == self.var_name {
            self.usages
                .push((node.location().start_offset(), node.location().end_offset()));
        }
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        if node.name().as_slice() == self.var_name {
            // Only replace the name part, not the value
            let name_loc = node.name_loc();
            self.usages
                .push((name_loc.start_offset(), name_loc.end_offset()));
        }
        ruby_prism::visit_local_variable_write_node(self, node);
    }
}

/// Visitor that checks if a preferred variable name appears as a local variable
/// READ in the body of a rescue clause. Matches RuboCop's `shadowed_variable_name?`
/// which only checks `:lvar` (read) nodes, not `:lvasgn` (write) or target nodes.
struct ShadowChecker<'a> {
    preferred: &'a [u8],
    found: bool,
}

impl<'pr> Visit<'pr> for ShadowChecker<'_> {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if node.name().as_slice() == self.preferred {
            self.found = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        RescuedExceptionsVariableName,
        "cops/naming/rescued_exceptions_variable_name"
    );
    crate::cop_autocorrect_fixture_tests!(
        RescuedExceptionsVariableName,
        "cops/naming/rescued_exceptions_variable_name"
    );

    #[test]
    fn autocorrect_renames_variable_and_body() {
        let input = b"begin\n  foo\nrescue => ex\n  bar(ex)\nend\n";
        let (diags, corrections) =
            crate::testutil::run_cop_autocorrect(&RescuedExceptionsVariableName, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"begin\n  foo\nrescue => e\n  bar(e)\nend\n");
    }

    #[test]
    fn autocorrect_underscore_prefix() {
        let input = b"begin\n  foo\nrescue => _exc\n  # ignored\nend\n";
        let (diags, corrections) =
            crate::testutil::run_cop_autocorrect(&RescuedExceptionsVariableName, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"begin\n  foo\nrescue => _e\n  # ignored\nend\n");
    }

    #[test]
    fn test_method_body_rescue() {
        // Rescue in method body (no explicit begin)
        let source = b"def test_break\n  proc { break }.call\nrescue LocalJumpError => lje\n  assert_equal :break, lje.reason\nend\n";
        let diags = crate::testutil::run_cop_full(&RescuedExceptionsVariableName, source);
        assert_eq!(diags.len(), 1, "Expected 1 offense, got {:?}", diags);
    }

    #[test]
    fn test_multiple_rescues_same_method() {
        // Multiple begin/rescue blocks in the same method should all be checked.
        // This is the exact pattern from the jruby corpus FN.
        let source = b"require 'test/unit'\n\nclass TestLocalJumpError < Test::Unit::TestCase\n  def test_lje_structure\n    begin\n      break 1\n    rescue LocalJumpError => lje\n      assert_equal(:break, lje.reason)\n      assert_equal(1, lje.exit_value)\n    end\n\n    begin\n      yield 1\n    rescue LocalJumpError => lje\n      assert_equal(:noreason, lje.reason)\n      assert_equal(nil, lje.exit_value)\n    end\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&RescuedExceptionsVariableName, source);
        assert_eq!(diags.len(), 2, "Expected 2 offenses, got {:?}", diags);
    }
}
