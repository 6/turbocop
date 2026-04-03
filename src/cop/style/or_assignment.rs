use crate::cop::shared::node_type::{
    CLASS_VARIABLE_WRITE_NODE, GLOBAL_VARIABLE_WRITE_NODE, IF_NODE, INSTANCE_VARIABLE_WRITE_NODE,
    LOCAL_VARIABLE_WRITE_NODE, UNLESS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Detects patterns that can be replaced with `||=`.
///
/// Handles three patterns matching RuboCop's Style/OrAssignment:
/// - Ternary: `x = x ? x : y` / `x = if x; x; else y; end` (skips elsif)
/// - Modifier unless: `x = y unless x`
/// - Block unless: `unless x; x = y; end` (skips unless-else)
///
/// Deliberately does NOT flag `x = x || y` — RuboCop's cop doesn't either.
pub struct OrAssignment;

impl OrAssignment {
    /// Get variable name from a local/instance/class/global variable write node
    fn get_write_name(node: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
        if let Some(lv) = node.as_local_variable_write_node() {
            return Some(lv.name().as_slice().to_vec());
        }
        if let Some(iv) = node.as_instance_variable_write_node() {
            return Some(iv.name().as_slice().to_vec());
        }
        if let Some(cv) = node.as_class_variable_write_node() {
            return Some(cv.name().as_slice().to_vec());
        }
        if let Some(gv) = node.as_global_variable_write_node() {
            return Some(gv.name().as_slice().to_vec());
        }
        None
    }

    /// Get variable name from a local/instance/class/global variable read node
    fn get_read_name(node: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
        if let Some(lv) = node.as_local_variable_read_node() {
            return Some(lv.name().as_slice().to_vec());
        }
        if let Some(iv) = node.as_instance_variable_read_node() {
            return Some(iv.name().as_slice().to_vec());
        }
        if let Some(cv) = node.as_class_variable_read_node() {
            return Some(cv.name().as_slice().to_vec());
        }
        if let Some(gv) = node.as_global_variable_read_node() {
            return Some(gv.name().as_slice().to_vec());
        }
        None
    }

    /// Check for `x = x || y` pattern — local variable or-assign
    fn check_or_assign(
        cop: &OrAssignment,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
    ) -> Vec<Diagnostic> {
        let write_name = match Self::get_write_name(node) {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Get the value being assigned
        let value = if let Some(lv) = node.as_local_variable_write_node() {
            lv.value()
        } else if let Some(iv) = node.as_instance_variable_write_node() {
            iv.value()
        } else if let Some(cv) = node.as_class_variable_write_node() {
            cv.value()
        } else if let Some(gv) = node.as_global_variable_write_node() {
            gv.value()
        } else {
            return Vec::new();
        };

        // Check for ternary: `x = x ? x : y` or `x = if x; x; else y; end`
        if let Some(if_node) = value.as_if_node() {
            // Skip if there's an elsif (subsequent is another IfNode, not ElseNode)
            if let Some(ref subsequent) = if_node.subsequent() {
                if subsequent.as_if_node().is_some() {
                    return Vec::new();
                }
            }

            let predicate = if_node.predicate();
            if let Some(pred_name) = Self::get_read_name(&predicate) {
                if pred_name == write_name {
                    // Check if true branch is the same variable
                    if let Some(true_branch) = if_node.statements() {
                        let true_nodes: Vec<_> = true_branch.body().into_iter().collect();
                        if true_nodes.len() == 1 {
                            if let Some(true_name) = Self::get_read_name(&true_nodes[0]) {
                                if true_name == write_name {
                                    let loc = node.location();
                                    let (line, column) =
                                        source.offset_to_line_col(loc.start_offset());
                                    return vec![
                                        cop.diagnostic(
                                            source,
                                            line,
                                            column,
                                            "Use the double pipe equals operator `||=` instead."
                                                .to_string(),
                                        ),
                                    ];
                                }
                            }
                        }
                    }
                }
            }
        }

        Vec::new()
    }

    /// Check for `unless x; x = y; end` and `x = y unless x` patterns
    fn check_unless_assign(
        cop: &OrAssignment,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
    ) -> Vec<Diagnostic> {
        let unless_node = match node.as_unless_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Skip unless with else clause — not equivalent to ||=
        if unless_node.else_clause().is_some() {
            return Vec::new();
        }

        // Get the predicate variable name
        let predicate = unless_node.predicate();
        let pred_name = match Self::get_read_name(&predicate) {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Get the statements body — must have exactly one statement
        let statements = match unless_node.statements() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body: Vec<_> = statements.body().into_iter().collect();
        if body.len() != 1 {
            return Vec::new();
        }

        // The single statement must be a variable write with the same name
        let write_name = match Self::get_write_name(&body[0]) {
            Some(n) => n,
            None => return Vec::new(),
        };
        if write_name != pred_name {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![cop.diagnostic(
            source,
            line,
            column,
            "Use the double pipe equals operator `||=` instead.".to_string(),
        )]
    }

    /// Check for `if x; else; x = y; end` pattern (empty then-branch)
    fn check_if_unless_assign(
        cop: &OrAssignment,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
    ) -> Vec<Diagnostic> {
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Must have empty then-branch (statements is None)
        if if_node.statements().is_some() {
            return Vec::new();
        }

        // Get the predicate variable name
        let predicate = if_node.predicate();
        let pred_name = match Self::get_read_name(&predicate) {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Get the else clause
        let subsequent = match if_node.subsequent() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // The subsequent must be an ElseNode (not elsif)
        let else_node = match subsequent.as_else_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Get the else body — must have exactly one statement
        let statements = match else_node.statements() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body: Vec<_> = statements.body().into_iter().collect();
        if body.len() != 1 {
            return Vec::new();
        }

        // The single statement must be a variable write with the same name
        let write_name = match Self::get_write_name(&body[0]) {
            Some(n) => n,
            None => return Vec::new(),
        };
        if write_name != pred_name {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![cop.diagnostic(
            source,
            line,
            column,
            "Use the double pipe equals operator `||=` instead.".to_string(),
        )]
    }
}

impl Cop for OrAssignment {
    fn name(&self) -> &'static str {
        "Style/OrAssignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CLASS_VARIABLE_WRITE_NODE,
            GLOBAL_VARIABLE_WRITE_NODE,
            IF_NODE,
            INSTANCE_VARIABLE_WRITE_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
            UNLESS_NODE,
        ]
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
        diagnostics.extend(Self::check_or_assign(self, source, node));
        diagnostics.extend(Self::check_unless_assign(self, source, node));
        diagnostics.extend(Self::check_if_unless_assign(self, source, node));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OrAssignment, "cops/style/or_assignment");
}
