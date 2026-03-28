use crate::cop::node_type::{AND_NODE, OR_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Fixed: `or_with_and` and `and_with_or` did not traverse through Prism
/// `ParenthesesNode` wrappers, missing patterns like `unless a && (b || c)`.
/// RuboCop's backtick node patterns search all descendants including through
/// `begin` (parentheses) nodes.
///
/// Important subtlety: when the *entire* condition is wrapped in outer parens
/// (e.g., `unless (a && b || c)`), RuboCop does NOT flag it because its
/// `or_with_and?`/`and_with_or?` patterns require the condition itself to be
/// an OR/AND node. Only the mixed_precedence checks work through outer parens
/// (via `each_descendant`). We match this by not unwrapping top-level parens
/// in `contains_mixed_logical_operators`.
///
/// Remaining FN (~51): mostly in repos where the condition spans multiple lines
/// or uses line continuations that may affect how Prism parses the unless node.
pub struct UnlessLogicalOperators;

impl Cop for UnlessLogicalOperators {
    fn name(&self) -> &'static str {
        "Style/UnlessLogicalOperators"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[AND_NODE, OR_NODE, UNLESS_NODE]
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
        let enforced_style = config.get_str("EnforcedStyle", "forbid_mixed_logical_operators");

        let unless_node = match node.as_unless_node() {
            Some(u) => u,
            None => return,
        };

        let predicate = unless_node.predicate();

        match enforced_style {
            "forbid_logical_operators" => {
                // Flag any logical operators in unless conditions
                if contains_logical_operator(&predicate) {
                    let (line, column) =
                        source.offset_to_line_col(unless_node.keyword_loc().start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not use logical operators in `unless` conditions.".to_string(),
                    ));
                }
            }
            _ => {
                // Flag mixed logical operators (both && and ||)
                if contains_mixed_logical_operators(&predicate) {
                    let (line, column) =
                        source.offset_to_line_col(unless_node.keyword_loc().start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not use mixed logical operators in `unless` conditions.".to_string(),
                    ));
                }
            }
        }
    }
}

fn contains_logical_operator(node: &ruby_prism::Node<'_>) -> bool {
    node.as_and_node().is_some() || node.as_or_node().is_some()
}

/// Check if the condition has mixed logical operators.
/// Matches RuboCop's `or_with_and?` and `and_with_or?` patterns which use
/// backtick (descendant) matching through parentheses, plus
/// `mixed_precedence_and?`/`mixed_precedence_or?` for `&&`/`and` or `||`/`or` mixing.
fn contains_mixed_logical_operators(node: &ruby_prism::Node<'_>) -> bool {
    // Do NOT unwrap top-level parentheses for or_with_and/and_with_or:
    // RuboCop's patterns require the condition itself to be an OR/AND node.
    // Wrapping the entire condition in parens (e.g. `unless (a && b || c)`)
    // prevents those checks from matching, which matches RuboCop behavior.
    // The mixed_precedence checks work through parens via collect functions.
    or_with_and(node)
        || and_with_or(node)
        || mixed_precedence_and(node)
        || mixed_precedence_or(node)
}

/// Unwrap a ParenthesesNode to get the inner expression.
/// Returns Some(inner) if the node is a ParenthesesNode, None otherwise.
/// Note: Prism may wrap the body in a StatementsNode, which is handled
/// transparently by the caller functions that recurse through it.
fn unwrap_parens<'a>(node: &ruby_prism::Node<'a>) -> Option<ruby_prism::Node<'a>> {
    let paren = node.as_parentheses_node()?;
    paren.body()
}

/// Check if a node or any descendant (through parens, OR, and AND nodes) is an AND node.
fn has_and_descendant(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_and_node().is_some() {
        return true;
    }
    if let Some(inner) = unwrap_parens(node) {
        return has_and_descendant(&inner);
    }
    if let Some(stmts) = node.as_statements_node() {
        return stmts.body().iter().any(|s| has_and_descendant(&s));
    }
    if let Some(or_node) = node.as_or_node() {
        return has_and_descendant(&or_node.left()) || has_and_descendant(&or_node.right());
    }
    false
}

/// Check if a node or any descendant (through parens, OR, and AND nodes) is an OR node.
fn has_or_descendant(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_or_node().is_some() {
        return true;
    }
    if let Some(inner) = unwrap_parens(node) {
        return has_or_descendant(&inner);
    }
    if let Some(stmts) = node.as_statements_node() {
        return stmts.body().iter().any(|s| has_or_descendant(&s));
    }
    if let Some(and_node) = node.as_and_node() {
        return has_or_descendant(&and_node.left()) || has_or_descendant(&and_node.right());
    }
    false
}

/// An OR node that contains an AND node anywhere in its subtree.
/// Searches through parentheses, matching RuboCop's `(if (or <`and ...>) ...)`.
fn or_with_and(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(or_node) = node.as_or_node() {
        has_and_descendant(&or_node.left()) || has_and_descendant(&or_node.right())
    } else {
        false
    }
}

/// An AND node that contains an OR node anywhere in its subtree.
/// Searches through parentheses, matching RuboCop's `(if (and <`or ...>) ...)`.
fn and_with_or(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(and_node) = node.as_and_node() {
        has_or_descendant(&and_node.left()) || has_or_descendant(&and_node.right())
    } else {
        false
    }
}

/// Check for mixing `&&` with `and` operators.
fn mixed_precedence_and(node: &ruby_prism::Node<'_>) -> bool {
    let mut ops = Vec::new();
    collect_and_operators(node, &mut ops);
    if ops.len() < 2 {
        return false;
    }
    // Mixed if not all symbolic (&&) and not all keyword (and)
    !(ops.iter().all(|&s| s) || ops.iter().all(|&s| !s))
}

/// Check for mixing `||` with `or` operators.
fn mixed_precedence_or(node: &ruby_prism::Node<'_>) -> bool {
    let mut ops = Vec::new();
    collect_or_operators(node, &mut ops);
    if ops.len() < 2 {
        return false;
    }
    !(ops.iter().all(|&s| s) || ops.iter().all(|&s| !s))
}

/// Collect all AND operators in the tree, traversing through parentheses and OR nodes.
/// Matches RuboCop's `each_descendant(:and)` behavior.
fn collect_and_operators(node: &ruby_prism::Node<'_>, ops: &mut Vec<bool>) {
    if let Some(and_node) = node.as_and_node() {
        let is_symbolic = and_node.operator_loc().as_slice() == b"&&";
        ops.push(is_symbolic);
        collect_and_operators(&and_node.left(), ops);
        collect_and_operators(&and_node.right(), ops);
    } else if let Some(inner) = unwrap_parens(node) {
        collect_and_operators(&inner, ops);
    } else if let Some(stmts) = node.as_statements_node() {
        for s in stmts.body().iter() {
            collect_and_operators(&s, ops);
        }
    } else if let Some(or_node) = node.as_or_node() {
        collect_and_operators(&or_node.left(), ops);
        collect_and_operators(&or_node.right(), ops);
    }
}

/// Collect all OR operators in the tree, traversing through parentheses and AND nodes.
/// Matches RuboCop's `each_descendant(:or)` behavior.
fn collect_or_operators(node: &ruby_prism::Node<'_>, ops: &mut Vec<bool>) {
    if let Some(or_node) = node.as_or_node() {
        let is_symbolic = or_node.operator_loc().as_slice() == b"||";
        ops.push(is_symbolic);
        collect_or_operators(&or_node.left(), ops);
        collect_or_operators(&or_node.right(), ops);
    } else if let Some(inner) = unwrap_parens(node) {
        collect_or_operators(&inner, ops);
    } else if let Some(stmts) = node.as_statements_node() {
        for s in stmts.body().iter() {
            collect_or_operators(&s, ops);
        }
    } else if let Some(and_node) = node.as_and_node() {
        collect_or_operators(&and_node.left(), ops);
        collect_or_operators(&and_node.right(), ops);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        UnlessLogicalOperators,
        "cops/style/unless_logical_operators"
    );
}
