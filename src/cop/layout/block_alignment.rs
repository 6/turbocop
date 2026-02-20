use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::BLOCK_NODE;

pub struct BlockAlignment;

impl Cop for BlockAlignment {
    fn name(&self) -> &'static str {
        "Layout/BlockAlignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE]
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
        let style = config.get_str("EnforcedStyleAlignWith", "either");
        let block_node = match node.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let closing_loc = block_node.closing_loc();

        // Only check do...end blocks, not brace blocks
        if closing_loc.as_slice() != b"end" {
            return;
        }

        let opening_loc = block_node.opening_loc();
        let (opening_line, _) = source.offset_to_line_col(opening_loc.start_offset());

        // Find the indentation of the line containing the block opener.
        let bytes = source.as_bytes();
        let start_of_line_indent = line_indent(bytes, opening_loc.start_offset());

        // For `start_of_line` and `either` styles, RuboCop walks up the
        // expression tree to find the outermost ancestor that starts on a
        // different line. For a chained method like:
        //   @account.things
        //            .where(...)
        //            .in_batches do |b|
        //     ...
        //   end
        // The `end` should align with `@account` (col 2), not `.in_batches` line.
        // Since Prism doesn't give parent pointers, we scan backwards through
        // source lines for continuation patterns (lines starting with `.`).
        let expression_start_indent = find_chain_expression_start(bytes, opening_loc.start_offset());

        // Get the column of `do` keyword itself
        let (_, do_col) = source.offset_to_line_col(opening_loc.start_offset());

        // Find the column of the call expression that owns this block.
        // Walk backward from `do` to find the start of the method call chain.
        let call_expr_col = find_call_expression_col(bytes, opening_loc.start_offset());

        let (end_line, end_col) = source.offset_to_line_col(closing_loc.start_offset());

        // Only flag if end is on a different line
        if end_line == opening_line {
            return;
        }

        match style {
            "start_of_block" => {
                // `end` must align with `do`
                if end_col != do_col {
                    diagnostics.push(self.diagnostic(
                        source,
                        end_line,
                        end_col,
                        "Align `end` with `do`.".to_string(),
                    ));
                }
            }
            "start_of_line" => {
                // `end` must align with start of the expression
                if end_col != expression_start_indent {
                    diagnostics.push(self.diagnostic(
                        source,
                        end_line,
                        end_col,
                        "Align `end` with the start of the line where the block is defined."
                            .to_string(),
                    ));
                }
            }
            _ => {
                // "either" (default): accept alignment with:
                // - the do-line indent, OR
                // - the do keyword column, OR
                // - the expression start indent, OR
                // - the call expression column (for hash-value blocks)
                if end_col != start_of_line_indent
                    && end_col != do_col
                    && end_col != expression_start_indent
                    && end_col != call_expr_col
                {
                    diagnostics.push(self.diagnostic(
                        source,
                        end_line,
                        end_col,
                        "Align `end` with the start of the line where the block is defined."
                            .to_string(),
                    ));
                }
            }
        }

    }
}

/// Check if a line has unclosed parentheses or brackets (more opening than closing).
/// This detects multiline argument lists and array/hash literals.
/// NOTE: We only count `(` and `[`, NOT `{`. Curly braces typically open blocks
/// or hash literals where each line is a separate statement, not a continuation
/// of the outer expression. Including `{` would cause false positives when a
/// `do...end` block is nested inside a brace block (e.g., `lambda { |env| ... }`).
fn line_has_unclosed_bracket(line: &[u8]) -> bool {
    let mut depth: i32 = 0;
    let mut in_single = false;
    let mut in_double = false;
    for &b in line {
        match b {
            b'\'' if !in_double => in_single = !in_single,
            b'"' if !in_single => in_double = !in_double,
            b'(' | b'[' if !in_single && !in_double => depth += 1,
            b')' | b']' if !in_single && !in_double => depth -= 1,
            _ => {}
        }
    }
    depth > 0
}

/// Get the indentation (number of leading spaces) for the line containing the given byte offset.
fn line_indent(bytes: &[u8], offset: usize) -> usize {
    let mut line_start = offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    let mut indent = 0;
    while line_start + indent < bytes.len() && bytes[line_start + indent] == b' ' {
        indent += 1;
    }
    indent
}

/// Walk backward from the `do` keyword on the same line to find the column where
/// the call expression starts. This handles cases like:
///   key: value.map do |x|
///        ^--- call_expr_col (aligned with value.map)
/// Returns the column of the first character of the call expression.
fn find_call_expression_col(bytes: &[u8], do_offset: usize) -> usize {
    // Find start of current line
    let mut line_start = do_offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }

    // Walk backward from `do` to skip whitespace before it
    let mut pos = do_offset;
    while pos > line_start && bytes[pos - 1] == b' ' {
        pos -= 1;
    }

    // Now walk backward through the call expression.
    // We need to handle balanced parens/brackets and stop at unbalanced
    // delimiters or spaces not inside parens.
    let mut paren_depth: i32 = 0;
    while pos > line_start {
        let ch = bytes[pos - 1];
        match ch {
            b')' | b']' => { paren_depth += 1; pos -= 1; }
            b'(' | b'[' => {
                if paren_depth > 0 { paren_depth -= 1; pos -= 1; }
                else { break; }
            }
            _ if paren_depth > 0 => { pos -= 1; } // inside parens, eat everything
            _ if ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'.'
                || ch == b'?' || ch == b'!' || ch == b'@' || ch == b'$' => {
                pos -= 1;
            }
            // `::` namespace separator
            b':' if pos >= 2 + line_start && bytes[pos - 2] == b':' => {
                pos -= 2;
            }
            _ => break,
        }
    }

    pos - line_start
}

/// Walk backwards from the do-line to find the start of the method chain expression.
/// If previous lines are continuations (e.g., starting with `.` or previous line
/// ends with `\`), keep going up.
fn find_chain_expression_start(bytes: &[u8], do_offset: usize) -> usize {
    // Find start of the line containing `do`
    let mut line_start = do_offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }

    // First, check if the do-line itself has more closing brackets than opening.
    // This means the expression started on a previous line (e.g., a multiline %i[] array).
    // If so, scan backwards to find where the bracket was opened.
    {
        let do_line_content = &bytes[line_start..do_offset];
        let bracket_balance = compute_bracket_balance(do_line_content);
        if bracket_balance < 0 {
            // More closing than opening brackets on the do-line.
            // Walk backwards to find the line that opens the bracket.
            let mut depth = bracket_balance;
            let mut search_start = line_start;
            while depth < 0 && search_start > 0 {
                let prev_line_end = search_start - 1;
                let mut prev_line_start = prev_line_end;
                while prev_line_start > 0 && bytes[prev_line_start - 1] != b'\n' {
                    prev_line_start -= 1;
                }
                let prev_content = &bytes[prev_line_start..prev_line_end];
                depth += compute_bracket_balance(prev_content);
                search_start = prev_line_start;
            }
            line_start = search_start;
        }
    }

    // Look at previous lines to check if they're part of the same chain
    loop {
        if line_start == 0 {
            break;
        }
        // Go to previous line
        let prev_line_end = line_start - 1; // the \n
        let mut prev_line_start = prev_line_end;
        while prev_line_start > 0 && bytes[prev_line_start - 1] != b'\n' {
            prev_line_start -= 1;
        }

        // Check if current line (the one at line_start) is a continuation
        // (starts with whitespace + `.`)
        let mut pos = line_start;
        while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
            pos += 1;
        }
        if pos < bytes.len() && bytes[pos] == b'.' {
            // This line starts with `.`, so the expression continues from the previous line
            line_start = prev_line_start;
            continue;
        }

        // Check if previous line ends with `\` (backslash continuation)
        // or ends with `,` (multiline argument list)
        // or has unclosed brackets (multiline literal/args)
        let prev_line_content = &bytes[prev_line_start..prev_line_end];
        let trimmed_end = prev_line_content.iter().rposition(|&b| b != b' ' && b != b'\t' && b != b'\r');
        if let Some(last_non_ws) = trimmed_end {
            let last_byte = prev_line_content[last_non_ws];
            if last_byte == b'\\' || last_byte == b',' {
                line_start = prev_line_start;
                continue;
            }
            // Check if previous line has unclosed brackets (multiline array/hash/args)
            if line_has_unclosed_bracket(prev_line_content) {
                line_start = prev_line_start;
                continue;
            }
        }

        break;
    }

    // Return the indent of the expression start line
    let mut indent = 0;
    while line_start + indent < bytes.len() && bytes[line_start + indent] == b' ' {
        indent += 1;
    }
    indent
}

/// Compute bracket balance for a line (positive = more opening, negative = more closing).
/// Ignores brackets inside strings.
fn compute_bracket_balance(line: &[u8]) -> i32 {
    let mut balance: i32 = 0;
    let mut in_single = false;
    let mut in_double = false;
    for &b in line {
        match b {
            b'\'' if !in_double => in_single = !in_single,
            b'"' if !in_single => in_double = !in_double,
            b'(' | b'[' | b'{' if !in_single && !in_double => balance += 1,
            b')' | b']' | b'}' if !in_single && !in_double => balance -= 1,
            _ => {}
        }
    }
    balance
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(BlockAlignment, "cops/layout/block_alignment");

    #[test]
    fn brace_block_no_offense() {
        let source = b"items.each { |x|\n  puts x\n}\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn start_of_block_style() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyleAlignWith".into(), serde_yml::Value::String("start_of_block".into())),
            ]),
            ..CopConfig::default()
        };
        // `end` aligned with start of line (col 0), not with `do` (col 11)
        let src = b"items.each do |x|\n  puts x\nend\n";
        let diags = run_cop_full_with_config(&BlockAlignment, src, config);
        assert_eq!(diags.len(), 1, "start_of_block should flag end not aligned with do");
        assert!(diags[0].message.contains("do"));
    }
}
