use crate::cop::shared::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, FALSE_NODE, FLOAT_NODE, IF_NODE,
    INTEGER_NODE, NIL_NODE, OR_NODE, REGULAR_EXPRESSION_NODE, STRING_NODE, SYMBOL_NODE, TRUE_NODE,
};
use crate::cop::shared::util;
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
/// - **`match?` vs `is_a?`**: These are handled differently. `is_a?`
///   always uses the receiver as target. `match?`/`match`/`=~` require one side
///   to be a regexp literal. Note: `kind_of?` is intentionally excluded —
///   RuboCop's `find_target_in_send_node` only handles `:is_a?`, not `:kind_of?`.
///
/// ## Corpus findings:
/// - 352 FPs were from: (1) `match?` grouped with `is_a?` causing non-regexp
///   `x.match?(y)` to be flagged, (2) missing `class_reference?` check on
///   equality conditions allowing `x == Foo` patterns through.
/// - 37 FNs were from: (1) `ConstantPathNode` not treated as const_reference,
///   (2) parenthesized conditions not deparenthesized, (3) `include?`/`cover?`
///   with range not supported.
/// - 12 FPs (third round): nitrocop handled `kind_of?` as equivalent to `is_a?`,
///   but RuboCop's `find_target_in_send_node` and `condition_from_send_node` only
///   handle `:is_a?`. Fix: remove `kind_of?` from both `find_target` and
///   `is_condition_convertible`.
/// - 398 FPs (second round): nitrocop was processing `elsif` IfNodes as
///   top-level `if` nodes. In Prism, `elsif` branches are nested IfNodes.
///   A chain of N branches produced up to N-2 extra offenses when N>=4
///   (with MinBranchesCount=3). Fix: skip nodes whose `if_keyword_loc`
///   starts with "elsif" or "unless", and skip modifier if (no end keyword)
///   and ternary (no if_keyword_loc). This matches RuboCop's `should_check?`.
/// - 4 FPs / 6 FNs (fourth round): (1) FPs from safe navigation (`&.`) —
///   Prism merges `send` and `csend` into `CallNode`, but RuboCop's
///   `find_target_in_send_node` and `condition_from_send_node` only handle
///   `:send`, not `:csend`. Fix: skip CallNodes with `call_operator()` containing
///   `&.`. (2) FNs from interpolated regexps — Prism uses separate
///   `InterpolatedRegularExpressionNode` for `/#{...}/`, but Parser AST uses
///   `:regexp` for both. Fix: check `as_interpolated_regular_expression_node()`
///   alongside `as_regular_expression_node()` in all regexp checks.
/// - 3 FPs (fifth round): RuboCop's `branch_conditions` walks into any
///   if_type node including nested if/unless in the `else` body (in Parser AST,
///   both `if` and `unless` are `:if` type). When the else body contains a
///   modifier `unless` (e.g., `x unless cond`) or a nested `if-else` block,
///   RuboCop treats their conditions as part of the branch chain. If these
///   conditions aren't convertible to `case-when` (e.g., `line.start_with?()`,
///   `value.nil?`), the entire chain is rejected. In Prism, `else` wraps its
///   body in an ElseNode and `unless` is a separate UnlessNode (not IfNode).
///   Fix: after the elsif chain walk, unwrap ElseNode → single-statement
///   IfNode or UnlessNode and continue walking their predicates/branches.
///   The 1 FN (chatwoot) appears to be a corpus oracle data issue — RuboCop
///   with default MinBranchesCount=3 does not flag a 2-branch+else chain,
///   verified independently.
/// - 2 FPs (sixth round): nitrocop flagged `if...else; if...; end; end` patterns
///   where the outer if has no `elsif` — only an `else` with a nested `if` block.
///   RuboCop's `should_check?` requires `elsif_conditional?` which checks that
///   the else_branch is both `if_type?` AND `elsif?` (keyword is 'elsif', not 'if').
///   In Prism, `elsif` subsequents are IfNodes while `else` subsequents are ElseNodes.
///   Fix: check that `if_node.subsequent()` is a direct IfNode before processing.
/// - 6 FNs (2026-03-30): RuboCop's `MinBranchesCount` counts `else` bodies whose
///   single expression is another `if_type?`, including ternaries. Its
///   `branch_conditions` walk still skips ternaries, so `if/elsif/else <ternary>`
///   chains are offenses even when only the non-ternary predicates are
///   convertible. Prism wraps `else` bodies in `ElseNode`, so fix by counting
///   `ElseNode -> single IfNode/UnlessNode` for the threshold while keeping the
///   convertibility walk strict and skipping ternary predicates. This restores
///   the missed corpus cases from kumi, chatwoot, discourse, iqvoc,
///   mixpanel_client, and admin_data without regressing the nested-if/unless
///   no-offense cases.
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

        // Match RuboCop's `elsif_conditional?` — the if node must have at least
        // one direct elsif branch. In Prism, elsif branches are IfNode subsequents;
        // else clauses are ElseNode subsequents. Without this check, `if...else; if...; end; end`
        // patterns would be incorrectly treated as case-like (the nested if in else
        // is a separate construct, not an elsif chain).
        match if_node.subsequent() {
            Some(ref sub) if sub.as_if_node().is_some() => {} // has elsif
            _ => return,                                      // no elsif — if-else or standalone if
        }

        // RuboCop uses different walks for branch counting vs. condition collection:
        // `MinBranchesCount` counts ternary else bodies because Parser represents
        // ternaries as if_type nodes, but `branch_conditions` stops before adding
        // ternary predicates. Prism wraps else bodies in ElseNode, so we mirror
        // that split explicitly.
        if count_if_conditional_branches(&if_node) < min_branches {
            return;
        }

        let predicates = collect_branch_conditions(&if_node);

        // Phase 1: Find the target from the first condition
        let target = match with_unwrapped(&predicates[0], &find_target) {
            Some(t) => t,
            None => return,
        };

        // Phase 1.5: Check for regexps with named captures (RuboCop's regexp_with_working_captures?)
        // case/when uses === which doesn't populate named capture local variables,
        // so if-elsif chains using named captures must not be converted.
        for pred in &predicates {
            if with_unwrapped(pred, &regexp_with_working_captures) {
                return;
            }
        }

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

/// RuboCop's `regexp_with_working_captures?`: checks if a condition contains
/// a regexp with named captures used with `=~` or `match`.
/// Named captures work with `=~` (regexp on LHS) and `match` (either side).
/// `match?` does NOT populate named captures, so it's not checked.
fn regexp_with_working_captures(node: &ruby_prism::Node<'_>) -> bool {
    // Handle `||` - check both sides
    if let Some(or_node) = node.as_or_node() {
        return with_unwrapped(&or_node.left(), &regexp_with_working_captures)
            || with_unwrapped(&or_node.right(), &regexp_with_working_captures);
    }

    // In Prism, `/(?<name>.*)/ =~ foo` becomes a MatchWriteNode wrapping a CallNode.
    // Check MatchWriteNode: the call inside is `=~` with regexp on LHS.
    if let Some(mw) = node.as_match_write_node() {
        let call = mw.call();
        if let Some(receiver) = call.receiver() {
            if regexp_has_named_captures(&receiver) {
                return true;
            }
        }
        return false;
    }

    if let Some(call) = node.as_call_node() {
        let method = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        // Only `match` populates named captures, not `match?` or `=~` (as send node)
        if method == "match" {
            if let Some(receiver) = call.receiver() {
                if regexp_has_named_captures(&receiver) {
                    return true;
                }
            }
            let args = call.arguments();
            if let Some(arg) = args.as_ref().and_then(|a| a.arguments().iter().next()) {
                if regexp_has_named_captures(&arg) {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a node is a regexp literal containing named captures (`(?<name>...)` or `(?'name'...)`).
fn regexp_has_named_captures(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(re) = node.as_regular_expression_node() {
        let content = re.unescaped();
        // Look for (?< or (?'  which indicate named captures
        // (?<= and (?<! are lookbehind assertions, not named captures
        let mut i = 0;
        while i + 2 < content.len() {
            if content[i] == b'(' && content[i + 1] == b'?' {
                if i + 3 < content.len() && content[i + 2] == b'<' {
                    // (?<  — named capture unless followed by = or ! (lookbehind)
                    if content[i + 3] != b'=' && content[i + 3] != b'!' {
                        return true;
                    }
                } else if content[i + 2] == b'\'' {
                    // (?'name'...) — named capture
                    return true;
                }
            }
            i += 1;
        }
    }
    false
}

/// Count branches the same way RuboCop's `MinBranchesCount#if_conditional_branches`
/// does for `if` chains. In Parser AST, a ternary else body is an `if_type?`
/// branch and therefore counts toward the threshold. In Prism, the else body is
/// wrapped in an ElseNode, so we unwrap a single nested if/unless node here.
fn count_if_conditional_branches(if_node: &ruby_prism::IfNode<'_>) -> usize {
    1 + if_node
        .subsequent()
        .as_ref()
        .map_or(0, count_else_if_type_branches)
}

fn count_else_if_type_branches(node: &ruby_prism::Node<'_>) -> usize {
    if let Some(elsif) = node.as_if_node() {
        return 1 + elsif
            .subsequent()
            .as_ref()
            .map_or(0, count_else_if_type_branches);
    }

    let else_node = match node.as_else_node() {
        Some(else_node) => else_node,
        None => return 0,
    };
    let child = match single_statement_else_child(&else_node) {
        Some(child) => child,
        None => return 0,
    };

    if let Some(if_node) = child.as_if_node() {
        return 1 + if_node
            .subsequent()
            .as_ref()
            .map_or(0, count_else_if_type_branches);
    }
    if let Some(unless_node) = child.as_unless_node() {
        return 1 + unless_node
            .else_clause()
            .map(|else_clause| count_else_if_type_branches(&else_clause.as_node()))
            .unwrap_or(0);
    }
    0
}

/// Collect branch conditions the same way RuboCop's `branch_conditions` does.
/// Unlike `MinBranchesCount`, this walk must stop before ternaries: Parser treats
/// a ternary as `if_type?`, but `branch_conditions` explicitly rejects it.
fn collect_branch_conditions<'a>(if_node: &ruby_prism::IfNode<'a>) -> Vec<ruby_prism::Node<'a>> {
    let mut predicates = vec![if_node.predicate()];
    let mut current_else = if_node.subsequent();
    while let Some(else_clause) = current_else {
        if let Some(elsif) = else_clause.as_if_node() {
            predicates.push(elsif.predicate());
            current_else = elsif.subsequent();
        } else if let Some((pred, next)) = unwrap_else_to_condition_branch_info(&else_clause) {
            predicates.push(pred);
            current_else = next;
        } else {
            break;
        }
    }
    predicates
}

fn single_statement_else_child<'a>(
    else_node: &ruby_prism::ElseNode<'a>,
) -> Option<ruby_prism::Node<'a>> {
    let stmts = else_node.statements()?;
    let mut children = stmts.body().iter();
    let child = children.next()?;
    if children.next().is_some() {
        return None;
    }
    Some(child)
}

/// If a node is an ElseNode whose body is a single non-ternary if/unless node,
/// return that node's predicate and its continuation (subsequent/else clause) for
/// RuboCop's `branch_conditions` walk.
fn unwrap_else_to_condition_branch_info<'a>(
    node: &ruby_prism::Node<'a>,
) -> Option<(ruby_prism::Node<'a>, Option<ruby_prism::Node<'a>>)> {
    let else_node = node.as_else_node()?;
    let child = single_statement_else_child(&else_node)?;

    if let Some(if_node) = child.as_if_node() {
        if util::is_ternary(&if_node) {
            return None;
        }
        return Some((if_node.predicate(), if_node.subsequent()));
    }
    if let Some(unless_node) = child.as_unless_node() {
        return Some((
            unless_node.predicate(),
            unless_node.else_clause().map(|e| e.as_node()),
        ));
    }
    None
}

/// Get the inner expression from a potentially parenthesized node.
/// Unwraps ParenthesesNode and BeginNode (RuboCop's `deparenthesize` handles
/// both via `:begin` type in Parser AST; in Prism these are separate node types).
/// If the node contains exactly one statement, recurse into it.
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
    // BeginNode: explicit `begin...end` wrapping a condition
    if let Some(begin) = node.as_begin_node() {
        if let Some(stmts) = begin.statements() {
            let children: Vec<_> = stmts.body().iter().collect();
            if children.len() == 1 {
                return with_unwrapped(&children[0], f);
            }
        }
    }
    f(node)
}

/// Check if a node is any kind of regexp (regular or interpolated).
/// In Parser AST, both are `:regexp`. In Prism, they're separate node types.
fn is_regexp_node(node: &ruby_prism::Node<'_>) -> bool {
    node.as_regular_expression_node().is_some()
        || node.as_interpolated_regular_expression_node().is_some()
}

/// Check if a CallNode uses safe navigation (`&.`).
/// RuboCop treats `csend` (safe navigation) as a different node type from `send`,
/// and `find_target_in_send_node`/`condition_from_send_node` only handle `:send`.
fn is_safe_navigation(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(op_loc) = call.call_operator_loc() {
        let op =
            &call.location().as_slice()[op_loc.start_offset() - call.location().start_offset()..];
        op.starts_with(b"&.")
    } else {
        false
    }
}

/// Extract the target from a condition node (RuboCop's `find_target`).
/// Returns the source bytes of the target expression.
fn find_target(node: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
    // Handle `||` - find target from the left side
    if let Some(or_node) = node.as_or_node() {
        return with_unwrapped(&or_node.left(), &find_target);
    }

    if let Some(call) = node.as_call_node() {
        // Skip safe navigation (&.) — RuboCop only handles :send, not :csend
        if is_safe_navigation(&call) {
            return None;
        }
        let method = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        match method {
            "is_a?" => {
                // Target is receiver: x.is_a?(Foo) -> target = x
                // Note: RuboCop only handles is_a?, NOT kind_of?, even though they
                // are aliases. kind_of? calls should not trigger case-when conversion.
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
/// Requires one side to be a regexp literal (regular or interpolated).
fn find_target_in_match_node(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let receiver = call.receiver()?;
    let args = call.arguments();
    let first_arg = args.as_ref().and_then(|a| a.arguments().iter().next());

    // For all match methods: one side must be a regexp, the other is the target
    if let Some(ref arg) = first_arg {
        if is_regexp_node(&receiver) {
            return Some(arg.location().as_slice().to_vec());
        } else if is_regexp_node(arg) {
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
        // Skip safe navigation (&.) — RuboCop only handles :send, not :csend
        if is_safe_navigation(&call) {
            return false;
        }
        let method = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        match method {
            "is_a?" => {
                // Receiver must be the target
                // Note: kind_of? is intentionally excluded — RuboCop only handles is_a?
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
        || is_regexp_node(node)
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
    use crate::testutil::{assert_cop_offenses_full, assert_cop_offenses_full_with_config};
    use std::collections::HashMap;

    crate::cop_fixture_tests!(CaseLikeIf, "cops/style/case_like_if");

    #[test]
    fn honors_min_branches_count_two_for_two_branch_else_chain() {
        let fixture = b"if resource == 'export'\n^^^^^^^^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.\n  1\nelsif resource == 'import'\n  2\nelse\n  3\nend\n";
        let mut options = HashMap::new();
        options.insert(
            "MinBranchesCount".to_string(),
            serde_yml::Value::Number(2.into()),
        );
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };

        assert_cop_offenses_full_with_config(&CaseLikeIf, fixture, config);
    }

    #[test]
    fn counts_ternary_else_for_min_branches_without_validating_its_predicate() {
        let fixture = b"current_val = query_hash['values'][0]\nif query_hash['attribute_key'] == 'phone_number'\n^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.\n  \"+#{current_val&.delete('+')}\"\nelsif query_hash['attribute_key'] == 'country_code'\n  current_val.downcase\nelse\n  current_val.is_a?(String) ? current_val.downcase : current_val\nend\n";

        assert_cop_offenses_full(&CaseLikeIf, fixture);
    }

    #[test]
    fn honors_min_branches_count_two_for_is_a_chain() {
        let fixture = b"if item.is_a?(Label::Base)\n^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.\n  label_path(id: item.origin)\nelsif item.is_a?(Collection::Base)\n  collection_path(id: item.origin)\nelse\n  concept_path(id: item.origin)\nend\n";
        let mut options = HashMap::new();
        options.insert(
            "MinBranchesCount".to_string(),
            serde_yml::Value::Number(2.into()),
        );
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };

        assert_cop_offenses_full_with_config(&CaseLikeIf, fixture, config);
    }

    #[test]
    fn honors_min_branches_count_two_for_regex_match_chain() {
        let fixture = b"if adapter =~ /postgresql/i\n^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.\n  monthly_sql\nelsif adapter =~ /mysql/i\n  mysql_sql\nelse\n  sqlite_sql\nend\n";
        let mut options = HashMap::new();
        options.insert(
            "MinBranchesCount".to_string(),
            serde_yml::Value::Number(2.into()),
        );
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };

        assert_cop_offenses_full_with_config(&CaseLikeIf, fixture, config);
    }
}
