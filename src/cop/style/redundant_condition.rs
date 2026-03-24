use crate::cop::node_type::{
    CALL_NODE, CLASS_VARIABLE_WRITE_NODE, CONSTANT_WRITE_NODE, ELSE_NODE,
    GLOBAL_VARIABLE_WRITE_NODE, IF_NODE, INSTANCE_VARIABLE_WRITE_NODE, LOCAL_VARIABLE_WRITE_NODE,
    TRUE_NODE, UNLESS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/RedundantCondition — checks for unnecessary conditional expressions.
///
/// ## Investigation notes (2026-03-23)
///
/// Root causes of FN (84):
/// - Missing `unless` support: `unless b; y; else; b; end` where condition == else_branch
/// - Missing no-else pattern: `if cond; cond; end` → "This condition is not needed."
/// - Missing assignment branches: `if foo; @value = foo; else; @value = 'bar'; end`
///   where both branches assign to the same variable and condition matches RHS of if-branch
/// - Missing method call branches: `if foo; bar(foo); else; bar(quux); end`
///   where both branches call same method on same receiver and condition matches if-branch arg
///
/// Root causes of FP (8):
/// - Missing `use_hash_key_assignment?` check: else branch is `[]=` assignment
/// - Missing `use_if_branch?` check: else branch body contains a ternary/if expression
/// - Predicate+true check was too aggressive for non-call conditions
///
/// Fixed by adding unless handling, no-else pattern, assignment/method branches,
/// and else-branch guards for hash key assignment and ternary-in-else.
pub struct RedundantCondition;

impl RedundantCondition {
    /// Check if two nodes represent the same source code
    fn nodes_equal(
        source: &SourceFile,
        a: &ruby_prism::Node<'_>,
        b: &ruby_prism::Node<'_>,
    ) -> bool {
        let a_bytes = &source.as_bytes()[a.location().start_offset()
            ..a.location().start_offset() + a.location().as_slice().len()];
        let b_bytes = &source.as_bytes()[b.location().start_offset()
            ..b.location().start_offset() + b.location().as_slice().len()];
        a_bytes == b_bytes
    }

    fn make_diagnostic_at(&self, source: &SourceFile, offset: usize, msg: &str) -> Diagnostic {
        let (line, column) = source.offset_to_line_col(offset);
        self.diagnostic(source, line, column, msg.to_string())
    }

    /// Check if an else branch body is an if/ternary node (vendor: use_if_branch?)
    fn else_body_is_if(else_stmts: &ruby_prism::StatementsNode<'_>) -> bool {
        let body: Vec<_> = else_stmts.body().into_iter().collect();
        if body.len() == 1 {
            body[0].as_if_node().is_some()
        } else {
            false
        }
    }

    /// Check if an else branch body is a hash key assignment `[]=` (vendor: use_hash_key_assignment?)
    fn else_body_is_hash_key_assignment(else_stmts: &ruby_prism::StatementsNode<'_>) -> bool {
        let body: Vec<_> = else_stmts.body().into_iter().collect();
        if body.len() == 1 {
            if let Some(call) = body[0].as_call_node() {
                return call.name().as_slice() == b"[]=";
            }
        }
        false
    }

    /// Check if else branch is multi-line (vendor skips multi-line else)
    fn else_is_multiline(source: &SourceFile, else_stmts: &ruby_prism::StatementsNode<'_>) -> bool {
        let body: Vec<_> = else_stmts.body().into_iter().collect();
        if body.len() > 1 {
            return true;
        }
        if body.len() == 1 {
            let loc = body[0].location();
            let (start_line, _) = source.offset_to_line_col(loc.start_offset());
            let end_offset = loc.start_offset() + loc.as_slice().len();
            let (end_line, _) =
                source.offset_to_line_col(if end_offset > 0 { end_offset - 1 } else { 0 });
            return start_line != end_line;
        }
        false
    }

    /// Check if a node is an assignment node (lvasgn, ivasgn, cvasgn, gvasgn, casgn)
    fn is_assignment_node(node: &ruby_prism::Node<'_>) -> bool {
        node.as_local_variable_write_node().is_some()
            || node.as_instance_variable_write_node().is_some()
            || node.as_class_variable_write_node().is_some()
            || node.as_global_variable_write_node().is_some()
            || node.as_constant_write_node().is_some()
    }

    /// Get the assignment target name for comparison
    fn assignment_name(_source: &SourceFile, node: &ruby_prism::Node<'_>) -> Option<String> {
        if let Some(n) = node.as_local_variable_write_node() {
            return Some(String::from_utf8_lossy(n.name().as_slice()).to_string());
        }
        if let Some(n) = node.as_instance_variable_write_node() {
            return Some(String::from_utf8_lossy(n.name().as_slice()).to_string());
        }
        if let Some(n) = node.as_class_variable_write_node() {
            return Some(String::from_utf8_lossy(n.name().as_slice()).to_string());
        }
        if let Some(n) = node.as_global_variable_write_node() {
            return Some(String::from_utf8_lossy(n.name().as_slice()).to_string());
        }
        if let Some(n) = node.as_constant_write_node() {
            return Some(String::from_utf8_lossy(n.name().as_slice()).to_string());
        }
        None
    }

    /// Get the assignment value (RHS) for comparison
    fn assignment_value<'a>(node: &'a ruby_prism::Node<'a>) -> Option<ruby_prism::Node<'a>> {
        if let Some(n) = node.as_local_variable_write_node() {
            return Some(n.value());
        }
        if let Some(n) = node.as_instance_variable_write_node() {
            return Some(n.value());
        }
        if let Some(n) = node.as_class_variable_write_node() {
            return Some(n.value());
        }
        if let Some(n) = node.as_global_variable_write_node() {
            return Some(n.value());
        }
        if let Some(n) = node.as_constant_write_node() {
            return Some(n.value());
        }
        None
    }

    /// Check branches_have_assignment pattern: both branches assign to same variable,
    /// and condition matches the if-branch's RHS value
    fn check_assignment_branches(
        source: &SourceFile,
        condition: &ruby_prism::Node<'_>,
        true_node: &ruby_prism::Node<'_>,
        else_node: &ruby_prism::Node<'_>,
    ) -> bool {
        if !Self::is_assignment_node(true_node) || !Self::is_assignment_node(else_node) {
            return false;
        }
        let true_name = Self::assignment_name(source, true_node);
        let else_name = Self::assignment_name(source, else_node);
        if true_name.is_none() || true_name != else_name {
            return false;
        }
        // condition must match the RHS of the if-branch assignment
        if let Some(value) = Self::assignment_value(true_node) {
            return Self::nodes_equal(source, condition, &value);
        }
        false
    }

    /// Check branches_have_method pattern: both branches call same single-arg method
    /// on same receiver, and condition matches the if-branch's argument
    fn check_method_branches(
        source: &SourceFile,
        condition: &ruby_prism::Node<'_>,
        true_node: &ruby_prism::Node<'_>,
        else_node: &ruby_prism::Node<'_>,
    ) -> bool {
        let true_call = match true_node.as_call_node() {
            Some(c) => c,
            None => return false,
        };
        let else_call = match else_node.as_call_node() {
            Some(c) => c,
            None => return false,
        };

        // Both must have exactly one argument
        let true_args: Vec<_> = true_call
            .arguments()
            .map_or(vec![], |a| a.arguments().into_iter().collect());
        let else_args: Vec<_> = else_call
            .arguments()
            .map_or(vec![], |a| a.arguments().into_iter().collect());
        if true_args.len() != 1 || else_args.len() != 1 {
            return false;
        }

        // Skip hash key access [] (vendor: use_hash_key_access?)
        if true_call.name().as_slice() == b"[]" {
            return false;
        }

        // Must be same method name
        if true_call.name().as_slice() != else_call.name().as_slice() {
            return false;
        }

        // Must have same receiver
        match (true_call.receiver(), else_call.receiver()) {
            (Some(r1), Some(r2)) => {
                if !Self::nodes_equal(source, &r1, &r2) {
                    return false;
                }
            }
            (None, None) => {}
            _ => return false,
        }

        // Check if the else arg has operator-like types (splat, block_pass, etc.)
        let else_arg = &else_args[0];
        if else_arg.as_splat_node().is_some() || else_arg.as_block_argument_node().is_some() {
            return false;
        }

        // condition must match the if-branch's first argument
        Self::nodes_equal(source, condition, &true_args[0])
    }

    /// Handle an unless node: condition == else_branch means offense
    fn check_unless(
        &self,
        source: &SourceFile,
        condition: &ruby_prism::Node<'_>,
        body_stmts: Option<ruby_prism::StatementsNode<'_>>,
        else_stmts: Option<ruby_prism::StatementsNode<'_>>,
        kw_offset: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let else_stmts = match else_stmts {
            Some(s) => s,
            None => return,
        };
        let unless_body = match body_stmts {
            Some(s) => s,
            None => return,
        };
        let unless_body_nodes: Vec<_> = unless_body.body().into_iter().collect();
        if unless_body_nodes.len() != 1 {
            return;
        }
        let else_body: Vec<_> = else_stmts.body().into_iter().collect();
        if else_body.len() != 1 {
            return;
        }
        if Self::nodes_equal(source, condition, &else_body[0]) {
            diagnostics.push(self.make_diagnostic_at(
                source,
                kw_offset,
                "Use double pipes `||` instead.",
            ));
        }
    }

    /// Handle an if node (including ternary): checks all offense patterns
    #[allow(clippy::too_many_arguments)]
    fn check_if(
        &self,
        source: &SourceFile,
        condition: &ruby_prism::Node<'_>,
        true_stmts: Option<ruby_prism::StatementsNode<'_>>,
        else_stmts: Option<ruby_prism::StatementsNode<'_>>,
        is_ternary: bool,
        kw_offset: usize,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Get true branch
        let true_stmts = match true_stmts {
            Some(s) => s,
            None => return,
        };
        let true_body: Vec<_> = true_stmts.body().into_iter().collect();
        if true_body.len() != 1 {
            return;
        }
        let true_value = &true_body[0];

        // No-else pattern: `if cond; cond; end` → "This condition is not needed."
        if else_stmts.is_none() {
            if Self::nodes_equal(source, condition, true_value) {
                diagnostics.push(self.make_diagnostic_at(
                    source,
                    kw_offset,
                    "This condition is not needed.",
                ));
            }
            return;
        }

        let else_stmts_unwrapped = else_stmts.unwrap();

        // Else branch guards (not for ternary)
        if !is_ternary {
            if Self::else_is_multiline(source, &else_stmts_unwrapped) {
                return;
            }
            if Self::else_body_is_if(&else_stmts_unwrapped) {
                return;
            }
            if Self::else_body_is_hash_key_assignment(&else_stmts_unwrapped) {
                return;
            }
        }

        // Pattern 1: condition == true_branch
        if Self::nodes_equal(source, condition, true_value) {
            diagnostics.push(self.make_diagnostic_at(
                source,
                kw_offset,
                "Use double pipes `||` instead.",
            ));
            return;
        }

        // Pattern 2: predicate+true — condition is predicate call, true branch is `true`
        if true_value.as_true_node().is_some() {
            if let Some(call) = condition.as_call_node() {
                let method_name = call.name().as_slice();
                if method_name.ends_with(b"?") {
                    let allowed = config
                        .get_string_array("AllowedMethods")
                        .unwrap_or_default();
                    let method_str = std::str::from_utf8(method_name).unwrap_or("");
                    let is_allowed = allowed.iter().any(|m| m == method_str);
                    if !is_allowed {
                        let else_body: Vec<_> = else_stmts_unwrapped.body().into_iter().collect();
                        let else_is_true =
                            else_body.len() == 1 && else_body[0].as_true_node().is_some();
                        if !else_body.is_empty() && !else_is_true {
                            diagnostics.push(self.make_diagnostic_at(
                                source,
                                kw_offset,
                                "Use double pipes `||` instead.",
                            ));
                            return;
                        }
                    }
                }
            }
        }

        // Pattern 3: assignment branches
        let else_body: Vec<_> = else_stmts_unwrapped.body().into_iter().collect();
        if else_body.len() == 1 {
            if Self::check_assignment_branches(source, condition, true_value, &else_body[0]) {
                diagnostics.push(self.make_diagnostic_at(
                    source,
                    kw_offset,
                    "Use double pipes `||` instead.",
                ));
                return;
            }

            // Pattern 4: method call branches
            if Self::check_method_branches(source, condition, true_value, &else_body[0]) {
                diagnostics.push(self.make_diagnostic_at(
                    source,
                    kw_offset,
                    "Use double pipes `||` instead.",
                ));
            }
        }
    }
}

impl Cop for RedundantCondition {
    fn name(&self) -> &'static str {
        "Style/RedundantCondition"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CLASS_VARIABLE_WRITE_NODE,
            CONSTANT_WRITE_NODE,
            ELSE_NODE,
            GLOBAL_VARIABLE_WRITE_NODE,
            IF_NODE,
            INSTANCE_VARIABLE_WRITE_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
            TRUE_NODE,
            UNLESS_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Handle IfNode (includes ternary)
        if let Some(if_node) = node.as_if_node() {
            // Skip modifier if (has keyword but no end keyword, and is not ternary)
            if let Some(kw_loc) = if_node.if_keyword_loc() {
                let kw_text = kw_loc.as_slice();
                // Skip unless and elsif
                if kw_text != b"if" {
                    return;
                }
                // Modifier if: has `if` keyword but no `end` keyword
                if if_node.end_keyword_loc().is_none() {
                    return;
                }
            }

            // Skip elsif (subsequent is another IfNode)
            if let Some(subsequent) = if_node.subsequent() {
                if subsequent.as_if_node().is_some() {
                    return;
                }
            }

            // Determine if this is a ternary
            let is_ternary = if_node.if_keyword_loc().is_none();

            let kw_offset = if let Some(kw_loc) = if_node.if_keyword_loc() {
                kw_loc.start_offset()
            } else {
                if_node.location().start_offset()
            };

            // Get else statements
            let else_stmts = if let Some(subsequent) = if_node.subsequent() {
                if let Some(else_node) = subsequent.as_else_node() {
                    else_node.statements()
                } else {
                    None
                }
            } else {
                None
            };

            self.check_if(
                source,
                &if_node.predicate(),
                if_node.statements(),
                else_stmts,
                is_ternary,
                kw_offset,
                config,
                diagnostics,
            );
            return;
        }

        // Handle UnlessNode
        if let Some(unless_node) = node.as_unless_node() {
            // Skip modifier unless (has keyword but no end keyword)
            if unless_node.end_keyword_loc().is_none() {
                return;
            }

            let kw_offset = unless_node.keyword_loc().start_offset();

            // Get else statements
            let else_stmts = unless_node.else_clause().and_then(|e| e.statements());

            self.check_unless(
                source,
                &unless_node.predicate(),
                unless_node.statements(),
                else_stmts,
                kw_offset,
                diagnostics,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantCondition, "cops/style/redundant_condition");
}
