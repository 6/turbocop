use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantSelfAssignmentBranch;

impl Cop for RedundantSelfAssignmentBranch {
    fn name(&self) -> &'static str {
        "Style/RedundantSelfAssignmentBranch"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Look for: x = if cond; expr; else; x; end
        // or: x = cond ? expr : x
        let write = match node.as_local_variable_write_node() {
            Some(w) => w,
            None => return Vec::new(),
        };

        let var_name = write.name().as_slice();
        let value = write.value();

        // Check if value is an if/ternary
        if let Some(if_node) = value.as_if_node() {
            return self.check_if_branch(source, node, &if_node, var_name);
        }

        // Check case expression
        if let Some(case_node) = value.as_case_node() {
            return self.check_case_branch(source, node, &case_node, var_name);
        }

        Vec::new()
    }
}

impl RedundantSelfAssignmentBranch {
    fn check_if_branch(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        if_node: &ruby_prism::IfNode<'_>,
        var_name: &[u8],
    ) -> Vec<Diagnostic> {
        // Check the if branch
        let if_branch_is_self = if let Some(stmts) = if_node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            body.len() == 1 && is_same_var(&body[0], var_name)
        } else {
            false
        };

        // Check the else branch
        let else_branch_is_self = if let Some(subsequent) = if_node.subsequent() {
            if let Some(else_node) = subsequent.as_else_node() {
                if let Some(stmts) = else_node.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    body.len() == 1 && is_same_var(&body[0], var_name)
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if if_branch_is_self || else_branch_is_self {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!(
                    "Redundant self-assignment branch. The variable `{}` is assigned to itself in one of the branches.",
                    String::from_utf8_lossy(var_name),
                ),
            )];
        }

        Vec::new()
    }

    fn check_case_branch(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        case_node: &ruby_prism::CaseNode<'_>,
        var_name: &[u8],
    ) -> Vec<Diagnostic> {
        // Check each when branch
        for condition in case_node.conditions().iter() {
            if let Some(when_node) = condition.as_when_node() {
                if let Some(stmts) = when_node.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    if body.len() == 1 && is_same_var(&body[0], var_name) {
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            format!(
                                "Redundant self-assignment branch. The variable `{}` is assigned to itself in one of the branches.",
                                String::from_utf8_lossy(var_name),
                            ),
                        )];
                    }
                }
            }
        }

        // Check else branch
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                if body.len() == 1 && is_same_var(&body[0], var_name) {
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        format!(
                            "Redundant self-assignment branch. The variable `{}` is assigned to itself in one of the branches.",
                            String::from_utf8_lossy(var_name),
                        ),
                    )];
                }
            }
        }

        Vec::new()
    }
}

fn is_same_var(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(lv) = node.as_local_variable_read_node() {
        return lv.name().as_slice() == var_name;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantSelfAssignmentBranch, "cops/style/redundant_self_assignment_branch");
}
