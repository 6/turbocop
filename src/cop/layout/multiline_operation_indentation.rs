use crate::cop::node_type::{AND_NODE, CALL_NODE, OR_NODE};
use crate::cop::util::indentation_of;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineOperationIndentation;

const OPERATOR_METHODS: &[&[u8]] = &[
    b"+", b"-", b"*", b"/", b"%", b"**", b"==", b"!=", b"<", b">", b"<=", b">=", b"<=>", b"&",
    b"|", b"^", b"<<", b">>",
];

impl Cop for MultilineOperationIndentation {
    fn name(&self) -> &'static str {
        "Layout/MultilineOperationIndentation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[AND_NODE, CALL_NODE, OR_NODE]
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
        let style = config.get_str("EnforcedStyle", "aligned");

        // Check CallNode with operator methods (binary operators are parsed as calls)
        if let Some(call_node) = node.as_call_node() {
            let method_name = call_node.name().as_slice();

            if !OPERATOR_METHODS.contains(&method_name) {
                return;
            }

            // Skip if inside a grouped expression or method call arg list parentheses.
            // Matches RuboCop's not_for_this_cop? check for operator method calls.
            if is_inside_parentheses(source, node) {
                return;
            }

            let receiver = match call_node.receiver() {
                Some(r) => r,
                None => return,
            };

            let args_node = match call_node.arguments() {
                Some(a) => a,
                None => return,
            };

            let args: Vec<_> = args_node.arguments().iter().collect();
            if args.is_empty() {
                return;
            }

            let recv_loc = receiver.location();
            let (recv_start_line, _) = source.offset_to_line_col(recv_loc.start_offset());
            let (recv_end_line, _) = source.offset_to_line_col(recv_loc.end_offset());
            let first_arg = &args[0];
            let arg_loc = first_arg.location();
            let (arg_line, arg_col) = source.offset_to_line_col(arg_loc.start_offset());

            // Only check multiline operations: the arg must be on a
            // different line than where the receiver ENDS (not starts).
            // For `end + tag.hr`, receiver ends at `end` on the same line as `tag.hr`.
            if arg_line == recv_end_line {
                return;
            }

            let width = config.get_usize("IndentationWidth", 2);

            let recv_line_bytes = source.lines().nth(recv_start_line - 1).unwrap_or(b"");
            let recv_indent = indentation_of(recv_line_bytes);
            let expected_indented = recv_indent + width;
            let expected = match style {
                "aligned" => {
                    // Align with the receiver's column
                    let (_, recv_col) = source.offset_to_line_col(recv_loc.start_offset());
                    recv_col
                }
                _ => expected_indented, // "indented" (default)
            };

            // RuboCop's `kw_node_with_special_indentation` doubles the
            // indentation width when the operation is inside a keyword expression
            // (return, if, while, etc.).
            let kw_expected = if is_in_keyword_condition(source, recv_start_line) {
                Some(recv_indent + 2 * width)
            } else {
                None
            };

            let right_line_bytes = source.lines().nth(arg_line - 1).unwrap_or(b"");
            let line_indent = indentation_of(right_line_bytes);

            // For "aligned" style, RuboCop accepts both aligned and properly
            // indented forms in non-condition contexts (assignments, method args).
            let is_ok = if style == "aligned" {
                arg_col == expected
                    || arg_col == expected_indented
                    || line_indent == expected_indented
                    || arg_col == recv_indent
                    || kw_expected.is_some_and(|kw| arg_col == kw || line_indent == kw)
            } else {
                arg_col == expected
                    || arg_col == recv_indent
                    || kw_expected.is_some_and(|kw| arg_col == kw || line_indent == kw)
            };

            if !is_ok {
                diagnostics.push(self.diagnostic(
                    source,
                    arg_line,
                    arg_col,
                    format!(
                        "Use {} (not {}) spaces for indentation of a continuation line.",
                        width,
                        arg_col.saturating_sub(recv_indent)
                    ),
                ));
            }
        }

        // Check AndNode
        if let Some(and_node) = node.as_and_node() {
            // Skip if inside a grouped expression (parentheses) or method call
            // arg list parentheses — matches RuboCop's not_for_this_cop? check.
            if is_inside_parentheses(source, node) {
                return;
            }
            diagnostics.extend(self.check_binary_node(
                source,
                &and_node.left(),
                &and_node.right(),
                config,
                style,
            ));
            return;
        }

        // Check OrNode
        if let Some(or_node) = node.as_or_node() {
            // Skip if inside a grouped expression or method call arg list parentheses
            if is_inside_parentheses(source, node) {
                return;
            }
            diagnostics.extend(self.check_binary_node(
                source,
                &or_node.left(),
                &or_node.right(),
                config,
                style,
            ));
        }
    }
}

/// Check if a node is enclosed by parentheses by scanning the source.
/// This matches RuboCop's `not_for_this_cop?` which skips and/or nodes inside
/// grouped expressions `(expr)` or method call arg list parentheses `foo(expr)`.
///
/// We scan backwards from the node's start offset counting unbalanced parens.
/// If we find an unmatched `(` that is also balanced by a `)` after the node's
/// end, the node is inside parentheses.
fn is_inside_parentheses(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    let bytes = source.as_bytes();
    let node_start = node.location().start_offset();
    let node_end = node.location().end_offset();

    // Scan backwards from node_start to find unmatched '('
    let mut depth = 0i32;
    let mut pos = node_start;
    while pos > 0 {
        pos -= 1;
        match bytes[pos] {
            b')' => depth += 1,
            b'(' => {
                if depth > 0 {
                    depth -= 1;
                } else {
                    // Found an unmatched '(' before the node.
                    // Now verify there's a matching ')' after the node.
                    let mut fwd_depth = 0i32;
                    for &b in &bytes[node_end..] {
                        match b {
                            b'(' => fwd_depth += 1,
                            b')' => {
                                if fwd_depth > 0 {
                                    fwd_depth -= 1;
                                } else {
                                    return true;
                                }
                            }
                            _ => {}
                        }
                    }
                    return false;
                }
            }
            // Don't cross method/class/module boundaries
            b'\n' => {
                // Check if this line starts a method/class def (rough check)
                // We allow scanning through multiple lines within a single expression.
            }
            _ => {}
        }
    }
    false
}

/// Check if the given line starts with a keyword that creates a condition
/// context (if, elsif, unless, while, until, return, for). RuboCop's
/// `kw_node_with_special_indentation` doubles the indentation width for
/// operations inside such conditions.
fn is_in_keyword_condition(source: &SourceFile, line: usize) -> bool {
    let line_bytes = source.lines().nth(line - 1).unwrap_or(b"");
    let trimmed: &[u8] = {
        let start = line_bytes
            .iter()
            .position(|&b| b != b' ' && b != b'\t')
            .unwrap_or(line_bytes.len());
        &line_bytes[start..]
    };
    trimmed.starts_with(b"if ")
        || trimmed.starts_with(b"elsif ")
        || trimmed.starts_with(b"unless ")
        || trimmed.starts_with(b"while ")
        || trimmed.starts_with(b"until ")
        || trimmed.starts_with(b"return ")
        || trimmed.starts_with(b"for ")
}

impl MultilineOperationIndentation {
    fn check_binary_node(
        &self,
        source: &SourceFile,
        left: &ruby_prism::Node<'_>,
        right: &ruby_prism::Node<'_>,
        config: &CopConfig,
        style: &str,
    ) -> Vec<Diagnostic> {
        let (left_line, left_col) = source.offset_to_line_col(left.location().start_offset());
        let (left_end_line, _) = source.offset_to_line_col(left.location().end_offset());
        let (right_line, right_col) = source.offset_to_line_col(right.location().start_offset());

        // Use end of left operand for same-line check. For chained ||/&&
        // like `a || b || c`, the outer Or has left=Or(a,b) spanning lines
        // but `c` may be on the same line as `b` (the end of the left subtree).
        if right_line == left_end_line {
            return Vec::new();
        }

        // Skip nested boolean expressions: when the left operand is itself
        // an And/Or node, alignment expectations get complex and the inner
        // operation is already checked separately.
        if left.as_and_node().is_some() || left.as_or_node().is_some() {
            return Vec::new();
        }

        let width = config.get_usize("IndentationWidth", 2);

        let left_line_bytes = source.lines().nth(left_line - 1).unwrap_or(b"");
        let left_indent = indentation_of(left_line_bytes);
        let expected_indented = left_indent + width;
        let expected = match style {
            "aligned" => left_col,
            _ => expected_indented, // "indented" (default)
        };

        // When the continuation line starts with the operator (leading operator
        // style), check the line's indentation rather than the right operand's.
        let right_line_bytes = source.lines().nth(right_line - 1).unwrap_or(b"");
        let line_indent = indentation_of(right_line_bytes);

        // RuboCop's `kw_node_with_special_indentation` doubles the indentation
        // width when the operation is in a keyword condition (if/elsif/unless/
        // while/until/return). For `indented` style, accept both normal and
        // double-width indentation when in such a context.
        let kw_expected = if is_in_keyword_condition(source, left_line) {
            Some(left_indent + 2 * width)
        } else {
            None
        };

        // For "aligned" style, accept both aligned and indented forms.
        // For "indented" style, also accept:
        // - Line indentation matching expected (leading operator: `&& expr`)
        // - Right col matching left indent (aligned with containing expression)
        // - Right col matching left col (aligned with left operand)
        // - Keyword-condition double-width indentation
        // For both styles, also accept:
        // - Line indentation matching expected_indented (leading operator: `&& expr`)
        // - Right col matching left indent (aligned with containing expression)
        // - Right col matching left col (aligned with left operand)
        let is_ok = if style == "aligned" {
            right_col == expected
                || right_col == expected_indented
                || line_indent == expected_indented
                || right_col == left_indent
                || kw_expected.is_some_and(|kw| right_col == kw || line_indent == kw)
        } else {
            right_col == expected
                || line_indent == expected
                || right_col == left_indent
                || right_col == left_col
                || kw_expected.is_some_and(|kw| right_col == kw || line_indent == kw)
        };

        if !is_ok {
            return vec![self.diagnostic(
                source,
                right_line,
                right_col,
                format!(
                    "Use {} (not {}) spaces for indentation of a continuation line.",
                    width,
                    right_col.saturating_sub(left_indent)
                ),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        MultilineOperationIndentation,
        "cops/layout/multiline_operation_indentation"
    );

    #[test]
    fn single_line_operation_ignored() {
        let source = b"x = 1 + 2\n";
        let diags = run_cop_full(&MultilineOperationIndentation, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn or_in_def_body_no_offense() {
        let src = b"def valid?(user)\n  user.foo ||\n    user.bar\nend\n";
        let diags = run_cop_full(&MultilineOperationIndentation, src);
        assert!(
            diags.is_empty(),
            "correctly indented || continuation should not flag, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn or_in_def_body_with_rescue_no_offense() {
        let src = b"  def valid_otp_attempt?(user)\n    user.validate_and_consume_otp!(user_params[:otp_attempt]) ||\n      user.invalidate_otp_backup_code!(user_params[:otp_attempt])\n  rescue OpenSSL::Cipher::CipherError\n    false\n  end\n";
        let diags = run_cop_full(&MultilineOperationIndentation, src);
        assert!(
            diags.is_empty(),
            "correctly indented || with rescue should not flag, got: {:?}",
            diags
                .iter()
                .map(|d| format!(
                    "line {} col {} {}",
                    d.location.line, d.location.column, d.message
                ))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn nested_and_or_deep_indent_no_offense() {
        let src = b"        def implicit_block?(node)\n          return false unless node.arguments.any?\n\n          node.last_argument.block_pass_type? ||\n            (node.last_argument.sym_type? &&\n            methods_accepting_symbol.include?(node.method_name.to_s))\n        end\n";
        let diags = run_cop_full(&MultilineOperationIndentation, src);
        assert!(
            diags.is_empty(),
            "nested && inside || with aligned continuation should not flag, got: {:?}",
            diags
                .iter()
                .map(|d| format!(
                    "line {} col {} {}",
                    d.location.line, d.location.column, d.message
                ))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn aligned_style() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("aligned".into()),
            )]),
            ..CopConfig::default()
        };
        // Continuation aligned with the left operand
        let src = b"x = a &&\n    b\n";
        let diags = run_cop_full_with_config(&MultilineOperationIndentation, src, config.clone());
        assert!(
            diags.is_empty(),
            "aligned style should accept operand-aligned continuation"
        );

        // In "aligned" style, RuboCop accepts indented form in non-condition contexts.
        let src2 = b"x = a &&\n  b\n";
        let diags2 = run_cop_full_with_config(&MultilineOperationIndentation, src2, config.clone());
        assert!(
            diags2.is_empty(),
            "aligned style should accept indented continuation in non-condition contexts"
        );

        // But wildly misaligned should still be flagged
        let src3 = b"x = a &&\n        b\n";
        let diags3 = run_cop_full_with_config(&MultilineOperationIndentation, src3, config);
        assert_eq!(
            diags3.len(),
            1,
            "aligned style should flag incorrectly indented continuation"
        );
    }
}
