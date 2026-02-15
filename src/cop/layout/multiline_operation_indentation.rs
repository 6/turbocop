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
            let (recv_line, _) = source.offset_to_line_col(recv_loc.start_offset());
            let first_arg = &args[0];
            let arg_loc = first_arg.location();
            let (arg_line, arg_col) = source.offset_to_line_col(arg_loc.start_offset());

            // Only check multiline operations
            if arg_line == recv_line {
                return Vec::new();
            }

            let width = config.get_usize("IndentationWidth", 2);

            let recv_line_bytes = source.lines().nth(recv_line - 1).unwrap_or(b"");
            let recv_indent = indentation_of(recv_line_bytes);
            let expected = match style {
                "aligned" => {
                    // Align with the receiver's column
                    let (_, recv_col) = source.offset_to_line_col(recv_loc.start_offset());
                    recv_col
                }
                _ => recv_indent + width, // "indented" (default)
            };

            if arg_col != expected {
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
        let (right_line, right_col) = source.offset_to_line_col(right.location().start_offset());

        if right_line == left_line {
            return Vec::new();
        }

        let width = config.get_usize("IndentationWidth", 2);

        let left_line_bytes = source.lines().nth(left_line - 1).unwrap_or(b"");
        let left_indent = indentation_of(left_line_bytes);
        let expected = match style {
            "aligned" => left_col,
            _ => left_indent + width, // "indented" (default)
        };

        if right_col != expected {
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

        // Continuation indented (default style) should be flagged with aligned style
        let src2 = b"x = a &&\n  b\n";
        let diags2 = run_cop_full_with_config(&MultilineOperationIndentation, src2, config);
        assert_eq!(diags2.len(), 1, "aligned style should flag indented continuation");
    }
}
