use crate::cop::util::is_screaming_snake_case;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CONSTANT_PATH_WRITE_NODE, CONSTANT_WRITE_NODE};

pub struct ConstantName;

impl Cop for ConstantName {
    fn name(&self) -> &'static str {
        "Naming/ConstantName"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CONSTANT_PATH_WRITE_NODE, CONSTANT_WRITE_NODE]
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
        if let Some(cw) = node.as_constant_write_node() {
            let const_name = cw.name().as_slice();
            let value = cw.value();
            diagnostics.extend(self.check_constant(source, const_name, &cw.name_loc(), &value));
        }

        if let Some(cpw) = node.as_constant_path_write_node() {
            let target = cpw.target();
            let name_loc = target.name_loc();
            let const_name = target.name().map(|n| n.as_slice()).unwrap_or(b"");
            let value = cpw.value();
            diagnostics.extend(self.check_constant(source, const_name, &name_loc, &value));
        }

    }
}

impl ConstantName {
    fn check_constant(
        &self,
        source: &SourceFile,
        const_name: &[u8],
        loc: &ruby_prism::Location<'_>,
        value: &ruby_prism::Node<'_>,
    ) -> Vec<Diagnostic> {
        // Allow SCREAMING_SNAKE_CASE (standard constant style)
        if is_screaming_snake_case(const_name) {
            return Vec::new();
        }

        // Allow non-SCREAMING_SNAKE_CASE only if the RHS is a class/module/struct creation
        // pattern. This matches RuboCop's `valid_for_assignment?` check.
        if is_valid_rhs_for_assignment(value) {
            return Vec::new();
        }

        let (line, column) = source.offset_to_line_col(loc.start_offset());

        vec![self.diagnostic(
            source,
            line,
            column,
            "Use SCREAMING_SNAKE_CASE for constants.".to_string(),
        )]
    }
}

/// Check if the RHS of a constant assignment is an acceptable pattern for
/// non-SCREAMING_SNAKE_CASE names. Matches RuboCop's `allowed_assignment?`:
///
/// 1. Block, constant reference, or chained constant assignment
/// 2. Method call where receiver is nil or not a literal
/// 3. `Class.new(...)` or `Struct.new(...)`
/// 4. Conditional expression containing a constant in branches
/// 5. Lambda literal
fn is_valid_rhs_for_assignment(value: &ruby_prism::Node<'_>) -> bool {
    // Lambda literal: `-> { }`
    if value.as_lambda_node().is_some() {
        return true;
    }

    // Block node: `proc { }`, `lambda { }`, `Foo.new { }`
    if value.as_block_node().is_some() {
        return true;
    }

    // Constant reference: `Server = BaseServer` or `Stream = Foo::Bar`
    if value.as_constant_read_node().is_some() || value.as_constant_path_node().is_some() {
        return true;
    }

    // Chained constant assignment: `A = B = 1`
    if value.as_constant_write_node().is_some() || value.as_constant_path_write_node().is_some() {
        return true;
    }

    // Method call: allowed if receiver is nil or receiver is not a literal.
    // This covers patterns like `NewClass = some_factory_method` and
    // `Uchar1max = (1<<7) - 1` (receiver is a call expression, not a literal).
    // Only method calls ON bare literals like `"foo".freeze` or `1 + 2` are disallowed.
    if let Some(call) = value.as_call_node() {
        match call.receiver() {
            None => return true, // receiverless method call
            Some(receiver) => {
                if !is_literal_receiver(&receiver) {
                    return true;
                }
            }
        }
    }

    // Conditional expression containing a constant in branches
    if let Some(if_node) = value.as_if_node() {
        if branch_contains_constant(&if_node) {
            return true;
        }
    }

    false
}

/// Check if a receiver is a literal (int, float, string, symbol, etc.)
/// or a parenthesized literal `(literal)`. Matches RuboCop's `literal_receiver?`.
fn is_literal_receiver(node: &ruby_prism::Node<'_>) -> bool {
    if is_literal(node) {
        return true;
    }
    // `(literal)` — parenthesized literal
    if let Some(parens) = node.as_parentheses_node() {
        if let Some(body) = parens.body() {
            if let Some(stmts) = body.as_statements_node() {
                let children: Vec<_> = stmts.body().iter().collect();
                if children.len() == 1 && is_literal(&children[0]) {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a node is a literal value (int, float, string, symbol, etc.)
fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
}

/// Check if an if-expression has a constant in any of its branches.
fn branch_contains_constant(if_node: &ruby_prism::IfNode<'_>) -> bool {
    // Check the "then" branch
    if let Some(stmts) = if_node.statements() {
        for child in stmts.body().iter() {
            if child.as_constant_read_node().is_some()
                || child.as_constant_path_node().is_some()
            {
                return true;
            }
        }
    }
    // Check the else branch
    if let Some(else_clause) = if_node.subsequent() {
        if let Some(else_if) = else_clause.as_if_node() {
            if branch_contains_constant(&else_if) {
                return true;
            }
        }
        if let Some(else_node) = else_clause.as_else_node() {
            if let Some(stmts) = else_node.statements() {
                for child in stmts.body().iter() {
                    if child.as_constant_read_node().is_some()
                        || child.as_constant_path_node().is_some()
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConstantName, "cops/naming/constant_name");
}
