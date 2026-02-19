use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct CaseLikeIf;

impl Cop for CaseLikeIf {
    fn name(&self) -> &'static str {
        "Style/CaseLikeIf"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let min_branches = config.get_usize("MinBranchesCount", 3);

        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Count branches (if + elsif chain)
        let mut branch_count = 1;
        let mut current_else = if_node.subsequent();
        while let Some(else_clause) = current_else {
            if let Some(elsif) = else_clause.as_if_node() {
                branch_count += 1;
                current_else = elsif.subsequent();
            } else {
                // else clause
                break;
            }
        }

        if branch_count < min_branches {
            return Vec::new();
        }

        // Check that ALL conditions compare against the same variable
        // using ==, ===, is_a?, kind_of?, match?, =~
        // Collect all predicates from the if-elsif chain
        let mut predicates = vec![if_node.predicate()];
        let mut current_else = if_node.subsequent();
        while let Some(else_clause) = current_else {
            if let Some(elsif) = else_clause.as_if_node() {
                predicates.push(elsif.predicate());
                current_else = elsif.subsequent();
            } else {
                break;
            }
        }

        // Extract operands from first predicate to find the target variable
        let first_operands = match get_comparison_operands(&predicates[0]) {
            Some(ops) => ops,
            None => return Vec::new(),
        };

        // Try each operand from the first condition as the potential target
        let target = 'find_target: {
            for candidate in &first_operands {
                let mut all_match = true;
                for pred in &predicates[1..] {
                    match get_comparison_operands(pred) {
                        Some(ops) if ops.contains(candidate) => {}
                        _ => { all_match = false; break; }
                    }
                }
                if all_match {
                    break 'find_target Some(candidate.clone());
                }
            }
            None
        };

        if target.is_none() {
            return Vec::new();
        }

        let loc = if_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Convert `if-elsif` to `case-when`.".to_string(),
        )]
    }
}

/// Check if a node is a literal value (string, symbol, integer, constant reference, etc.)
/// For constants, only fully uppercase names (like `HTTP`, `PI`) are treated as literal
/// references. Mixed-case constants like `MyClass` could be class references, so they
/// are not treated as literals (matching RuboCop's `const_reference?` behavior).
fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_regular_expression_node().is_some()
        || is_const_reference(node)
}

/// RuboCop's `const_reference?`: only returns true for simple constants whose name
/// is all uppercase and longer than 1 character (e.g. `HTTP`, `PI`, `CONSTANT1`).
/// This prevents treating class names like `MyClass` or `Foo::Bar` as literals.
fn is_const_reference(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(c) = node.as_constant_read_node() {
        let name = c.name().as_slice();
        // Name must be > 1 char, entirely uppercase/digits/underscores,
        // and must equal its uppercase form (no lowercase letters)
        if name.len() > 1 && name.iter().all(|&b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_') {
            return true;
        }
    }
    // as_constant_path_node (e.g. Foo::BAR) - not treated as a const reference
    // since each segment would need checking
    false
}

/// Extract the "target" (non-literal operand) from a comparison condition.
/// For `x == 'foo'`, returns `[x]` (the non-literal side).
/// For `x.is_a?(Foo)`, returns `[x]` (the receiver).
/// For `x == 1 || x == 2`, returns the target from the || branches.
fn get_comparison_operands(node: &ruby_prism::Node<'_>) -> Option<Vec<Vec<u8>>> {
    if let Some(call) = node.as_call_node() {
        let method = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        match method {
            "===" => {
                // For ===, the first argument is always the target (case subject)
                let args = call.arguments();
                let first_arg = args.as_ref().and_then(|a| a.arguments().iter().next());
                if let Some(arg) = first_arg {
                    return Some(vec![arg.location().as_slice().to_vec()]);
                }
            }
            "==" | "eql?" | "equal?" | "=~" => {
                let receiver = call.receiver();
                let args = call.arguments();
                let first_arg = args.as_ref().and_then(|a| a.arguments().iter().next());
                if let (Some(recv), Some(arg)) = (receiver, first_arg) {
                    // The target is the non-literal side; the literal side is the value
                    if is_literal(&arg) {
                        return Some(vec![recv.location().as_slice().to_vec()]);
                    } else if is_literal(&recv) {
                        return Some(vec![arg.location().as_slice().to_vec()]);
                    }
                    // Both non-literal: RuboCop requires at least one side to be
                    // a literal or const reference for the pattern to be case-like.
                    // Return None to skip this comparison.
                    return None;
                }
            }
            "is_a?" | "kind_of?" | "match?" => {
                if let Some(receiver) = call.receiver() {
                    return Some(vec![receiver.location().as_slice().to_vec()]);
                }
            }
            _ => {}
        }
    }
    // Handle `||` conditions: `x == 1 || x == 2`
    if let Some(or_node) = node.as_or_node() {
        let left_ops = get_comparison_operands(&or_node.left());
        let right_ops = get_comparison_operands(&or_node.right());
        if let (Some(mut l), Some(r)) = (left_ops, right_ops) {
            for op in r {
                if !l.contains(&op) {
                    l.push(op);
                }
            }
            return Some(l);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CaseLikeIf, "cops/style/case_like_if");
}
