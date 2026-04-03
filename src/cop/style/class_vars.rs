use crate::cop::shared::node_type::{
    CALL_NODE, CLASS_VARIABLE_AND_WRITE_NODE, CLASS_VARIABLE_OPERATOR_WRITE_NODE,
    CLASS_VARIABLE_OR_WRITE_NODE, CLASS_VARIABLE_WRITE_NODE, FOR_NODE, MULTI_WRITE_NODE,
};
use crate::cop::{CodeMap, Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/ClassVars: flags assignments to class variables and `class_variable_set`.
///
/// ## Investigation findings (2026-03-27)
///
/// FN root cause (37 corpus misses):
/// - Parallel assignment uses `MultiWriteNode` with `ClassVariableTargetNode`
///   children, so patterns like `@@a, @@b = foo` never reached the direct
///   `ClassVariable*WriteNode` handlers.
/// - The enclosing context varied (method bodies, begin/ensure blocks, modules,
///   and blocks), but the bug was the same Prism node shape in each case.
///
/// Fix:
/// - Added `MULTI_WRITE_NODE` handling and recursive traversal of nested
///   `MultiTargetNode` / `SplatNode` targets so every class-variable target in a
///   parallel assignment is flagged, matching RuboCop's per-target behavior.
///
/// ## Investigation findings (2026-03-30)
///
/// FN root cause (4 corpus misses):
/// - Prism models `for @@var in expr` and `rescue => @@error` as
///   `ClassVariableTargetNode` children of `ForNode` / `RescueNode`, not as
///   `ClassVariableWriteNode`.
/// - Broadly subscribing to `CLASS_VARIABLE_TARGET_NODE` would risk false
///   positives in unrelated target contexts, and Prism's visitor bypasses the
///   normal `check_node` dispatch for `RescueNode`.
///
/// Fix:
/// - Added `FOR_NODE` handling so loop iterator targets are checked with the
///   existing recursive target walker.
/// - Added a narrow `check_source` visitor for rescue references so
///   `rescue => @@error` matches RuboCop without broadening target-node
///   coverage.
pub struct ClassVars;

impl Cop for ClassVars {
    fn name(&self) -> &'static str {
        "Style/ClassVars"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CLASS_VARIABLE_AND_WRITE_NODE,
            CLASS_VARIABLE_OPERATOR_WRITE_NODE,
            CLASS_VARIABLE_OR_WRITE_NODE,
            CLASS_VARIABLE_WRITE_NODE,
            FOR_NODE,
            MULTI_WRITE_NODE,
        ]
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        use ruby_prism::Visit;

        let mut visitor = RescueClassVarVisitor {
            cop: self,
            source,
            diagnostics,
        };
        visitor.visit(&parse_result.node());
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Check class variable write: @@foo = 1
        if let Some(cvasgn) = node.as_class_variable_write_node() {
            self.push_class_var_diagnostic(
                source,
                cvasgn.name().as_slice(),
                cvasgn.name_loc().start_offset(),
                diagnostics,
            );
            return;
        }

        // Check class variable and-write: @@foo &&= 1
        if let Some(cvasgn) = node.as_class_variable_and_write_node() {
            self.push_class_var_diagnostic(
                source,
                cvasgn.name().as_slice(),
                cvasgn.name_loc().start_offset(),
                diagnostics,
            );
            return;
        }

        // Check class variable or-write: @@foo ||= 1
        if let Some(cvasgn) = node.as_class_variable_or_write_node() {
            self.push_class_var_diagnostic(
                source,
                cvasgn.name().as_slice(),
                cvasgn.name_loc().start_offset(),
                diagnostics,
            );
            return;
        }

        // Check class variable operator-write: @@foo += 1
        if let Some(cvasgn) = node.as_class_variable_operator_write_node() {
            self.push_class_var_diagnostic(
                source,
                cvasgn.name().as_slice(),
                cvasgn.name_loc().start_offset(),
                diagnostics,
            );
            return;
        }

        // Check for-loop iterator targets: for @@foo in items
        if let Some(for_node) = node.as_for_node() {
            self.check_target_node(source, &for_node.index(), diagnostics);
            return;
        }

        // Check parallel assignment targets: @@foo, @@bar = value
        if let Some(multi_write) = node.as_multi_write_node() {
            self.check_multi_write_targets(source, multi_write, diagnostics);
            return;
        }

        // Check class_variable_set(:@@foo, value) call
        if let Some(call_node) = node.as_call_node() {
            if call_node.name().as_slice() == b"class_variable_set" {
                if let Some(args) = call_node.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if !arg_list.is_empty() {
                        let first_arg = &arg_list[0];
                        let arg_src = first_arg.location().as_slice();
                        let (line, column) =
                            source.offset_to_line_col(first_arg.location().start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!(
                                "Replace class var {} with a class instance var.",
                                String::from_utf8_lossy(arg_src),
                            ),
                        ));
                    }
                }
            }
        }
    }
}

/// Visitor that finds rescue reference targets (`rescue => @@error`).
///
/// Prism visits rescue clauses through `visit_rescue_node` instead of the normal
/// branch dispatch, so `check_node` never sees `RescueNode`.
struct RescueClassVarVisitor<'a> {
    cop: &'a ClassVars,
    source: &'a SourceFile,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl<'pr> ruby_prism::Visit<'pr> for RescueClassVarVisitor<'_> {
    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        if let Some(reference) = node.reference() {
            self.cop
                .check_target_node(self.source, &reference, self.diagnostics);
        }

        ruby_prism::visit_rescue_node(self, node);
    }
}

impl ClassVars {
    fn push_class_var_diagnostic(
        &self,
        source: &SourceFile,
        name: &[u8],
        start_offset: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let (line, column) = source.offset_to_line_col(start_offset);
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Replace class var {} with a class instance var.",
                String::from_utf8_lossy(name),
            ),
        ));
    }

    fn check_multi_write_targets(
        &self,
        source: &SourceFile,
        multi_write: ruby_prism::MultiWriteNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for target in multi_write.lefts().iter() {
            self.check_target_node(source, &target, diagnostics);
        }
        if let Some(rest) = multi_write.rest() {
            self.check_target_node(source, &rest, diagnostics);
        }
        for target in multi_write.rights().iter() {
            self.check_target_node(source, &target, diagnostics);
        }
    }

    fn check_target_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if let Some(target) = node.as_class_variable_target_node() {
            self.push_class_var_diagnostic(
                source,
                target.name().as_slice(),
                target.location().start_offset(),
                diagnostics,
            );
            return;
        }

        if let Some(splat) = node.as_splat_node() {
            if let Some(expr) = splat.expression() {
                self.check_target_node(source, &expr, diagnostics);
            }
            return;
        }

        if let Some(targets) = node.as_multi_target_node() {
            for target in targets.lefts().iter() {
                self.check_target_node(source, &target, diagnostics);
            }
            if let Some(rest) = targets.rest() {
                self.check_target_node(source, &rest, diagnostics);
            }
            for target in targets.rights().iter() {
                self.check_target_node(source, &target, diagnostics);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ClassVars, "cops/style/class_vars");
}
