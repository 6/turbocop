use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ELSE_NODE, IF_NODE, INSTANCE_VARIABLE_WRITE_NODE, LOCAL_VARIABLE_WRITE_NODE};

pub struct ConditionalAssignment;

impl Cop for ConditionalAssignment {
    fn name(&self) -> &'static str {
        "Style/ConditionalAssignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ELSE_NODE, IF_NODE, INSTANCE_VARIABLE_WRITE_NODE, LOCAL_VARIABLE_WRITE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "assign_to_condition");
        let _single_line_only = config.get_bool("SingleLineConditionsOnly", true);
        let _include_ternary = config.get_bool("IncludeTernaryExpressions", true);

        if enforced_style != "assign_to_condition" {
            return;
        }

        // Check for if/else where each branch assigns to the same variable
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return,
        };

        // Must be a top-level `if`, not an `elsif` branch
        if let Some(kw_loc) = if_node.if_keyword_loc() {
            if kw_loc.as_slice() == b"elsif" {
                return;
            }
        }

        // Must have an else clause
        let else_clause = match if_node.subsequent() {
            Some(s) => s,
            None => return,
        };

        // Must be a simple if/else (not if/elsif/else)
        if else_clause.as_if_node().is_some() {
            return;
        }

        // Check if both branches assign to the same variable
        let if_body = match if_node.statements() {
            Some(s) => s,
            None => return,
        };

        let if_stmts: Vec<_> = if_body.body().iter().collect();
        if if_stmts.len() != 1 {
            return;
        }

        let if_assign_name = get_assignment_target(&if_stmts[0]);

        if let Some(else_node) = else_clause.as_else_node() {
            if let Some(else_stmts) = else_node.statements() {
                let else_list: Vec<_> = else_stmts.body().iter().collect();
                if else_list.len() != 1 {
                    return;
                }

                let else_assign_name = get_assignment_target(&else_list[0]);

                if let (Some(if_name), Some(else_name)) = (if_assign_name, else_assign_name) {
                    if if_name == else_name {
                        let loc = if_node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!(
                                "Use the return value of `if` expression for variable assignment and comparison."
                            ),
                        ));
                    }
                }
            }
        }

    }
}

fn get_assignment_target(node: &ruby_prism::Node<'_>) -> Option<String> {
    if let Some(write) = node.as_local_variable_write_node() {
        return Some(
            std::str::from_utf8(write.name().as_slice())
                .unwrap_or("")
                .to_string(),
        );
    }
    if let Some(write) = node.as_instance_variable_write_node() {
        return Some(
            std::str::from_utf8(write.name().as_slice())
                .unwrap_or("")
                .to_string(),
        );
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConditionalAssignment, "cops/style/conditional_assignment");
}
