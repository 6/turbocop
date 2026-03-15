use crate::cop::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, FALSE_NODE, FLOAT_NODE, IF_NODE,
    INTEGER_NODE, NIL_NODE, OR_NODE, REGULAR_EXPRESSION_NODE, STRING_NODE, SYMBOL_NODE, TRUE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/CaseLikeIf flags `if-elsif` chains that can be converted to `case-when`.
///
/// ## Key behaviors matching RuboCop:
///
/// - **`find_target`**: Extracts the common comparison target from the first branch.
///   Only the first branch's condition determines the target. Supported patterns:
///   `==`/`eql?`/`equal?` (with literal or const_reference on one side),
///   `===` (argument is target), `is_a?` (receiver is target),
///   `=~`/`match`/`match?` (requires one side to be a regexp),
///   `include?`/`cover?` (requires range receiver).
///
/// - **`collect_conditions`**: Validates ALL branches are convertible against the
///   found target. Rejects branches where `==` compares against a class reference
///   (mixed-case constant like `Foo`) since `case/when` uses `===` which behaves
///   differently for classes.
///
/// - **Parenthesized conditions**: `if (x == 1)` is deparenthesized (ParenthesesNode
///   with single-statement body is unwrapped).
///
/// - **`ConstantPathNode`**: `Module::CONSTANT` is treated as a const_reference
///   if the last segment is all-uppercase (>1 char). This matches RuboCop's
///   `const_reference?` which checks `const_type?` nodes.
///
/// - **`match?` vs `is_a?`**: These are handled differently. `is_a?`/`kind_of?`
///   always use the receiver as target. `match?`/`match`/`=~` require one side
///   to be a regexp literal.
///
/// ## Corpus findings:
/// - 352 FPs were from: (1) `match?` grouped with `is_a?` causing non-regexp
///   `x.match?(y)` to be flagged, (2) missing `class_reference?` check on
///   equality conditions allowing `x == Foo` patterns through.
/// - 37 FNs were from: (1) `ConstantPathNode` not treated as const_reference,
///   (2) parenthesized conditions not deparenthesized, (3) `include?`/`cover?`
///   with range not supported.
/// - 398 FPs (second round): nitrocop was processing `elsif` IfNodes as
///   top-level `if` nodes. In Prism, `elsif` branches are nested IfNodes.
///   A chain of N branches produced up to N-2 extra offenses when N>=4
///   (with MinBranchesCount=3). Fix: skip nodes whose `if_keyword_loc`
///   starts with "elsif" or "unless", and skip modifier if (no end keyword)
///   and ternary (no if_keyword_loc). This matches RuboCop's `should_check?`.
pub struct CaseLikeIf;

impl Cop for CaseLikeIf {
    fn name(&self) -> &'static str {
        "Style/CaseLikeIf"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            FALSE_NODE,
            FLOAT_NODE,
            IF_NODE,
            INTEGER_NODE,
            NIL_NODE,
            OR_NODE,
            REGULAR_EXPRESSION_NODE,
            STRING_NODE,
            SYMBOL_NODE,
            TRUE_NODE,
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
        let min_branches = config.get_usize("MinBranchesCount", 3);

        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return,
        };

        // Skip unless, elsif, modifier if, and ternary nodes (matches RuboCop's should_check?)
        // In Prism, elsif branches are nested IfNodes whose if_keyword_loc starts with "elsif".
        // Unless nodes have if_keyword_loc starting with "unless".
        // Ternary has no if_keyword_loc. Modifier if has no end_keyword_loc.
        match if_node.if_keyword_loc() {
            None => return, // ternary operator
            Some(kw_loc) => {
                let kw = &source.as_bytes()[kw_loc.start_offset()..kw_loc.end_offset()];
                if kw.starts_with(b"elsif") || kw.starts_with(b"unless") {
                    return;
                }
            }
        }
        // Modifier if: no end keyword
        if if_node.end_keyword_loc().is_none() {
            return;
        }

        // Count branches (if + elsif chain)
        let mut branch_count = 1;
        let mut current_else = if_node.subsequent();
        while let Some(else_clause) = current_else {
            if let Some(elsif) = else_clause.as_if_node() {
                branch_count += 1;
                current_else = elsif.subsequent();
            } else {
                break;
            }
        }

        if branch_count < min_branches {
            return;
        }

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

        // Phase 1: Find the target from the first condition
        let target = match with_unwrapped(&predicates[0], &find_target) {
            Some(t) => t,
            None => return,
        };

        // Phase 2: Verify all conditions are convertible against the target
        for pred in &predicates {
            let convertible = with_unwrapped(pred, &|n| is_condition_convertible(n, &target));
            if !convertible {
                return;
            }
        }

        let loc = if_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Convert `if-elsif` to `case-when`.".to_string(),
        ));
    }
}

/// Get the inner expression from a potentially parenthesized node.
/// If the node is a ParenthesesNode containing exactly one statement,
/// call the provided closure with that inner statement.
/// Otherwise call the closure with the original node.
fn with_unwrapped<R>(node: &ruby_prism::Node<'_>, f: &dyn Fn(&ruby_prism::Node<'_>) -> R) -> R {
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            if let Some(stmts) = body.as_statements_node() {
                let children: Vec<_> = stmts.body().iter().collect();
                if children.len() == 1 {
                    return with_unwrapped(&children[0], f);
                }
            }
        }
    }
    f(node)
}

/// Extract the target from a condition node (RuboCop's `find_target`).
/// Returns the source bytes of the target expression.
fn find_target(node: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
    // Handle `||` - find target from the left side
    if let Some(or_node) = node.as_or_node() {
        return with_unwrapped(&or_node.left(), &find_target);
    }

    if let Some(call) = node.as_call_node() {
        let method = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        match method {
            "is_a?" | "kind_of?" => {
                // Target is receiver: x.is_a?(Foo) -> target = x
                if let Some(receiver) = call.receiver() {
                    return Some(receiver.location().as_slice().to_vec());
                }
            }
            "==" | "eql?" | "equal?" => {
                return find_target_in_equality_node(&call);
            }
            "===" => {
                // Target is the argument: Integer === x -> target = x
                let args = call.arguments();
                let first_arg = args.as_ref().and_then(|a| a.arguments().iter().next());
                if let Some(arg) = first_arg {
                    return Some(arg.location().as_slice().to_vec());
                }
            }
            "=~" | "match" | "match?" => {
                return find_target_in_match_node(&call);
            }
            "include?" | "cover?" => {
                return find_target_in_include_or_cover_node(&call);
            }
            _ => {}
        }
    }
    None
}

/// For `==`/`eql?`/`equal?`, find the target (non-literal side).
/// Requires one side to be a literal or const_reference.
fn find_target_in_equality_node(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let receiver = call.receiver()?;
    let args = call.arguments();
    let first_arg = args.as_ref().and_then(|a| a.arguments().iter().next())?;

    if is_literal(&first_arg) || is_const_reference(&first_arg) {
        Some(receiver.location().as_slice().to_vec())
    } else if is_literal(&receiver) || is_const_reference(&receiver) {
        Some(first_arg.location().as_slice().to_vec())
    } else {
        None
    }
}

/// For `=~`/`match`/`match?`, find the target (non-regexp side).
/// Requires one side to be a regexp literal.
fn find_target_in_match_node(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let receiver = call.receiver()?;
    let args = call.arguments();
    let first_arg = args.as_ref().and_then(|a| a.arguments().iter().next());

    // For all match methods: one side must be a regexp, the other is the target
    if let Some(ref arg) = first_arg {
        if receiver.as_regular_expression_node().is_some() {
            return Some(arg.location().as_slice().to_vec());
        } else if arg.as_regular_expression_node().is_some() {
            return Some(receiver.location().as_slice().to_vec());
        }
    }
    None
}

/// For `include?`/`cover?`, find the target (argument when receiver is a range).
fn find_target_in_include_or_cover_node(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let receiver = call.receiver()?;
    // Check if receiver (possibly parenthesized) is a range
    let is_range = with_unwrapped(&receiver, &|n| n.as_range_node().is_some());
    if !is_range {
        return None;
    }
    let args = call.arguments();
    let first_arg = args.as_ref().and_then(|a| a.arguments().iter().next())?;
    Some(first_arg.location().as_slice().to_vec())
}

/// Check if a condition is convertible against the target.
/// This handles `||` by recursing into both sides.
fn is_condition_convertible(node: &ruby_prism::Node<'_>, target: &[u8]) -> bool {
    // Handle `||` - both sides must be convertible
    if let Some(or_node) = node.as_or_node() {
        let left_ok = with_unwrapped(&or_node.left(), &|n| is_condition_convertible(n, target));
        let right_ok = with_unwrapped(&or_node.right(), &|n| is_condition_convertible(n, target));
        return left_ok && right_ok;
    }

    if let Some(call) = node.as_call_node() {
        let method = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        match method {
            "is_a?" | "kind_of?" => {
                // Receiver must be the target
                if let Some(receiver) = call.receiver() {
                    return receiver.location().as_slice() == target;
                }
            }
            "==" | "eql?" | "equal?" => {
                return is_equality_condition_convertible(&call, target);
            }
            "===" => {
                // Argument must be the target
                let args = call.arguments();
                let first_arg = args.as_ref().and_then(|a| a.arguments().iter().next());
                if let Some(arg) = first_arg {
                    return arg.location().as_slice() == target;
                }
            }
            "=~" | "match" | "match?" => {
                return is_match_condition_convertible(&call, target);
            }
            "include?" | "cover?" => {
                return is_include_condition_convertible(&call, target);
            }
            _ => {}
        }
    }
    false
}

/// Check if an equality condition (`==`/`eql?`/`equal?`) is convertible.
/// One side must be the target, and the other side must NOT be a class reference
/// (mixed-case constant like `Foo` or `Foo::Bar`).
fn is_equality_condition_convertible(call: &ruby_prism::CallNode<'_>, target: &[u8]) -> bool {
    let receiver = match call.receiver() {
        Some(r) => r,
        None => return false,
    };
    let args = call.arguments();
    let first_arg = match args.as_ref().and_then(|a| a.arguments().iter().next()) {
        Some(a) => a,
        None => return false,
    };

    // Determine which side is the target and which is the condition
    let condition = if receiver.location().as_slice() == target {
        first_arg
    } else if first_arg.location().as_slice() == target {
        receiver
    } else {
        return false;
    };

    // RuboCop rejects conditions that are class references (mixed-case constants)
    // since case/when uses === which behaves differently for classes
    !is_class_reference(&condition)
}

/// Check if a match condition (`=~`/`match`/`match?`) is convertible.
fn is_match_condition_convertible(call: &ruby_prism::CallNode<'_>, target: &[u8]) -> bool {
    let receiver = match call.receiver() {
        Some(r) => r,
        None => return false,
    };
    let args = call.arguments();
    let first_arg = args.as_ref().and_then(|a| a.arguments().iter().next());

    // One side must be the target
    if let Some(ref arg) = first_arg {
        if receiver.location().as_slice() == target || arg.location().as_slice() == target {
            return true;
        }
    }
    false
}

/// Check if an include?/cover? condition is convertible.
fn is_include_condition_convertible(call: &ruby_prism::CallNode<'_>, target: &[u8]) -> bool {
    let receiver = match call.receiver() {
        Some(r) => r,
        None => return false,
    };
    let is_range = with_unwrapped(&receiver, &|n| n.as_range_node().is_some());
    if !is_range {
        return false;
    }
    let args = call.arguments();
    let first_arg = match args.as_ref().and_then(|a| a.arguments().iter().next()) {
        Some(a) => a,
        None => return false,
    };
    first_arg.location().as_slice() == target
}

/// Check if a node is a literal value (string, symbol, integer, etc.)
/// Does NOT include constants - those are checked separately via is_const_reference.
fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_array_node().is_some()
}

/// RuboCop's `const_reference?`: returns true for constants whose last segment
/// is all uppercase and longer than 1 character (e.g. `HTTP`, `PI`, `CONSTANT1`,
/// `Module::CONSTANT`). This prevents treating class names like `MyClass` as literals.
fn is_const_reference(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(c) = node.as_constant_read_node() {
        let name = c.name().as_slice();
        return is_all_uppercase_name(name);
    }
    if let Some(cp) = node.as_constant_path_node() {
        // For Foo::BAR, check if the last segment (name) is all uppercase
        if let Some(name_node) = cp.name() {
            let name = name_node.as_slice();
            return is_all_uppercase_name(name);
        }
    }
    false
}

/// Check if a name is all uppercase (letters, digits, underscores) and >1 char.
fn is_all_uppercase_name(name: &[u8]) -> bool {
    name.len() > 1
        && name
            .iter()
            .all(|&b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

/// RuboCop's `class_reference?`: returns true for constants that contain lowercase
/// letters in their last segment name (e.g. `Foo`, `Bar`, `MyClass`, `Foo::Bar`).
/// These represent class/module references that behave differently with `===`.
fn is_class_reference(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(c) = node.as_constant_read_node() {
        let name = c.name().as_slice();
        return name.iter().any(|&b| b.is_ascii_lowercase());
    }
    if let Some(cp) = node.as_constant_path_node() {
        if let Some(name_node) = cp.name() {
            let name = name_node.as_slice();
            return name.iter().any(|&b| b.is_ascii_lowercase());
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CaseLikeIf, "cops/style/case_like_if");
}
