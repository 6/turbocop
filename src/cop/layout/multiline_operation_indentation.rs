use crate::cop::node_type::{AND_NODE, CALL_NODE, OR_NODE};
use crate::cop::util::indentation_of;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks indentation of multiline binary operations.
///
/// Key fix (2026-04-01): removed blanket skip for nested boolean chains
/// (And/Or as left operand) which was the root cause of ~39k FN.  Added
/// RuboCop-compatible `begins_its_line?` guard that skips leading-operator
/// patterns like `expr \n  && other_expr`, fixing the confirmed FP class.
/// Tightened the `is_ok` check for And/Or nodes: in non-keyword contexts
/// only `left_indent + width` is accepted (not `left_col`); in keyword
/// conditions with aligned style, alignment with `left_col` or double-width
/// `kw_expected` are accepted.
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

/// Count leading whitespace bytes (spaces and tabs) on a line.
fn leading_whitespace_len(line: &[u8]) -> usize {
    line.iter()
        .take_while(|&&b| b == b' ' || b == b'\t')
        .count()
}

/// Check if a line's trimmed content starts with one of the keyword prefixes.
fn line_starts_with_keyword(line_bytes: &[u8]) -> bool {
    let start = line_bytes
        .iter()
        .position(|&b| b != b' ' && b != b'\t')
        .unwrap_or(line_bytes.len());
    let trimmed = &line_bytes[start..];
    trimmed.starts_with(b"if ")
        || trimmed.starts_with(b"elsif ")
        || trimmed.starts_with(b"unless ")
        || trimmed.starts_with(b"while ")
        || trimmed.starts_with(b"until ")
        || trimmed.starts_with(b"return ")
        || trimmed.starts_with(b"for ")
}

/// Check if the given line (or a preceding line connected by backslash
/// continuation) starts with a keyword that creates a condition context
/// (if, elsif, unless, while, until, return, for). RuboCop's
/// `kw_node_with_special_indentation` walks AST ancestors; we approximate
/// by also scanning the immediately preceding line when it ends with `\`.
fn is_in_keyword_condition(source: &SourceFile, line: usize) -> bool {
    let line_bytes = source.lines().nth(line - 1).unwrap_or(b"");
    if line_starts_with_keyword(line_bytes) {
        return true;
    }
    // Check the preceding line if it ends with backslash continuation
    // (handles `if \<newline> condition` patterns).
    if line > 1 {
        let prev_line = source.lines().nth(line - 2).unwrap_or(b"");
        let trimmed_end = prev_line
            .iter()
            .rposition(|&b| b != b' ' && b != b'\t' && b != b'\r' && b != b'\n')
            .map(|i| prev_line[i])
            .unwrap_or(0);
        if trimmed_end == b'\\' && line_starts_with_keyword(prev_line) {
            return true;
        }
    }
    false
}

/// Extract the keyword name from a line (or backslash-connected preceding line).
fn keyword_on_line(source: &SourceFile, line: usize) -> Option<&'static str> {
    fn extract_keyword(line_bytes: &[u8]) -> Option<&'static str> {
        let start = line_bytes
            .iter()
            .position(|&b| b != b' ' && b != b'\t')
            .unwrap_or(line_bytes.len());
        let trimmed = &line_bytes[start..];
        if trimmed.starts_with(b"if ") || trimmed.starts_with(b"if(") {
            Some("if")
        } else if trimmed.starts_with(b"elsif ") {
            Some("elsif")
        } else if trimmed.starts_with(b"unless ") {
            Some("unless")
        } else if trimmed.starts_with(b"while ") {
            Some("while")
        } else if trimmed.starts_with(b"until ") {
            Some("until")
        } else if trimmed.starts_with(b"return ") {
            Some("return")
        } else if trimmed.starts_with(b"for ") {
            Some("for")
        } else {
            None
        }
    }
    let line_bytes = source.lines().nth(line - 1).unwrap_or(b"");
    if let Some(kw) = extract_keyword(line_bytes) {
        return Some(kw);
    }
    // Check preceding line if connected by backslash
    if line > 1 {
        let prev_line = source.lines().nth(line - 2).unwrap_or(b"");
        let trimmed_end = prev_line
            .iter()
            .rposition(|&b| b != b' ' && b != b'\t' && b != b'\r' && b != b'\n')
            .map(|i| prev_line[i])
            .unwrap_or(0);
        if trimmed_end == b'\\' {
            return extract_keyword(prev_line);
        }
    }
    None
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

        // RuboCop's `begins_its_line?` — only check if the right operand is
        // the first non-whitespace on its line. When the operator is leading
        // (e.g., `expr \n  && other_expr`), the right operand is NOT the first
        // token on the line and RuboCop skips the check.
        // Use tab-aware whitespace counting (not just spaces) to handle
        // codebases with tab indentation.
        let right_line_bytes = source.lines().nth(right_line - 1).unwrap_or(b"");
        let first_nonws = leading_whitespace_len(right_line_bytes);
        if right_col != first_nonws {
            return Vec::new();
        }

        let width = config.get_usize("IndentationWidth", 2);

        // For chained boolean expressions like And(And(a,b), c), the left
        // operand's start_offset points to `a`'s position (the root of the
        // chain). This gives us the correct base indentation.
        let left_line_bytes = source.lines().nth(left_line - 1).unwrap_or(b"");
        let left_indent = indentation_of(left_line_bytes);
        let expected_indented = left_indent + width;

        // Determine if we're in a keyword condition (if/elsif/unless/while/
        // until/return/for). RuboCop's `kw_node_with_special_indentation`
        // doubles the indentation width and in aligned style accepts
        // alignment with the left operand.
        let in_kw = is_in_keyword_condition(source, left_line);
        let kw_expected = if in_kw {
            Some(left_indent + 2 * width)
        } else {
            None
        };

        // RuboCop's `should_align?` returns true for keyword conditions
        // (with aligned style), certain assignment contexts, and method
        // call arguments. We implement the keyword case; the others are
        // less common and are left for future work.
        let should_align = in_kw && style == "aligned";

        let is_ok = if should_align {
            // Keyword + aligned: accept alignment with left operand column
            // or keyword double-width indentation.
            right_col == left_col || kw_expected.is_some_and(|kw| right_col == kw)
        } else {
            // Standard indentation: left_indent + width
            right_col == expected_indented || kw_expected.is_some_and(|kw| right_col == kw)
        };

        if !is_ok {
            let message = if should_align {
                let keyword = keyword_on_line(source, left_line).unwrap_or("if");
                let kind = if keyword == "for" {
                    "collection"
                } else {
                    "condition"
                };
                let article = if keyword.starts_with('i') || keyword.starts_with('u') {
                    "an"
                } else {
                    "a"
                };
                format!(
                    "Align the operands of a {kind} in {article} `{keyword}` statement spanning multiple lines."
                )
            } else {
                let used = right_col.saturating_sub(left_indent);
                format!(
                    "Use {width} (not {used}) spaces for indenting an expression spanning multiple lines."
                )
            };
            return vec![self.diagnostic(source, right_line, right_col, message)];
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
        // Aligned with left operand in keyword condition (should_align = true)
        let src = b"if a &&\n   b\n  c\nend\n";
        let diags = run_cop_full_with_config(&MultilineOperationIndentation, src, config.clone());
        assert!(
            diags.is_empty(),
            "aligned style in keyword condition should accept operand-aligned continuation, got: {:?}",
            diags
                .iter()
                .map(|d| format!("L{}:C{} {}", d.location.line, d.location.column, &d.message))
                .collect::<Vec<_>>()
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
