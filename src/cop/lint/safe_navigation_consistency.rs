use crate::cop::shared::node_type::{AND_NODE, OR_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

const DEFAULT_ALLOWED_METHODS: &[&str] = &["present?", "blank?", "presence", "try", "try!"];

/// Lint/SafeNavigationConsistency checks that if safe navigation (`&.`) is used
/// in an `&&` or `||` condition, consistent and appropriate navigation is used
/// for all method calls on the same object.
///
/// ## Implementation notes
///
/// Mirrors RuboCop's approach: flatten entire `&&`/`||` chains by recursing into
/// nested and/or nodes, collecting all leaf call operands. Each operand is tagged
/// with whether it appears in an `&&` context or `||` context (determined by the
/// immediate parent logical operator). Operands are grouped by receiver name,
/// then `find_consistent_parts` determines the expected call operator (`.` or `&.`)
/// based on the leftmost csend/send positions in and/or contexts. Remaining
/// operands that don't match the expected operator get an offense.
///
/// The `nilable?` check (csend or nil_methods including AllowedMethods) excludes
/// certain operands from establishing the "first regular send" baseline, matching
/// RuboCop's `NilMethods` mixin behavior.
/// ## Corpus investigation (2026-03-14)
///
/// Corpus oracle reported FP=3, FN=0. All 3 FPs are config/exclude differences:
///
/// - consuldemocracy: cop disabled in project config
/// - excon: cop disabled/excluded in project config
/// - rubocop: cop disabled for own source files
///
/// Verified by reading vendor RuboCop source — the cop logic matches exactly
/// (same find_consistent_parts algorithm). No cop logic bugs.
///
/// ## Corpus investigation (2026-03-15)
///
/// The remaining FP=3 were not config issues. The corpus oracle uses the shared
/// baseline config, so repo-local disables do not apply.
///
/// Root cause:
/// - The simplified operand collector merged safe-navigation calls from
///   separate `&&` branches inside a larger `||` expression, treating them as a
///   single `and` group for the same receiver. That produced false positives for
///   shapes like `(foo&.a && x) || (foo&.b && y) || (foo&.c && z)` and
///   `foo&.a || foo&.b || (foo&.c && flag) || foo&.d`.
///
/// Fix:
/// - Track the nearest `&&` group for each operand and suppress the "all csend
///   in and-context" heuristic when the grouped operands come from distinct
///   `&&` branches and no regular send establishes a baseline.
///
/// ## Corpus investigation (2026-03-24)
///
/// FP=4 (tagteam=2, brick=1, 18xx=1). brick and 18xx no longer reproduce with
/// current binary. tagteam FP root cause: Prism represents block calls
/// (e.g. `foo.bar { }`) as CallNode with a block field, while Parser gem wraps
/// them in a `(block (send ...))` node. RuboCop's `operand_nodes` only collects
/// `call_type?` nodes, so block-wrapped calls are excluded. Fix: skip CallNodes
/// with `call.block().is_some()` in `extract_operand_info`.
///
/// ## Corpus investigation (2026-03-25)
///
/// FP=2 (brick=1, 18xx=1). Two root causes:
///
/// 1. **brick FP** (`(A&.b && C) || (A&.d && E)`): nitrocop recursed into
///    parenthesized expressions, merging operands from separate `&&` groups.
///    RuboCop's `operand_nodes` only recurses into `operator_keyword?` nodes
///    (and/or), NOT `begin` nodes (parentheses). Fix: remove ParenthesesNode
///    recursion from `collect_operands_from_node`.
///
/// 2. **18xx FP** (`idx&.positive? && idx&.<(n)`): nitrocop set
///    `is_operator_method` based on absence of a call operator (`&.`/`.`),
///    but RuboCop's `operator_method?` is based purely on the method name
///    (e.g. `<`, `+`, `==`). A call like `foo&.<(bar)` has `operator_method?`
///    = true in RuboCop, so `already_appropriate_call?` returns true when
///    expected is `.`. Fix: determine `is_operator_method` from the method
///    name and update `already_appropriate` to match RuboCop's logic.
pub struct SafeNavigationConsistency;

impl Cop for SafeNavigationConsistency {
    fn name(&self) -> &'static str {
        "Lint/SafeNavigationConsistency"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[AND_NODE, OR_NODE]
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
        let allowed_methods = config
            .get_string_array("AllowedMethods")
            .unwrap_or_else(|| {
                DEFAULT_ALLOWED_METHODS
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            });

        let is_and = node.as_and_node().is_some();
        let (left, right) = if let Some(and_node) = node.as_and_node() {
            (and_node.left(), and_node.right())
        } else if let Some(or_node) = node.as_or_node() {
            (or_node.left(), or_node.right())
        } else {
            return;
        };

        // Collect all operands by flattening nested and/or chains
        let mut operands = Vec::new();
        let root_and_group = if is_and {
            Some((node.location().start_offset(), node.location().end_offset()))
        } else {
            None
        };
        collect_operands_from_node(&left, is_and, root_and_group, &mut operands);
        collect_operands_from_node(&right, is_and, root_and_group, &mut operands);

        // Group operands by receiver name
        let mut groups: std::collections::HashMap<String, Vec<&OperandInfo>> =
            std::collections::HashMap::new();
        for op in &operands {
            groups.entry(op.receiver_name.clone()).or_default().push(op);
        }

        // Check each group for consistency
        for grouped in groups.values() {
            if let Some((expected_op, begin_idx)) = find_consistent_parts(grouped, &allowed_methods)
            {
                for op in &grouped[begin_idx..] {
                    if already_appropriate(op, &expected_op) {
                        continue;
                    }
                    let (line, column) = if expected_op == "." {
                        // Offense is at the &. operator
                        source.offset_to_line_col(op.call_operator_offset)
                    } else if op.is_operator_method {
                        // For operator methods like `foo + 1`, highlight the whole expression
                        source.offset_to_line_col(op.receiver_offset)
                    } else {
                        // Offense is at the . operator
                        source.offset_to_line_col(op.call_operator_offset)
                    };
                    let message = if expected_op == "." {
                        "Use `.` instead of unnecessary `&.`."
                    } else {
                        "Use `&.` for consistency with safe navigation."
                    };
                    // Deduplicate: both && and || handlers in a chain may fire
                    // on the same operand. RuboCop deduplicates at the collector
                    // level; we deduplicate here by checking existing diagnostics.
                    let already_reported = diagnostics
                        .iter()
                        .any(|d| d.location.line == line && d.location.column == column);
                    if !already_reported {
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            message.to_string(),
                        ));
                    }
                }
            }
        }
    }
}

/// Info about a single operand (call node) in the flattened chain.
struct OperandInfo {
    receiver_name: String,
    method_name: String,
    is_safe_nav: bool,
    is_in_and: bool,
    and_group_key: Option<(usize, usize)>,
    is_operator_method: bool,
    call_operator_offset: usize,
    receiver_offset: usize,
}

/// Recursively collect call-node operands from a node, flattening nested and/or.
fn collect_operands_from_node<'a>(
    node: &ruby_prism::Node<'a>,
    parent_is_and: bool,
    and_group_key: Option<(usize, usize)>,
    operands: &mut Vec<OperandInfo>,
) {
    if let Some(and_node) = node.as_and_node() {
        let group_key = if parent_is_and {
            and_group_key.or(Some((
                and_node.location().start_offset(),
                and_node.location().end_offset(),
            )))
        } else {
            Some((
                and_node.location().start_offset(),
                and_node.location().end_offset(),
            ))
        };
        collect_operands_from_node(&and_node.left(), true, group_key, operands);
        collect_operands_from_node(&and_node.right(), true, group_key, operands);
    } else if let Some(or_node) = node.as_or_node() {
        collect_operands_from_node(&or_node.left(), false, None, operands);
        collect_operands_from_node(&or_node.right(), false, None, operands);
    } else if let Some(info) = extract_operand_info(node, parent_is_and, and_group_key) {
        operands.push(info);
    }
}

fn extract_operand_info(
    node: &ruby_prism::Node<'_>,
    is_in_and: bool,
    and_group_key: Option<(usize, usize)>,
) -> Option<OperandInfo> {
    let call = node.as_call_node()?;
    // Skip calls with blocks — RuboCop's Parser gem wraps these in a block node
    // which is not call_type?, so they are never collected as operands.
    // Prism represents them as CallNode with a block field.
    if call.block().is_some() {
        return None;
    }
    let recv = call.receiver()?;

    let receiver_name = get_receiver_source(&recv)?;

    let method_name = std::str::from_utf8(call.name().as_slice())
        .unwrap_or("")
        .to_string();

    let call_op = call.call_operator_loc();
    let is_safe_nav = call_op
        .as_ref()
        .map(|loc| {
            let src = loc.as_slice();
            src.len() >= 2 && src[0] == b'&' && src[1] == b'.'
        })
        .unwrap_or(false);

    // RuboCop's operator_method? is based on the method name, not on whether
    // a call operator (./&.) is present. foo&.<(bar) has operator_method?=true.
    let is_operator_method = is_ruby_operator_method(&method_name);

    let call_operator_offset = call_op.as_ref().map(|loc| loc.start_offset()).unwrap_or(0);
    let receiver_offset = recv.location().start_offset();

    Some(OperandInfo {
        receiver_name,
        method_name,
        is_safe_nav,
        is_in_and,
        and_group_key,
        is_operator_method,
        call_operator_offset,
        receiver_offset,
    })
}

/// Get the source representation of a receiver for grouping.
/// Simple local variables and bare method calls return their name.
/// For chained calls (x.foo), returns the full source text.
fn get_receiver_source(node: &ruby_prism::Node<'_>) -> Option<String> {
    if let Some(read) = node.as_local_variable_read_node() {
        return Some(
            std::str::from_utf8(read.name().as_slice())
                .unwrap_or("")
                .to_string(),
        );
    }
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none() && call.arguments().is_none() {
            return Some(
                std::str::from_utf8(call.name().as_slice())
                    .unwrap_or("")
                    .to_string(),
            );
        }
        // Chained receiver like x.foo - use full source
        return Some(
            std::str::from_utf8(node.location().as_slice())
                .unwrap_or("")
                .to_string(),
        );
    }
    None
}

/// Determine if an operand is "nilable" - csend or nil-method or allowed method.
/// Nilable operands don't count as "regular sends" for baseline determination.
/// Matches RuboCop's `NilMethods` mixin: `nil.methods + [:to_d] + allowed_methods`.
fn is_nilable(op: &OperandInfo, allowed_methods: &[String]) -> bool {
    if op.is_safe_nav {
        return true;
    }
    if is_nil_method(&op.method_name) {
        return true;
    }
    if allowed_methods.iter().any(|m| m == &op.method_name) {
        return true;
    }
    false
}

/// Methods that nil responds to (NilClass + Object + Kernel + BasicObject).
/// This list mirrors Ruby's `nil.methods` which RuboCop uses via the NilMethods mixin.
fn is_nil_method(name: &str) -> bool {
    matches!(
        name,
        "nil?"
            | "!"
            | "!="
            | "!~"
            | "&"
            | "<=>"
            | "=="
            | "==="
            | "=~"
            | "^"
            | "|"
            | "__id__"
            | "__send__"
            | "class"
            | "clone"
            | "define_singleton_method"
            | "display"
            | "dup"
            | "enum_for"
            | "eql?"
            | "equal?"
            | "extend"
            | "freeze"
            | "frozen?"
            | "hash"
            | "inspect"
            | "instance_eval"
            | "instance_exec"
            | "instance_of?"
            | "instance_variable_defined?"
            | "instance_variable_get"
            | "instance_variable_set"
            | "instance_variables"
            | "is_a?"
            | "itself"
            | "kind_of?"
            | "method"
            | "methods"
            | "object_id"
            | "private_methods"
            | "protected_methods"
            | "public_method"
            | "public_methods"
            | "public_send"
            | "rationalize"
            | "remove_instance_variable"
            | "respond_to?"
            | "send"
            | "singleton_class"
            | "singleton_method"
            | "singleton_methods"
            | "tap"
            | "then"
            | "to_a"
            | "to_c"
            | "to_d"
            | "to_enum"
            | "to_f"
            | "to_h"
            | "to_i"
            | "to_r"
            | "to_s"
            | "yield_self"
    )
}

/// Mirrors RuboCop's `find_consistent_parts`. Returns `(expected_op, begin_index)`.
/// `expected_op` is "." or "&." — what the remaining operands should use.
/// `begin_index` is the index into the grouped operands from which to start checking.
fn find_consistent_parts(
    grouped: &[&OperandInfo],
    allowed_methods: &[String],
) -> Option<(String, usize)> {
    // Find the leftmost indices of csend/send in and/or contexts
    let mut csend_in_and: Option<usize> = None;
    let mut csend_in_or: Option<usize> = None;
    let mut send_in_and: Option<usize> = None;
    let mut send_in_or: Option<usize> = None;

    for (i, op) in grouped.iter().enumerate() {
        if op.is_in_and && op.is_safe_nav && csend_in_and.is_none() {
            csend_in_and = Some(i);
        }
        if !op.is_in_and && op.is_safe_nav && csend_in_or.is_none() {
            csend_in_or = Some(i);
        }
        if op.is_in_and && !is_nilable(op, allowed_methods) && send_in_and.is_none() {
            send_in_and = Some(i);
        }
        if !op.is_in_and && !is_nilable(op, allowed_methods) && send_in_or.is_none() {
            send_in_or = Some(i);
        }
    }

    if send_in_and.is_none() && send_in_or.is_none() {
        let and_groups = grouped
            .iter()
            .filter(|op| op.is_in_and && op.is_safe_nav)
            .filter_map(|op| op.and_group_key)
            .collect::<std::collections::HashSet<_>>();

        if (csend_in_and.is_some() && csend_in_or.is_some()) || and_groups.len() > 1 {
            return None;
        }
    }

    // If csend appears in both && and || contexts, and the && one comes first, bail
    if let (Some(ca), Some(co)) = (csend_in_and, csend_in_or) {
        if ca < co {
            return None;
        }
    }

    if let Some(ca) = csend_in_and {
        // csend in && context: expect "."
        let begin = if let Some(sa) = send_in_and {
            sa.min(ca) + 1
        } else {
            ca + 1
        };
        Some((".".to_string(), begin))
    } else if let (Some(so), Some(co)) = (send_in_or, csend_in_or) {
        // Both send and csend in || context
        if so < co {
            Some((".".to_string(), so + 1))
        } else {
            Some(("&.".to_string(), co + 1))
        }
    } else if let (Some(sa), Some(co)) = (send_in_and, csend_in_or) {
        if sa < co {
            Some((".".to_string(), co))
        } else {
            None
        }
    } else {
        None
    }
}

/// Check if a method name is a Ruby operator method.
/// Matches RuboCop-AST's `operator_method?` predicate.
fn is_ruby_operator_method(name: &str) -> bool {
    matches!(
        name,
        "+" | "-"
            | "*"
            | "/"
            | "%"
            | "**"
            | "<"
            | ">"
            | "<="
            | ">="
            | "=="
            | "!="
            | "==="
            | "<=>"
            | "<<"
            | ">>"
            | "&"
            | "|"
            | "^"
            | "~"
            | "=~"
            | "!~"
            | "!"
            | "[]"
            | "[]="
    )
}

/// Check if an operand already uses the appropriate call style.
/// Mirrors RuboCop's `already_appropriate_call?`:
///   operand.safe_navigation? && dot_op == '&.'  ||
///   (operand.dot? || operand.operator_method?) && dot_op == '.'
fn already_appropriate(op: &OperandInfo, expected_op: &str) -> bool {
    if op.is_safe_nav && expected_op == "&." {
        return true;
    }
    if (!op.is_safe_nav || op.is_operator_method) && expected_op == "." {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SafeNavigationConsistency,
        "cops/lint/safe_navigation_consistency"
    );
}
