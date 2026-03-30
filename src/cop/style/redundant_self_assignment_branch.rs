use crate::cop::node_type::{IF_NODE, LOCAL_VARIABLE_WRITE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/RedundantSelfAssignmentBranch
///
/// Checks for places where conditional branch makes redundant self-assignment.
///
/// RuboCop only detects local variable assignments (not instance/class/global
/// vars) because replacing those with nil could change state across methods.
///
/// ## Conditions for offense
/// - LHS is a local variable assignment (`LocalVariableWriteNode`)
/// - RHS is an if/else expression or ternary (`IfNode`)
/// - No `elsif` branch present
/// - Neither branch has multiple statements
/// - One branch is a bare read of the same local variable
///
/// ## Historical FN causes
/// - Prism represents ternaries as `IfNode`s without `if_keyword_loc()`. This
///   cop used that as a blanket "skip ternary" check, which missed self-
///   assignment ternaries like `foo = condition ? foo : bar` and
///   `foo = condition ? bar(arg) : foo`.
/// - RuboCop still skips ternaries when either branch is explicitly wrapped in
///   parentheses. Prism exposes those as `ParenthesesNode`s, so we must keep
///   that narrow exemption to avoid FP regressions.
pub struct RedundantSelfAssignmentBranch;

impl Cop for RedundantSelfAssignmentBranch {
    fn name(&self) -> &'static str {
        "Style/RedundantSelfAssignmentBranch"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, LOCAL_VARIABLE_WRITE_NODE]
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
        let write = match node.as_local_variable_write_node() {
            Some(w) => w,
            None => return,
        };

        let var_name = write.name().as_slice();
        let value = write.value();

        // Only handle if/else expressions. Prism also uses IfNode for ternaries.
        let if_node = match value.as_if_node() {
            Some(n) => n,
            None => return,
        };

        if if_node.if_keyword_loc().is_none() && ternary_has_parenthesized_branch(&if_node) {
            return;
        }

        self.check_if_node(source, &if_node, var_name, diagnostics);
    }
}

impl RedundantSelfAssignmentBranch {
    fn check_if_node(
        &self,
        source: &SourceFile,
        if_node: &ruby_prism::IfNode<'_>,
        var_name: &[u8],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Get the if-branch statements
        let if_stmts = if_node.statements();

        // Get the else/subsequent branch
        let subsequent = match if_node.subsequent() {
            Some(s) => s,
            None => return, // no else branch — skip
        };

        // If subsequent is another IfNode (elsif), skip entirely
        if subsequent.as_if_node().is_some() {
            return;
        }

        // Must be an ElseNode
        let else_node = match subsequent.as_else_node() {
            Some(e) => e,
            None => return,
        };

        let else_stmts = else_node.statements();

        // Check for multiple statements in either branch
        if has_multiple_statements(&if_stmts) || has_multiple_statements(&else_stmts) {
            return;
        }

        // Check if the if-branch is a self-assignment
        if is_single_var_read(&if_stmts, var_name) {
            if let Some(stmts) = &if_stmts {
                let body: Vec<_> = stmts.body().iter().collect();
                if let Some(read_node) = body.first() {
                    let loc = read_node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Remove the self-assignment branch.".to_string(),
                    ));
                }
            }
            return;
        }

        // Check if the else-branch is a self-assignment
        if is_single_var_read(&else_stmts, var_name) {
            if let Some(stmts) = &else_stmts {
                let body: Vec<_> = stmts.body().iter().collect();
                if let Some(read_node) = body.first() {
                    let loc = read_node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Remove the self-assignment branch.".to_string(),
                    ));
                }
            }
        }
    }
}

fn has_multiple_statements(stmts: &Option<ruby_prism::StatementsNode<'_>>) -> bool {
    if let Some(s) = stmts {
        let body: Vec<_> = s.body().iter().collect();
        body.len() > 1
    } else {
        false
    }
}

fn is_single_var_read(stmts: &Option<ruby_prism::StatementsNode<'_>>, var_name: &[u8]) -> bool {
    if let Some(s) = stmts {
        let body: Vec<_> = s.body().iter().collect();
        body.len() == 1 && is_same_var(&body[0], var_name)
    } else {
        false
    }
}

fn is_same_var(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(lv) = node.as_local_variable_read_node() {
        return lv.name().as_slice() == var_name;
    }
    false
}

fn ternary_has_parenthesized_branch(if_node: &ruby_prism::IfNode<'_>) -> bool {
    branch_is_parenthesized(&if_node.statements())
        || if_node
            .subsequent()
            .and_then(|subsequent| subsequent.as_else_node())
            .is_some_and(|else_node| branch_is_parenthesized(&else_node.statements()))
}

fn branch_is_parenthesized(stmts: &Option<ruby_prism::StatementsNode<'_>>) -> bool {
    let Some(stmts) = stmts else {
        return false;
    };

    let body: Vec<_> = stmts.body().iter().collect();
    body.len() == 1 && body[0].as_parentheses_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantSelfAssignmentBranch,
        "cops/style/redundant_self_assignment_branch"
    );
}
