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

/// Extract all operands from a comparison condition.
/// For `x == 1`, returns `[x, 1]`.
/// For `x.is_a?(Foo)`, returns `[x]` (the receiver).
/// For `x == 1 || x == 2`, returns the operands from the || branches.
fn get_comparison_operands(node: &ruby_prism::Node<'_>) -> Option<Vec<Vec<u8>>> {
    if let Some(call) = node.as_call_node() {
        let method = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        match method {
            "==" | "===" | "=~" => {
                let mut operands = Vec::new();
                if let Some(receiver) = call.receiver() {
                    operands.push(receiver.location().as_slice().to_vec());
                }
                if let Some(args) = call.arguments() {
                    for arg in args.arguments().iter() {
                        operands.push(arg.location().as_slice().to_vec());
                    }
                }
                if !operands.is_empty() {
                    return Some(operands);
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
