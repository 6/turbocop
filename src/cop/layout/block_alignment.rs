use crate::cop::node_type::BLOCK_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks whether the end keywords / closing braces are aligned properly for
/// do..end and {..} blocks.
///
/// ## Corpus investigation findings (2026-03-11)
///
/// Root causes of 1,187 FP:
/// 1. **Trailing-dot method chains** — `find_chain_expression_start` only checked
///    for lines starting with `.` (leading dot) but NOT for lines ending with `.`
///    (trailing dot style). This caused the chain root to not be found, computing
///    wrong `expression_start_indent` and flagging correctly-aligned `end`.
/// 2. **Tab indentation** — `line_indent` only counted spaces, returning 0 for
///    tab-indented lines. But `offset_to_line_col` counts tabs as 1 character,
///    causing a mismatch between computed indent and actual `end` column.
/// 3. **Missing `begins_its_line?` check** — RuboCop skips alignment checks when
///    `end`/`}` is not the first non-whitespace on its line (e.g., `end.select`).
///    nitrocop checked all `end` keywords regardless.
///
/// Root causes of 334 FN:
/// 1. **Brace blocks not checked** — RuboCop checks both `do..end` and `{..}`
///    blocks, but nitrocop only checked `do..end`. Many FNs were misaligned `}`.
///
/// Fixes applied:
/// - `line_indent` now counts both spaces and tabs
/// - `find_chain_expression_start` now handles trailing-dot chains (lines ending with `.`)
/// - Added `begins_its_line` check to skip non-line-beginning closers
/// - Added brace block (`{..}`) checking with same alignment rules
/// - Fixed `start_of_block` style to use do-line indent (not `do` column) per RuboCop spec
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
        let closing_slice = closing_loc.as_slice();
        let is_do_end = closing_slice == b"end";
        let is_brace = closing_slice == b"}";
        if !is_do_end && !is_brace {
            return;
        }

        // RuboCop's begins_its_line? check: only inspect alignment when the
        // closing keyword/brace is the first non-whitespace on its line.
        let bytes = source.as_bytes();
        if !begins_its_line(bytes, closing_loc.start_offset()) {
            return;
        }

        let opening_loc = block_node.opening_loc();
        let (opening_line, _) = source.offset_to_line_col(opening_loc.start_offset());

        // Find the indentation of the line containing the block opener.
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
        let expression_start_indent =
            find_chain_expression_start(bytes, opening_loc.start_offset());

        // Get the column of `do`/`{` keyword itself
        let (_, do_col) = source.offset_to_line_col(opening_loc.start_offset());

        // Find the column of the call expression that owns this block.
        // Walk backward from `do`/`{` to find the start of the method call chain.
        let call_expr_col = find_call_expression_col(bytes, opening_loc.start_offset());

        let (end_line, end_col) = source.offset_to_line_col(closing_loc.start_offset());

        // Only flag if closing is on a different line than opening
        if end_line == opening_line {
            return;
        }

        let close_word = if is_brace { "`}`" } else { "`end`" };
        let open_word = if is_brace { "`{`" } else { "`do`" };

        match style {
            "start_of_block" => {
                // closing must align with do/{-line indent (first non-ws on that line)
                if end_col != start_of_line_indent {
                    diagnostics.push(self.diagnostic(
                        source,
                        end_line,
                        end_col,
                        format!("Align {} with {}.", close_word, open_word),
                    ));
                }
            }
            "start_of_line" => {
                // closing must align with start of the expression
                if end_col != expression_start_indent {
                    diagnostics.push(self.diagnostic(
                        source,
                        end_line,
                        end_col,
                        format!(
                            "Align {} with the start of the line where the block is defined.",
                            close_word
                        ),
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
                        format!(
                            "Align {} with the start of the line where the block is defined.",
                            close_word
                        ),
                    ));
                }
            }
        }
    }
}

/// Check if a byte offset is at the beginning of its line (only whitespace before it).
/// Matches RuboCop's `begins_its_line?` helper.
fn begins_its_line(bytes: &[u8], offset: usize) -> bool {
    let mut pos = offset;
    while pos > 0 && bytes[pos - 1] != b'\n' {
        pos -= 1;
        if bytes[pos] != b' ' && bytes[pos] != b'\t' {
            return false;
        }
    }
    true
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

/// Get the indentation (number of leading whitespace characters) for the line
/// containing the given byte offset. Counts both spaces and tabs as 1 character
/// each to match `offset_to_line_col` which uses character (codepoint) offsets.
fn line_indent(bytes: &[u8], offset: usize) -> usize {
    let mut line_start = offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    let mut indent = 0;
    while line_start + indent < bytes.len()
        && (bytes[line_start + indent] == b' ' || bytes[line_start + indent] == b'\t')
    {
        indent += 1;
    }
    indent
}

/// Walk backward from the `do` keyword on the same line to find the column where
/// the call expression starts. This handles cases like:
///   key: value.map do |x|
///        ^--- call_expr_col (aligned with value.map)
///
/// When the block is on the RHS of an assignment (=, +=, ||=, etc.), this
/// continues walking backward through the assignment operator to find the LHS
/// variable, matching RuboCop's behavior of aligning with the assignment target.
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
            b')' | b']' => {
                paren_depth += 1;
                pos -= 1;
            }
            b'(' | b'[' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                    pos -= 1;
                } else {
                    break;
                }
            }
            _ if paren_depth > 0 => {
                pos -= 1;
            } // inside parens, eat everything
            _ if ch.is_ascii_alphanumeric()
                || ch == b'_'
                || ch == b'.'
                || ch == b'?'
                || ch == b'!'
                || ch == b'@'
                || ch == b'$' =>
            {
                pos -= 1;
            }
            // `::` namespace separator
            b':' if pos >= 2 + line_start && bytes[pos - 2] == b':' => {
                pos -= 2;
            }
            _ => break,
        }
    }

    // Check if we stopped at an assignment operator. If so, continue backward
    // through it to find the LHS variable (RuboCop aligns with the assignment target).
    let call_pos = pos;
    if call_pos > line_start {
        let after_call = skip_assignment_backward(bytes, line_start, call_pos);
        if after_call != call_pos {
            return after_call - line_start;
        }
    }

    pos - line_start
}

/// If `pos` points just after a call expression and there's an assignment
/// operator (=, +=, -=, *=, /=, ||=, &&=, <<=, >>=, etc.) before it,
/// skip backward through the operator and whitespace, then walk backward
/// through the LHS identifier to find the assignment target.
/// Returns the new position (start of LHS), or `pos` unchanged if no
/// assignment is found.
fn skip_assignment_backward(bytes: &[u8], line_start: usize, pos: usize) -> usize {
    // Skip whitespace before the call expression
    let mut p = pos;
    while p > line_start && bytes[p - 1] == b' ' {
        p -= 1;
    }

    // Check for assignment operator ending with '='
    if p > line_start && bytes[p - 1] == b'=' {
        // Could be =, +=, -=, *=, /=, ||=, &&=, <<=, >>=, %=, **=, ^=
        // But NOT ==, !=, <=, >=
        let eq_pos = p - 1;
        let mut op_start = eq_pos;

        if op_start > line_start {
            let prev = bytes[op_start - 1];
            match prev {
                b'+' | b'-' | b'*' | b'/' | b'%' | b'^' => {
                    op_start -= 1;
                }
                b'|' if op_start >= 2 + line_start && bytes[op_start - 2] == b'|' => {
                    op_start -= 2;
                }
                b'&' if op_start >= 2 + line_start && bytes[op_start - 2] == b'&' => {
                    op_start -= 2;
                }
                b'<' if op_start >= 2 + line_start && bytes[op_start - 2] == b'<' => {
                    op_start -= 2;
                }
                b'>' if op_start >= 2 + line_start && bytes[op_start - 2] == b'>' => {
                    op_start -= 2;
                }
                b'*' if op_start >= 2 + line_start && bytes[op_start - 2] == b'*' => {
                    op_start -= 2;
                }
                // Bare `=` — but reject `==`, `!=`, `<=`, `>=`
                b'=' | b'!' | b'<' | b'>' => {
                    return pos; // Not a simple assignment
                }
                _ => {
                    // Bare `=` with a non-operator char before it — this is a simple assignment
                }
            }
        }

        // Skip whitespace before the operator
        let mut lhs_end = op_start;
        while lhs_end > line_start && bytes[lhs_end - 1] == b' ' {
            lhs_end -= 1;
        }

        // Walk backward through the LHS identifier (variable, ivar, cvar, etc.)
        let mut lhs_pos = lhs_end;
        while lhs_pos > line_start {
            let ch = bytes[lhs_pos - 1];
            if ch.is_ascii_alphanumeric()
                || ch == b'_'
                || ch == b'@'
                || ch == b'$'
                || ch == b'.'
                || ch == b'['
                || ch == b']'
            {
                lhs_pos -= 1;
            } else if ch == b':' && lhs_pos >= 2 + line_start && bytes[lhs_pos - 2] == b':' {
                lhs_pos -= 2;
            } else if ch == b',' {
                // Multi-assignment: `a, b = ...` — continue to find the first variable
                lhs_pos -= 1;
                while lhs_pos > line_start && bytes[lhs_pos - 1] == b' ' {
                    lhs_pos -= 1;
                }
            } else {
                break;
            }
        }

        if lhs_pos < lhs_end {
            return lhs_pos;
        }
    }

    pos
}

/// Walk backwards from the do-line to find the start of the method chain expression.
/// If previous lines are continuations (e.g., starting with `.` or previous line
/// ends with `\` or `.`), keep going up.
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

        // Check if previous line ends with `\` (backslash continuation),
        // `.` or `&.` (trailing dot method chain), `,` (multiline argument list),
        // `+` or other binary operators (string concatenation, logical operators),
        // or has unclosed brackets (multiline literal/args).
        let prev_line_content = &bytes[prev_line_start..prev_line_end];
        let trimmed_end = prev_line_content
            .iter()
            .rposition(|&b| b != b' ' && b != b'\t' && b != b'\r');
        if let Some(last_non_ws) = trimmed_end {
            let last_byte = prev_line_content[last_non_ws];
            if last_byte == b'\\' || last_byte == b',' || last_byte == b'.' || last_byte == b'+' {
                line_start = prev_line_start;
                continue;
            }
            // Check for trailing logical operators: ||, &&
            // Single | or & could be block parameter delimiter or block-pass, so only
            // match double operators.
            if last_byte == b'|' && last_non_ws > 0 && prev_line_content[last_non_ws - 1] == b'|' {
                line_start = prev_line_start;
                continue;
            }
            if last_byte == b'&' && last_non_ws > 0 && prev_line_content[last_non_ws - 1] == b'&' {
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

    // Return the indent of the expression start line (count both spaces and tabs)
    let mut indent = 0;
    while line_start + indent < bytes.len()
        && (bytes[line_start + indent] == b' ' || bytes[line_start + indent] == b'\t')
    {
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
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyleAlignWith".into(),
                serde_yml::Value::String("start_of_block".into()),
            )]),
            ..CopConfig::default()
        };
        // In start_of_block style, `end` must align with the do-line indent
        // (first non-ws on the do-line), not the `do` keyword column.
        // For `items.each do |x|`, do-line indent = 0, so end at col 0 is fine.
        let src = b"items.each do |x|\n  puts x\nend\n";
        let diags = run_cop_full_with_config(&BlockAlignment, src, config.clone());
        assert!(
            diags.is_empty(),
            "start_of_block: end at col 0 matches do-line indent 0. Got: {:?}",
            diags
        );

        // But end at col 2 should be flagged (doesn't match do-line indent 0)
        let src2 = b"items.each do |x|\n  puts x\n  end\n";
        let diags2 = run_cop_full_with_config(&BlockAlignment, src2, config.clone());
        assert_eq!(
            diags2.len(),
            1,
            "start_of_block should flag end at col 2 (doesn't match do-line indent 0)"
        );

        // Chained: .each do at col 2, end should align at col 2
        let src3 = b"foo.bar\n  .each do\n    baz\n  end\n";
        let diags3 = run_cop_full_with_config(&BlockAlignment, src3, config.clone());
        assert!(
            diags3.is_empty(),
            "start_of_block: end at col 2 matches .each do line indent. Got: {:?}",
            diags3
        );

        // Chained: .each do at col 2, end at col 0 should flag
        let src4 = b"foo.bar\n  .each do\n    baz\nend\n";
        let diags4 = run_cop_full_with_config(&BlockAlignment, src4, config);
        assert_eq!(
            diags4.len(),
            1,
            "start_of_block: end at col 0 doesn't match .each do line indent 2"
        );
    }

    // FP fix: trailing-dot method chains
    #[test]
    fn no_offense_trailing_dot_chain() {
        let source =
            b"all_objects.flat_map { |o| o }.\n  uniq(&:first).each do |a, o|\n  process(a, o)\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Trailing dot chain: end should align with chain root. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_trailing_dot_chain_indented() {
        let source = b"def foo\n  objects.flat_map { |o| o }.\n    uniq.each do |item|\n    process(item)\n  end\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Indented trailing dot chain: end at col 2 matches chain start at col 2. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_trailing_dot_multi_line() {
        let source = b"  records.\n    where(active: true).\n    order(:name).each do |r|\n    process(r)\n  end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Multi trailing dot: end at col 2 matches chain root at col 2. Got: {:?}",
            diags
        );
    }

    // FP fix: tab indentation
    #[test]
    fn no_offense_tab_indented_block() {
        let source = b"if true\n\titems.each do\n\t\tputs 'hello'\n\tend\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Tab-indented block should not be flagged. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_tab_indented_assignment_block() {
        let source = b"\tvariable = test do |x|\n\t\tx.to_s\n\tend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Tab-indented assignment block should not be flagged. Got: {:?}",
            diags
        );
    }

    // FP fix: begins_its_line check
    #[test]
    fn fp_end_not_beginning_its_line() {
        // end.select is at start of line (after whitespace) but has continuation
        // The first block's end should not be checked since it has .select after it
        let source = b"def foo(bar)\n  bar.get_stuffs\n      .reject do |stuff|\n        stuff.long_expr\n      end.select do |stuff|\n        stuff.other\n      end\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Should not flag end that doesn't begin its line. Got: {:?}",
            diags
        );
    }

    // FN fix: brace block misalignment
    #[test]
    fn offense_brace_block_misaligned() {
        let source = b"test {\n  stuff\n  }\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert_eq!(
            diags.len(),
            1,
            "Misaligned brace block should be flagged. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_brace_block_aligned() {
        let source = b"test {\n  stuff\n}\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "Aligned brace block should not be flagged. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_brace_block_not_beginning_line() {
        let source = b"scope :bar, lambda { joins(:baz)\n                     .distinct }\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "closing brace not beginning its line should not be flagged"
        );
    }

    // Other patterns from RuboCop spec
    #[test]
    fn no_offense_variable_assignment() {
        let source = b"variable = test do |ala|\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "end aligned with variable start. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_op_asgn() {
        let source = b"rb += files.select do |file|\n  file << something\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(diags.is_empty(), "end aligned with rb. Got: {:?}", diags);
    }

    #[test]
    fn no_offense_logical_operand() {
        let source = b"(value.is_a? Array) && value.all? do |subvalue|\n  type_check_value(subvalue, array_type)\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "end aligns with expression start. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_send_shovel() {
        let source = b"parser.children << lambda do |token|\n  token << 1\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "end aligns with parser.children. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_chain_pretty_alignment() {
        let source = b"def foo(bar)\n  bar.get_stuffs\n      .reject do |stuff|\n        stuff.long_expr\n      end\n      .select do |stuff|\n        stuff.other\n      end\nend\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "end at col 6 matches do-line indent. Got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_next_line_assignment() {
        let source = b"variable =\n  a_long_method do |v|\n    v.foo\n  end\n";
        let diags = run_cop_full(&BlockAlignment, source);
        assert!(
            diags.is_empty(),
            "end aligns with a_long_method. Got: {:?}",
            diags
        );
    }
}
