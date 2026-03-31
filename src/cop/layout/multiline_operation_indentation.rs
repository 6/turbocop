use crate::cop::node_type::{AND_NODE, CALL_NODE, OR_NODE};
use crate::cop::util::indentation_of;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks indentation of multiline binary operations using RuboCop's context
/// rules instead of the previous permissive column heuristics.
///
/// 2026-03-31:
/// - FN root cause: chained `&&`/`||` conditions in `if` predicates, plain
///   multiline boolean expressions, assignment RHS continuations, and modifier
///   `if` conditions were all missed because the cop accepted too many fallback
///   columns (`left_indent`, `left_col`, line indent) and skipped outer boolean
///   nodes when the left side was another boolean op.
/// - FP root cause: leading-operator continuations such as
///   `expr \\\n&& other_expr` were treated like right operands that began their
///   line, even though RuboCop ignores them.
/// - Fix: only inspect RHS operands that actually begin their line, stop
///   special-casing nested boolean chains, and choose the expected column from
///   RuboCop-style ancestor context: keyword predicates, assignment RHS,
///   method-call arguments, and grouped/parenthesized exclusions.
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

        if let Some(call_node) = node.as_call_node() {
            if !Self::relevant_operator_call(&call_node) {
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

            let first_arg = match args_node.arguments().iter().next() {
                Some(arg) => arg,
                None => return,
            };

            let lhs = left_hand_side(receiver);
            if let Some(diagnostic) =
                self.check_operation(source, node, &lhs, &first_arg, config, style)
            {
                diagnostics.push(diagnostic);
            }
            return;
        }

        if let Some(and_node) = node.as_and_node() {
            if let Some(diagnostic) = self.check_operation(
                source,
                node,
                &and_node.left(),
                &and_node.right(),
                config,
                style,
            ) {
                diagnostics.push(diagnostic);
            }
            return;
        }

        if let Some(or_node) = node.as_or_node() {
            if let Some(diagnostic) = self.check_operation(
                source,
                node,
                &or_node.left(),
                &or_node.right(),
                config,
                style,
            ) {
                diagnostics.push(diagnostic);
            }
        }
    }
}

impl MultilineOperationIndentation {
    fn relevant_operator_call(call: &ruby_prism::CallNode<'_>) -> bool {
        OPERATOR_METHODS.contains(&call.name().as_slice()) && call.call_operator_loc().is_none()
    }

    fn check_operation(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        left: &ruby_prism::Node<'_>,
        right: &ruby_prism::Node<'_>,
        config: &CopConfig,
        style: &str,
    ) -> Option<Diagnostic> {
        if !begins_its_line(source, right.location().start_offset())
            || not_for_this_cop(source, node)
        {
            return None;
        }

        let (right_line, right_col) = source.offset_to_line_col(right.location().start_offset());
        let width = config.get_usize("IndentationWidth", 2);
        let assignment = assignment_context(source, node.location().start_offset());
        let should_align = should_align(source, node, style, &assignment);
        let correct_column = if should_align {
            source.offset_to_line_col(node.location().start_offset()).1
        } else {
            indentation_for_node(source, left) + correct_indentation(source, node, width)
        };

        if right_col != correct_column {
            return Some(self.diagnostic(
                source,
                right_line,
                right_col,
                message(source, node, left, right, width, &assignment, should_align),
            ));
        }

        None
    }
}

#[derive(Clone)]
struct KeywordContext {
    keyword: &'static str,
    postfix: bool,
}

#[derive(Default)]
struct AssignmentContext {
    is_assignment: bool,
    starts_next_line: bool,
}

fn begins_its_line(source: &SourceFile, start_offset: usize) -> bool {
    let bytes = source.as_bytes();
    let mut pos = start_offset.min(bytes.len());

    while pos > 0 {
        pos -= 1;
        match bytes[pos] {
            b'\n' => return true,
            b' ' | b'\t' => continue,
            _ => return false,
        }
    }

    true
}

fn indentation_for_node(source: &SourceFile, node: &ruby_prism::Node<'_>) -> usize {
    let (line, _) = source.offset_to_line_col(node.location().start_offset());
    indentation_of(source.lines().nth(line - 1).unwrap_or(b""))
}

fn correct_indentation(source: &SourceFile, node: &ruby_prism::Node<'_>, width: usize) -> usize {
    match keyword_context(source, node.location().start_offset()) {
        Some(ctx) if !ctx.postfix => width * 2,
        _ => width,
    }
}

fn should_align(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    style: &str,
    assignment: &AssignmentContext,
) -> bool {
    if assignment.starts_next_line {
        return true;
    }

    if style != "aligned" {
        return false;
    }

    keyword_context(source, node.location().start_offset()).is_some() || assignment.is_assignment
}

fn message(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    left: &ruby_prism::Node<'_>,
    right: &ruby_prism::Node<'_>,
    width: usize,
    assignment: &AssignmentContext,
    should_align: bool,
) -> String {
    let what = operation_description(source, node, assignment);
    if should_align {
        format!("Align the operands of {what} spanning multiple lines.")
    } else {
        let used_indentation = source.offset_to_line_col(right.location().start_offset()).1
            - indentation_for_node(source, left);
        format!(
            "Use {} (not {}) spaces for indenting {} spanning multiple lines.",
            correct_indentation(source, node, width),
            used_indentation,
            what
        )
    }
}

fn operation_description(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    assignment: &AssignmentContext,
) -> String {
    if let Some(ctx) = keyword_context(source, node.location().start_offset()) {
        let kind = if ctx.keyword == "for" {
            "collection"
        } else {
            "condition"
        };
        let statement_article = if ctx.keyword.starts_with('i') || ctx.keyword.starts_with('u') {
            "an"
        } else {
            "a"
        };
        return format!(
            "a {kind} in {statement_article} `{}` statement",
            ctx.keyword
        );
    }

    if assignment.is_assignment {
        return "an expression in an assignment".to_string();
    }

    "an expression".to_string()
}

fn keyword_context(source: &SourceFile, start_offset: usize) -> Option<KeywordContext> {
    let (line, col) = source.offset_to_line_col(start_offset);
    let line_bytes = source.lines().nth(line - 1).unwrap_or(b"");
    let line_prefix = &line_bytes[..col.min(line_bytes.len())];
    let trimmed = trim_leading_ascii_whitespace(line_bytes);

    if trimmed.starts_with(b"if ") || trimmed.starts_with(b"elsif ") {
        return Some(KeywordContext {
            keyword: if trimmed.starts_with(b"elsif ") {
                "elsif"
            } else {
                "if"
            },
            postfix: false,
        });
    }
    if trimmed.starts_with(b"unless ") {
        return Some(KeywordContext {
            keyword: "unless",
            postfix: false,
        });
    }
    if trimmed.starts_with(b"while ") {
        return Some(KeywordContext {
            keyword: "while",
            postfix: false,
        });
    }
    if trimmed.starts_with(b"until ") {
        return Some(KeywordContext {
            keyword: "until",
            postfix: false,
        });
    }
    if trimmed.starts_with(b"for ") {
        return Some(KeywordContext {
            keyword: "for",
            postfix: false,
        });
    }
    if trimmed.starts_with(b"return ") {
        return Some(KeywordContext {
            keyword: "return",
            postfix: false,
        });
    }
    if ends_with_keyword(line_prefix, b" if ") {
        return Some(KeywordContext {
            keyword: "if",
            postfix: true,
        });
    }
    if ends_with_keyword(line_prefix, b" unless ") {
        return Some(KeywordContext {
            keyword: "unless",
            postfix: true,
        });
    }

    None
}

fn not_for_this_cop(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    is_inside_parentheses(source, node)
}

fn assignment_context(source: &SourceFile, start_offset: usize) -> AssignmentContext {
    let (line, col) = source.offset_to_line_col(start_offset);
    let line_bytes = source.lines().nth(line - 1).unwrap_or(b"");
    let line_prefix = &line_bytes[..col.min(line_bytes.len())];

    let mut context = AssignmentContext::default();
    if has_assignment_operator(line_prefix) {
        context.is_assignment = true;
        return context;
    }

    if line > 1 {
        let prev_line = source.lines().nth(line - 2).unwrap_or(b"");
        if line_ends_with_assignment(prev_line) {
            context.is_assignment = true;
            context.starts_next_line = true;
        }
    }

    context
}

fn left_hand_side(lhs: ruby_prism::Node<'_>) -> ruby_prism::Node<'_> {
    lhs
}

fn trim_leading_ascii_whitespace(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    &bytes[start..]
}

fn ends_with_keyword(line_prefix: &[u8], keyword: &[u8]) -> bool {
    line_prefix.ends_with(keyword)
        || trim_leading_ascii_whitespace(line_prefix).starts_with(&keyword[1..])
}

fn has_assignment_operator(prefix: &[u8]) -> bool {
    for i in 0..prefix.len() {
        if prefix[i] != b'=' {
            continue;
        }

        if i + 1 < prefix.len() && prefix[i + 1] == b'=' {
            continue;
        }
        if i > 0 && matches!(prefix[i - 1], b'=' | b'!' | b'<' | b'>' | b':') {
            continue;
        }
        if i + 1 < prefix.len() && prefix[i + 1] == b'>' {
            continue;
        }

        return true;
    }

    false
}

fn line_ends_with_assignment(line: &[u8]) -> bool {
    let trimmed = line
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map(|end| &line[..=end])
        .unwrap_or(b"");
    trimmed.ends_with(b"=")
        || trimmed.ends_with(b"+=")
        || trimmed.ends_with(b"-=")
        || trimmed.ends_with(b"*=")
        || trimmed.ends_with(b"/=")
        || trimmed.ends_with(b"%=")
        || trimmed.ends_with(b"<<=")
        || trimmed.ends_with(b">>=")
        || trimmed.ends_with(b"&&=")
        || trimmed.ends_with(b"||=")
}

fn is_inside_parentheses(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    let bytes = source.as_bytes();
    let node_start = node.location().start_offset();
    let node_end = node.location().end_offset();

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
            _ => {}
        }
    }

    false
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

        // Ordinary expressions still use a normal indentation step.
        let src2 = b"a &&\n  b\n";
        let diags2 = run_cop_full_with_config(&MultilineOperationIndentation, src2, config.clone());
        assert!(
            diags2.is_empty(),
            "aligned style should accept indented continuation outside assignment/keyword contexts"
        );

        // Assignment RHS should not accept ordinary indentation in aligned style.
        let src3 = b"x = a &&\n  b\n";
        let diags3 = run_cop_full_with_config(&MultilineOperationIndentation, src3, config);
        assert_eq!(
            diags3.len(),
            1,
            "aligned style should flag indented assignment continuation"
        );
    }
}
