use ruby_prism::Visit;

use crate::cop::util::indentation_of;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FirstArgumentIndentation;

impl Cop for FirstArgumentIndentation {
    fn name(&self) -> &'static str {
        "Layout/FirstArgumentIndentation"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let style = config.get_str(
            "EnforcedStyle",
            "special_for_inner_method_call_in_parentheses",
        );
        let width = config.get_usize("IndentationWidth", 2);
        let mut visitor = FirstArgVisitor {
            cop: self,
            source,
            style,
            width,
            diagnostics: Vec::new(),
            // Stack of parent call info: (is_parenthesized, call_start_offset)
            parent_call_stack: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct FirstArgVisitor<'a> {
    cop: &'a FirstArgumentIndentation,
    source: &'a SourceFile,
    style: &'a str,
    width: usize,
    diagnostics: Vec<Diagnostic>,
    /// Stack of parent call info: (is_parenthesized, call_node_start_col)
    /// call_node_start_col is the column of the start of the entire call expression
    /// (including receiver), matching RuboCop's node.source_range.begin_pos
    parent_call_stack: Vec<ParentCallInfo>,
}

struct ParentCallInfo {
    /// Whether the parent call has parenthesized arguments
    is_parenthesized: bool,
    /// The start offsets of each argument in the parent call, so we can check
    /// if the current call node is one of the parent's arguments
    arg_start_offsets: Vec<usize>,
    /// Not a setter or bare operator
    is_eligible: bool,
}

impl FirstArgVisitor<'_> {
    fn check_call(
        &mut self,
        call_start_offset: usize,
        message_loc: Option<ruby_prism::Location<'_>>,
        opening_loc: Option<ruby_prism::Location<'_>>,
        arguments: Option<ruby_prism::ArgumentsNode<'_>>,
        name: &str,
    ) {
        // Must have parenthesized arguments
        let _open_loc = match opening_loc {
            Some(loc) if loc.as_slice() == b"(" => loc,
            _ => return,
        };

        let args_node = match arguments {
            Some(a) => a,
            None => return,
        };

        let args: Vec<_> = args_node.arguments().iter().collect();
        if args.is_empty() {
            return;
        }

        let first_arg = &args[0];

        // Use message_loc (method name) for determining the call line.
        // This handles chained calls where call_start_offset would be on
        // a different line (the receiver's line).
        let call_line_offset = message_loc
            .map(|loc| loc.start_offset())
            .unwrap_or(call_start_offset);
        let (call_line, _) = self.source.offset_to_line_col(call_line_offset);

        let first_arg_loc = first_arg.location();
        let (arg_line, arg_col) = self.source.offset_to_line_col(first_arg_loc.start_offset());

        // Skip if first arg is on same line as method call
        if arg_line == call_line {
            return;
        }

        // Skip bare operators (like `a + b`) and setter methods (like `self.x = 1`)
        if is_bare_operator(name) || is_setter_method(name) {
            return;
        }

        let expected = self.compute_expected_indent(
            call_start_offset,
            first_arg_loc.start_offset(),
            arg_line,
        );

        if arg_col != expected {
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                arg_line,
                arg_col,
                "Indent the first argument one step more than the start of the previous line."
                    .to_string(),
            ));
        }
    }

    fn compute_expected_indent(
        &self,
        call_start_offset: usize,
        first_arg_start_offset: usize,
        arg_line: usize,
    ) -> usize {
        if self.style == "consistent" {
            // Always use previous code line indent + width
            return previous_code_line_indent(self.source, arg_line) + self.width;
        }

        if self.style == "consistent_relative_to_receiver" {
            // Use the column of the call node start (includes receiver) + width
            let (_, call_col) = self.source.offset_to_line_col(call_start_offset);
            return call_col + self.width;
        }

        // special_for_inner_method_call or special_for_inner_method_call_in_parentheses
        if self.is_special_inner_call(call_start_offset) {
            // Check if base_range (from call start to first arg) spans multiple lines
            let (call_start_line, call_start_col) =
                self.source.offset_to_line_col(call_start_offset);
            let (arg_start_line, _) = self.source.offset_to_line_col(first_arg_start_offset);

            // Determine if the range from call start to arg start is "single line"
            // after stripping whitespace (matching RuboCop's column_of behavior)
            if is_single_line_base_range(
                self.source,
                call_start_offset,
                first_arg_start_offset,
                call_start_line,
                arg_start_line,
            ) {
                // Single-line: use the column of the call expression start
                call_start_col + self.width
            } else {
                // Multi-line: use previous code line indent
                previous_code_line_indent(self.source, arg_line) + self.width
            }
        } else {
            // Not a special inner call: use previous code line indent + width
            previous_code_line_indent(self.source, arg_line) + self.width
        }
    }

    /// Check if the current call is a "special inner call" — meaning it is an
    /// argument of an outer method call. For `special_for_inner_method_call_in_parentheses`,
    /// the outer call must be parenthesized.
    fn is_special_inner_call(&self, call_start_offset: usize) -> bool {
        if let Some(parent) = self.parent_call_stack.last() {
            if !parent.is_eligible {
                return false;
            }

            if self.style == "special_for_inner_method_call_in_parentheses"
                && !parent.is_parenthesized
            {
                return false;
            }

            // The call must be an argument of the parent (not just any descendant).
            // We check if call_start_offset matches any of the parent's argument
            // start offsets or is contained within one of them.
            // Actually, RuboCop checks: node.source_range.begin_pos > parent.source_range.begin_pos
            // which means the inner call starts inside the parent call (not being the
            // first part of a chained call).
            // Since we're inside the parent's argument visitor, we know we're a descendant.
            // We just need to verify the call starts after the parent call start
            // (which is always true for arguments).
            // But we also need to make sure the current call IS a direct argument,
            // not just a deeply nested expression. For simplicity, we check if
            // call_start_offset appears in the parent's arg_start_offsets.
            // Actually, RuboCop just checks that the node is a direct child argument
            // of the parent. It walks up one level via node.parent. We can simulate
            // this by checking if the call_start_offset matches one of the parent's
            // argument start offsets.
            parent
                .arg_start_offsets
                .iter()
                .any(|&off| off == call_start_offset)
        } else {
            false
        }
    }
}

/// Check if the range from call_start to arg_start is effectively single-line
/// after stripping whitespace (matching RuboCop's column_of behavior).
/// This is true when the text between call_start and arg_start, after stripping
/// leading/trailing whitespace, contains no newlines.
fn is_single_line_base_range(
    source: &SourceFile,
    call_start: usize,
    arg_start: usize,
    call_line: usize,
    arg_line: usize,
) -> bool {
    if call_line == arg_line {
        return true;
    }
    // For a range spanning exactly 2 lines (call on line N, arg on line N+1),
    // after stripping the trailing whitespace (spaces/newlines between the call's
    // opening paren and the arg), the result is typically single-line.
    // We check the actual source bytes.
    let bytes = source.as_bytes();
    if arg_start > call_start && arg_start <= bytes.len() {
        let range_bytes = &bytes[call_start..arg_start];
        // Strip trailing whitespace (spaces, tabs, newlines)
        let stripped = trim_end_whitespace(range_bytes);
        // Check if the stripped content contains any newline
        !stripped.iter().any(|&b| b == b'\n')
    } else {
        call_line == arg_line
    }
}

fn trim_end_whitespace(bytes: &[u8]) -> &[u8] {
    let mut end = bytes.len();
    while end > 0 && matches!(bytes[end - 1], b' ' | b'\t' | b'\n' | b'\r') {
        end -= 1;
    }
    &bytes[..end]
}

/// Find the indentation of the previous non-blank, non-comment code line.
/// This matches RuboCop's `previous_code_line` behavior.
fn previous_code_line_indent(source: &SourceFile, line_number: usize) -> usize {
    let mut line_num = line_number;
    loop {
        if line_num <= 1 {
            return 0;
        }
        line_num -= 1;
        let line_bytes = source.lines().nth(line_num - 1).unwrap_or(b"");
        // Skip blank lines
        if line_bytes.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r') {
            continue;
        }
        // Skip comment lines (lines where first non-whitespace is #)
        let trimmed = line_bytes
            .iter()
            .skip_while(|&&b| b == b' ' || b == b'\t')
            .copied()
            .next();
        if trimmed == Some(b'#') {
            continue;
        }
        return indentation_of(line_bytes);
    }
}

fn is_bare_operator(name: &str) -> bool {
    // Operators that can be defined as methods
    matches!(
        name,
        "+" | "-" | "*" | "/" | "%" | "**" | "==" | "!=" | ">" | "<" | ">=" | "<=" | "<=>"
            | "<<" | ">>" | "&" | "|" | "^" | "~" | "!" | "=~" | "!~" | "[]" | "[]="
    )
}

fn is_setter_method(name: &str) -> bool {
    name.ends_with('=') && name != "==" && name != "!=" && name != "[]="
}

impl<'pr> Visit<'pr> for FirstArgVisitor<'_, > {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let call_start_offset = node.location().start_offset();
        let name_bytes = node.name().as_slice();
        let name_str = std::str::from_utf8(name_bytes).unwrap_or("");

        // Check this call node for first argument indentation
        self.check_call(
            call_start_offset,
            node.message_loc(),
            node.opening_loc(),
            node.arguments(),
            name_str,
        );

        // Determine if this call is parenthesized and eligible for being a
        // "parent call" context for inner calls
        let is_parenthesized = node
            .opening_loc()
            .is_some_and(|loc| loc.as_slice() == b"(");
        let is_eligible = !is_bare_operator(name_str) && !is_setter_method(name_str);

        // Collect argument start offsets
        let arg_start_offsets: Vec<usize> = node
            .arguments()
            .map(|args| {
                args.arguments()
                    .iter()
                    .map(|arg| arg.location().start_offset())
                    .collect()
            })
            .unwrap_or_default();

        let parent_info = ParentCallInfo {
            is_parenthesized,
            arg_start_offsets,
            is_eligible,
        };

        self.parent_call_stack.push(parent_info);
        ruby_prism::visit_call_node(self, node);
        self.parent_call_stack.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        FirstArgumentIndentation,
        "cops/layout/first_argument_indentation"
    );

    #[test]
    fn args_on_same_line_ignored() {
        let source = b"foo(1, 2, 3)\n";
        let diags = run_cop_full(&FirstArgumentIndentation, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn chained_call_same_line_args_ignored() {
        // Chained call where arg is on same line as .method — should not flag
        let source = b"params\n  .require(:domain_block)\n  .slice(*PERMITTED)\n";
        let diags = run_cop_full(&FirstArgumentIndentation, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn special_inner_call_aligned_to_receiver() {
        // Inner call in parenthesized outer call — first arg aligned after receiver start + width
        let source = b"Conversation.create!(conversation_params.merge(\n                       contact_inbox_id: id\n                     ))\n";
        let diags = run_cop_full(&FirstArgumentIndentation, source);
        assert!(diags.is_empty(), "inner call with args aligned to receiver start should not be flagged, got: {:?}", diags);
    }

    #[test]
    fn special_inner_call_in_expect() {
        // expect(helper.generate_category_link(\n         portal_slug: 'x'\n       ))
        let source = b"expect(helper.generate_category_link(\n         portal_slug: 'portal_slug'\n       ))\n";
        let diags = run_cop_full(&FirstArgumentIndentation, source);
        assert!(diags.is_empty(), "inner call in expect() should not be flagged, got: {:?}", diags);
    }

    #[test]
    fn non_inner_call_still_flagged() {
        // Top-level call (not inside another call) with wrong indent
        let source = b"foo(\n      1\n)\n";
        let diags = run_cop_full(&FirstArgumentIndentation, source);
        assert_eq!(diags.len(), 1, "non-inner call with wrong indent should be flagged");
    }
}
