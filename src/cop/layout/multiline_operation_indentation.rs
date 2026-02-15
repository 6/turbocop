use crate::cop::util::indentation_of;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineOperationIndentation;

const OPERATOR_METHODS: &[&[u8]] = &[
    b"+", b"-", b"*", b"/", b"%", b"**",
    b"==", b"!=", b"<", b">", b"<=", b">=", b"<=>",
    b"&", b"|", b"^",
    b"<<", b">>",
];

impl Cop for MultilineOperationIndentation {
    fn name(&self) -> &'static str {
        "Layout/MultilineOperationIndentation"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "indented");

        // Check CallNode with operator methods (binary operators are parsed as calls)
        if let Some(call_node) = node.as_call_node() {
            let method_name = call_node.name().as_slice();

            if !OPERATOR_METHODS.contains(&method_name) {
                return Vec::new();
            }

            let receiver = match call_node.receiver() {
                Some(r) => r,
                None => return Vec::new(),
            };

            let args_node = match call_node.arguments() {
                Some(a) => a,
                None => return Vec::new(),
            };

            let args: Vec<_> = args_node.arguments().iter().collect();
            if args.is_empty() {
                return Vec::new();
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
                return Vec::new();
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

            // For "aligned" style, RuboCop accepts both aligned and properly
            // indented forms in non-condition contexts (assignments, method args).
            let is_ok = if style == "aligned" {
                arg_col == expected || arg_col == expected_indented
            } else {
                arg_col == expected
            };

            if !is_ok {
                return vec![self.diagnostic(
                    source,
                    arg_line,
                    arg_col,
                    format!(
                        "Use {} (not {}) spaces for indentation of a continuation line.",
                        width,
                        arg_col.saturating_sub(recv_indent)
                    ),
                )];
            }
        }

        // Check AndNode
        if let Some(and_node) = node.as_and_node() {
            return self.check_binary_node(
                source,
                &and_node.left(),
                &and_node.right(),
                config,
                style,
            );
        }

        // Check OrNode
        if let Some(or_node) = node.as_or_node() {
            return self.check_binary_node(
                source,
                &or_node.left(),
                &or_node.right(),
                config,
                style,
            );
        }

        Vec::new()
    }
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

        let width = config.get_usize("IndentationWidth", 2);

        let left_line_bytes = source.lines().nth(left_line - 1).unwrap_or(b"");
        let left_indent = indentation_of(left_line_bytes);
        let expected_indented = left_indent + width;
        let expected = match style {
            "aligned" => left_col,
            _ => expected_indented, // "indented" (default)
        };

        // For "aligned" style, RuboCop only enforces alignment in certain
        // contexts (if/while conditions, etc). In other contexts, the
        // second operand should be indented. Accept both forms.
        let is_ok = if style == "aligned" {
            right_col == expected || right_col == expected_indented
        } else {
            right_col == expected
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
        assert!(diags.is_empty(), "correctly indented || continuation should not flag, got: {:?}", diags.iter().map(|d| &d.message).collect::<Vec<_>>());
    }

    #[test]
    fn or_in_def_body_with_rescue_no_offense() {
        let src = b"  def valid_otp_attempt?(user)\n    user.validate_and_consume_otp!(user_params[:otp_attempt]) ||\n      user.invalidate_otp_backup_code!(user_params[:otp_attempt])\n  rescue OpenSSL::Cipher::CipherError\n    false\n  end\n";
        let diags = run_cop_full(&MultilineOperationIndentation, src);
        assert!(diags.is_empty(), "correctly indented || with rescue should not flag, got: {:?}", diags.iter().map(|d| format!("line {} col {} {}", d.location.line, d.location.column, d.message)).collect::<Vec<_>>());
    }

    #[test]
    fn aligned_style() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("aligned".into())),
            ]),
            ..CopConfig::default()
        };
        // Continuation aligned with the left operand
        let src = b"x = a &&\n    b\n";
        let diags = run_cop_full_with_config(&MultilineOperationIndentation, src, config.clone());
        assert!(diags.is_empty(), "aligned style should accept operand-aligned continuation");

        // In "aligned" style, RuboCop accepts indented form in non-condition contexts.
        let src2 = b"x = a &&\n  b\n";
        let diags2 = run_cop_full_with_config(&MultilineOperationIndentation, src2, config.clone());
        assert!(diags2.is_empty(), "aligned style should accept indented continuation in non-condition contexts");

        // But wildly misaligned should still be flagged
        let src3 = b"x = a &&\n        b\n";
        let diags3 = run_cop_full_with_config(&MultilineOperationIndentation, src3, config);
        assert_eq!(diags3.len(), 1, "aligned style should flag incorrectly indented continuation");
    }
}
