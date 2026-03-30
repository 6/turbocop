use crate::cop::node_type::{CALL_NODE, PARENTHESES_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/ArrayIntersect detects array intersection patterns replaceable
/// with `Array#intersect?` (Ruby 3.1+).
///
/// FN investigation (2026-03-30):
/// - RuboCop also flags block forms like `array1.any? { |e| array2.member?(e) }`
///   and `array1.none? { array2.member?(_1) }`, but the Prism port only checked
///   intersection receivers (`(a & b)` / `a.intersection(b)`). Fixed by matching
///   explicit, numbered, and Ruby 3.4 `it` block parameters.
/// - The `.present?` / `.blank?` family already matches when
///   `ActiveSupportExtensionsEnabled` is present in the cop config. Remaining
///   corpus misses in that family point at config propagation rather than local
///   AST matching in this file.
///
/// Handles four families of patterns:
/// 1. Direct predicates: `(a & b).any?` / `.empty?` / `.none?`
///    (plus `.present?` / `.blank?` when `ActiveSupportExtensionsEnabled`)
/// 2. Size comparisons: `(a & b).count > 0`, `== 0`, `!= 0`
///    (also `.size` and `.length`)
/// 3. Size predicates: `(a & b).count.positive?`, `.count.zero?`
/// 4. Block predicates: `array1.any? { |e| array2.member?(e) }`
///    and the `none?`/`_1`/`it` variants
///
/// Intersection-receiver patterns also match the `a.intersection(b)` form
/// (1 argument only).
pub struct ArrayIntersect;

/// Extract (lhs_source, rhs_source) from an intersection expression node.
/// Matches `(a & b)` (parenthesized `&` call) and `a.intersection(b)` (1-arg form).
fn extract_intersection_parts(node: &ruby_prism::Node<'_>) -> Option<(String, String)> {
    // (a & b) form
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            if let Some(stmts) = body.as_statements_node() {
                let stmt_list: Vec<_> = stmts.body().iter().collect();
                if stmt_list.len() == 1 {
                    if let Some(inner_call) = stmt_list[0].as_call_node() {
                        let m = std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");
                        if m == "&" {
                            let recv = inner_call.receiver()?;
                            let args = inner_call.arguments()?;
                            let arg_list: Vec<_> = args.arguments().iter().collect();
                            if arg_list.len() == 1 {
                                let lhs = std::str::from_utf8(recv.location().as_slice())
                                    .unwrap_or("")
                                    .to_string();
                                let rhs = std::str::from_utf8(arg_list[0].location().as_slice())
                                    .unwrap_or("")
                                    .to_string();
                                return Some((lhs, rhs));
                            }
                        }
                    }
                }
            }
        }
    }
    // a.intersection(b) form
    if let Some(call) = node.as_call_node() {
        let m = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if m == "intersection" {
            let recv = call.receiver()?;
            let args = call.arguments()?;
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() == 1 {
                let lhs = std::str::from_utf8(recv.location().as_slice())
                    .unwrap_or("")
                    .to_string();
                let rhs = std::str::from_utf8(arg_list[0].location().as_slice())
                    .unwrap_or("")
                    .to_string();
                return Some((lhs, rhs));
            }
        }
    }
    None
}

fn single_body_expression<'a>(body: ruby_prism::Node<'a>) -> Option<ruby_prism::Node<'a>> {
    if let Some(stmts) = body.as_statements_node() {
        let mut stmt_iter = stmts.body().iter();
        let first = stmt_iter.next()?;
        if stmt_iter.next().is_some() {
            None
        } else {
            Some(first)
        }
    } else {
        Some(body)
    }
}

fn explicit_block_param_matches(params: &ruby_prism::BlockParametersNode<'_>) -> Option<Vec<u8>> {
    let inner = params.parameters()?;
    let requireds: Vec<_> = inner.requireds().iter().collect();
    if requireds.len() != 1
        || !inner.optionals().is_empty()
        || inner.rest().is_some()
        || !inner.posts().is_empty()
        || !inner.keywords().is_empty()
        || inner.keyword_rest().is_some()
        || inner.block().is_some()
    {
        return None;
    }

    requireds[0]
        .as_required_parameter_node()
        .map(|param| param.name().as_slice().to_vec())
}

fn extract_member_block_parts(
    call: &ruby_prism::CallNode<'_>,
    ruby_version: f64,
) -> Option<(String, String, String)> {
    let receiver = call.receiver()?;
    let block_node = call.block()?.as_block_node()?;
    let params = block_node.parameters()?;
    let body = block_node.body()?;
    let body_expr = single_body_expression(body)?;
    let member_call = body_expr.as_call_node()?;

    if member_call.name().as_slice() != b"member?" {
        return None;
    }

    let member_receiver = member_call.receiver()?;
    let member_args = member_call.arguments()?;
    let member_arg_list: Vec<_> = member_args.arguments().iter().collect();
    if member_arg_list.len() != 1 {
        return None;
    }

    let arg_matches = if let Some(block_params) = params.as_block_parameters_node() {
        let param_name = explicit_block_param_matches(&block_params)?;
        member_arg_list[0]
            .as_local_variable_read_node()
            .is_some_and(|arg| arg.name().as_slice() == param_name)
    } else if params.as_numbered_parameters_node().is_some() {
        member_arg_list[0]
            .as_local_variable_read_node()
            .is_some_and(|arg| arg.name().as_slice() == b"_1")
    } else if ruby_version >= 3.4 && params.as_it_parameters_node().is_some() {
        member_arg_list[0]
            .as_it_local_variable_read_node()
            .is_some()
    } else {
        false
    };

    if !arg_matches {
        return None;
    }

    let recv = std::str::from_utf8(receiver.location().as_slice())
        .unwrap_or("")
        .to_string();
    let op = call
        .call_operator_loc()
        .and_then(|loc| std::str::from_utf8(loc.as_slice()).ok())
        .unwrap_or(".")
        .to_string();
    let arg_receiver = std::str::from_utf8(member_receiver.location().as_slice())
        .unwrap_or("")
        .to_string();

    Some((recv, op, arg_receiver))
}

impl Cop for ArrayIntersect {
    fn name(&self) -> &'static str {
        "Style/ArrayIntersect"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, PARENTHESES_NODE, STATEMENTS_NODE]
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
        // intersect? requires Ruby >= 3.1
        let ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(3.4);
        if ruby_version < 3.1 {
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");

        let active_support = config.get_bool("ActiveSupportExtensionsEnabled", false);

        // Pattern 4: array1.any? { |e| array2.member?(e) } / none? variants
        if matches!(method_name, "any?" | "none?")
            && call.arguments().is_none()
            && call.block().is_some()
        {
            if let Some((recv, op, arg_receiver)) = extract_member_block_parts(&call, ruby_version)
            {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let existing = source.byte_slice(
                    loc.start_offset(),
                    call.block().unwrap().location().end_offset(),
                    "array1.any? { |e| array2.member?(e) }",
                );
                let bang = if method_name == "none?" { "!" } else { "" };
                let msg = format!(
                    "Use `{bang}{recv}{op}intersect?({arg_receiver})` instead of `{existing}`."
                );
                diagnostics.push(self.diagnostic(source, line, column, msg));
                return;
            }
        }

        // Pattern 1: (a & b).any? / .empty? / .none? / .present? / .blank?
        if matches!(method_name, "any?" | "empty?" | "none?")
            || (active_support && matches!(method_name, "present?" | "blank?"))
        {
            // Skip if the call has arguments or a block (any? with block)
            if call.arguments().is_some() || call.block().is_some() {
                return;
            }

            if let Some(receiver) = call.receiver() {
                // Check for parenthesized expression containing &
                if let Some(paren) = receiver.as_parentheses_node() {
                    if let Some(body) = paren.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            let stmt_list: Vec<_> = stmts.body().iter().collect();
                            if stmt_list.len() == 1 {
                                if let Some(inner_call) = stmt_list[0].as_call_node() {
                                    let inner_method =
                                        std::str::from_utf8(inner_call.name().as_slice())
                                            .unwrap_or("");
                                    if inner_method == "&" {
                                        let loc = node.location();
                                        let (line, column) =
                                            source.offset_to_line_col(loc.start_offset());

                                        // Keep backward-compatible message for original patterns
                                        let msg =
                                            if matches!(method_name, "any?" | "empty?" | "none?") {
                                                format!(
                                                    "Use `intersect?` instead of `({}).{}`.",
                                                    std::str::from_utf8(
                                                        inner_call.location().as_slice()
                                                    )
                                                    .unwrap_or("array1 & array2"),
                                                    method_name
                                                )
                                            } else if let Some((lhs, rhs)) =
                                                extract_intersection_parts(&receiver)
                                            {
                                                let existing = std::str::from_utf8(loc.as_slice())
                                                    .unwrap_or("");
                                                format!(
                                                    "Use `{}.intersect?({})` instead of `{}`.",
                                                    lhs, rhs, existing
                                                )
                                            } else {
                                                let existing = std::str::from_utf8(loc.as_slice())
                                                    .unwrap_or("");
                                                format!(
                                                    "Use `intersect?` instead of `{}`.",
                                                    existing
                                                )
                                            };
                                        diagnostics
                                            .push(self.diagnostic(source, line, column, msg));
                                    }
                                }
                            }
                        }
                    }
                }

                // Check for a.intersection(b).any? / .empty? / .none?
                if let Some(recv_call) = receiver.as_call_node() {
                    let recv_method =
                        std::str::from_utf8(recv_call.name().as_slice()).unwrap_or("");
                    if recv_method == "intersection" {
                        // Must have exactly 1 argument and a receiver
                        if recv_call.receiver().is_some() {
                            if let Some(args) = recv_call.arguments() {
                                let arg_list: Vec<_> = args.arguments().iter().collect();
                                if arg_list.len() == 1 {
                                    let loc = node.location();
                                    let (line, column) =
                                        source.offset_to_line_col(loc.start_offset());

                                    let msg = if matches!(method_name, "any?" | "empty?" | "none?")
                                    {
                                        format!(
                                            "Use `intersect?` instead of `intersection(...).{}`.",
                                            method_name
                                        )
                                    } else if let Some((lhs, rhs)) =
                                        extract_intersection_parts(&receiver)
                                    {
                                        let existing =
                                            std::str::from_utf8(loc.as_slice()).unwrap_or("");
                                        format!(
                                            "Use `{}.intersect?({})` instead of `{}`.",
                                            lhs, rhs, existing
                                        )
                                    } else {
                                        let existing =
                                            std::str::from_utf8(loc.as_slice()).unwrap_or("");
                                        format!("Use `intersect?` instead of `{}`.", existing)
                                    };
                                    diagnostics.push(self.diagnostic(source, line, column, msg));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Pattern 2: (a & b).count > 0 / == 0 / != 0
        if matches!(method_name, ">" | "==" | "!=") {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() == 1 {
                    if let Some(int_node) = arg_list[0].as_integer_node() {
                        if int_node.location().as_slice() == b"0" {
                            if let Some(recv) = call.receiver() {
                                if let Some(recv_call) = recv.as_call_node() {
                                    let rm = std::str::from_utf8(recv_call.name().as_slice())
                                        .unwrap_or("");
                                    if matches!(rm, "count" | "size" | "length")
                                        && recv_call.arguments().is_none()
                                        && recv_call.block().is_none()
                                    {
                                        if let Some(inner_recv) = recv_call.receiver() {
                                            if let Some((lhs, rhs)) =
                                                extract_intersection_parts(&inner_recv)
                                            {
                                                let loc = node.location();
                                                let (line, column) =
                                                    source.offset_to_line_col(loc.start_offset());
                                                let existing = std::str::from_utf8(loc.as_slice())
                                                    .unwrap_or("");
                                                let msg = format!(
                                                    "Use `{}.intersect?({})` instead of `{}`.",
                                                    lhs, rhs, existing
                                                );
                                                diagnostics.push(
                                                    self.diagnostic(source, line, column, msg),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Pattern 3: (a & b).count.positive? / .zero?
        if matches!(method_name, "positive?" | "zero?")
            && call.arguments().is_none()
            && call.block().is_none()
        {
            if let Some(recv) = call.receiver() {
                if let Some(recv_call) = recv.as_call_node() {
                    let rm = std::str::from_utf8(recv_call.name().as_slice()).unwrap_or("");
                    if matches!(rm, "count" | "size" | "length")
                        && recv_call.arguments().is_none()
                        && recv_call.block().is_none()
                    {
                        if let Some(inner_recv) = recv_call.receiver() {
                            if let Some((lhs, rhs)) = extract_intersection_parts(&inner_recv) {
                                let loc = node.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                let existing = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                                let msg = format!(
                                    "Use `{}.intersect?({})` instead of `{}`.",
                                    lhs, rhs, existing
                                );
                                diagnostics.push(self.diagnostic(source, line, column, msg));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ArrayIntersect, "cops/style/array_intersect");

    #[test]
    fn present_with_active_support() {
        let config = {
            let mut c = CopConfig::default();
            c.options.insert(
                "ActiveSupportExtensionsEnabled".to_string(),
                serde_yml::Value::Bool(true),
            );
            c
        };
        let diags = crate::testutil::run_cop_full_with_config(
            &ArrayIntersect,
            b"(a & b).present?\n",
            config,
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(
            diags[0].message,
            "Use `a.intersect?(b)` instead of `(a & b).present?`."
        );
    }

    #[test]
    fn blank_with_active_support() {
        let config = {
            let mut c = CopConfig::default();
            c.options.insert(
                "ActiveSupportExtensionsEnabled".to_string(),
                serde_yml::Value::Bool(true),
            );
            c
        };
        let diags =
            crate::testutil::run_cop_full_with_config(&ArrayIntersect, b"(a & b).blank?\n", config);
        assert_eq!(diags.len(), 1);
        assert_eq!(
            diags[0].message,
            "Use `a.intersect?(b)` instead of `(a & b).blank?`."
        );
    }

    #[test]
    fn present_without_active_support_is_ok() {
        let diags = crate::testutil::run_cop_full(&ArrayIntersect, b"(a & b).present?\n");
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn intersection_present_with_active_support() {
        let config = {
            let mut c = CopConfig::default();
            c.options.insert(
                "ActiveSupportExtensionsEnabled".to_string(),
                serde_yml::Value::Bool(true),
            );
            c
        };
        let diags = crate::testutil::run_cop_full_with_config(
            &ArrayIntersect,
            b"a.intersection(b).present?\n",
            config,
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(
            diags[0].message,
            "Use `a.intersect?(b)` instead of `a.intersection(b).present?`."
        );
    }

    #[test]
    fn nested_present_with_active_support() {
        let config = {
            let mut c = CopConfig::default();
            c.options.insert(
                "ActiveSupportExtensionsEnabled".to_string(),
                serde_yml::Value::Bool(true),
            );
            c
        };
        let diags = crate::testutil::run_cop_full_with_config(
            &ArrayIntersect,
            b"(cost_keys.to_set & report_cols).present?\n",
            config,
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(
            diags[0].message,
            "Use `cost_keys.to_set.intersect?(report_cols)` instead of `(cost_keys.to_set & report_cols).present?`."
        );
    }

    #[test]
    fn any_block_member_detection() {
        let diags = crate::testutil::run_cop_full(
            &ArrayIntersect,
            b"array1.any? { |e| array2.member?(e) }\n",
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(
            diags[0].message,
            "Use `array1.intersect?(array2)` instead of `array1.any? { |e| array2.member?(e) }`."
        );
    }

    #[test]
    fn none_block_member_detection_with_numbered_params() {
        let diags = crate::testutil::run_cop_full(
            &ArrayIntersect,
            b"array1.none? { array2.member?(_1) }\n",
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(
            diags[0].message,
            "Use `!array1.intersect?(array2)` instead of `array1.none? { array2.member?(_1) }`."
        );
    }
}
