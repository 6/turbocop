use crate::cop::node_type::{
    ARRAY_NODE, FLOAT_NODE, INTEGER_NODE, INTERPOLATED_STRING_NODE, SPLAT_NODE, STRING_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/RedundantSplatExpansion — detects unnecessary `*` on literals.
///
/// ## AllowPercentLiteralArrayArgument handling
///
/// RuboCop's `use_percent_literal_array_argument?` checks
/// `method_argument?(node) && percent_literal?`, where `method_argument?`
/// means `node.parent.call_type?` — i.e., the splat is a direct child of
/// the call's arguments list. When `*%w[...]` appears inside an array
/// literal `[*%w[...]]` that is itself a method argument, the splat's
/// parent is the ArrayNode, not the CallNode, so the exemption does NOT
/// apply. Previously nitrocop skipped ALL percent literal splats
/// unconditionally, causing 17 FN in the corpus (mostly jruby patterns
/// like `assert_in_out_err([*%W"--disable=gems ..."])`).
///
/// ## FP=8 fix: `[]` method calls vs array literals
///
/// `ClassName[*%w[...]]` is a `[]` method call, not an array literal.
/// `find_enclosing_bracket` finds `[` but we must distinguish method call
/// brackets (preceded by identifier/constant/`)`) from array literal
/// brackets (preceded by whitespace/operator/start-of-line). When the `[`
/// is a method call bracket, the splat IS a method argument, so the
/// percent literal exemption applies.
///
/// ## FN=2 fix: `when *%w[...]` patterns
///
/// `when *%w[...]` has no enclosing bracket — `find_enclosing_bracket`
/// returns None. Previously the exemption fired because `!in_array_literal`
/// was true. Now the exemption requires being in a method call context
/// (either `(` bracket or `[]` method call bracket).
///
/// ## FN=2 fix: `*Array.new(...)` patterns
///
/// RuboCop also flags `*Array.new(n)` and `*Array.new(n) { block }` as
/// redundant splat expansions via the `array_new?` / `literal_expansion`
/// matchers. Exemptions mirror RuboCop: `when`/`rescue` clauses skip,
/// multi-element array literals (`[1, *Array.new(n), 2]`) skip, but
/// assignments, method args, single-element arrays, and `[]` method
/// calls are flagged. In Prism, both `Array.new(n)` and
/// `Array.new(n) { block }` are `CallNode`s (block is a child).
///
/// ## Corpus investigation (2026-03-21)
///
/// Corpus oracle reported FP=36, FN=0.
///
/// FP=36: All from `Const[*Array.new(...)]` patterns (e.g.,
/// `Numo::Int32[*Array.new(n) { |n| ... }].transpose.dup`) inside method
/// bodies. RuboCop's `redundant_splat_expansion` has a grandparent check:
/// `return if grandparent && !ASSIGNMENT_TYPES.include?(grandparent.type)`.
/// For `*Array.new` inside a `[]` method call in a method body, the
/// grandparent is the method chain or def node (NOT assignment), so
/// RuboCop skips it. Fix: exempt `*Array.new(...)` when the enclosing
/// bracket is a `[]` method call.
///
/// ## Corpus investigation (2026-03-23)
///
/// FP=10, FN=1. All 10 FPs are `*Array.new(...)` in paren-free method
/// call contexts (e.g., `include *Array.new(3, SomeClass)`,
/// `*Array.new(10) { ... }` inside array literal in method args). RuboCop's
/// grandparent check exempts these because the grandparent is a method
/// def/call node, not an assignment type. Fix: when `find_enclosing_bracket`
/// returns None for `*Array.new`, only flag if preceded by `=` (assignment).
/// Paren-free method calls and other non-assignment contexts are exempt.
/// FN=1 is `NoteSet[*Array.new(n) { ... }]` — a `[]` method call already
/// exempted by the prior FP=36 fix; accepted as tradeoff.
///
/// ## Corpus fix (2026-03-23): FP=6→0, FN=1→0
///
/// Remaining 6 FPs were `*Array.new(...)` inside parenthesized method calls
/// (e.g., `super(*Array.new(9))`, `diagonal(*Array.new(n, value))`,
/// `tmux.send_keys(*Array.new(110) { ... })`). The grandparent check
/// exempted `[]` method call brackets and no-bracket contexts, but not `(`
/// brackets. Refactored to use a match on `find_enclosing_bracket`: `(`
/// brackets without `=` are exempt (method call args), `[` method call
/// brackets without `=` are exempt, no-bracket without `=` is exempt.
/// Only `[` array literals, assignment contexts, and no-bracket with `=`
/// are flagged. Also fixed FN=1: `ns = NoteSet[*Array.new(n) { ... }]` —
/// `[]` method call in assignment context is now correctly flagged because
/// `is_preceded_by_assignment` detects the `=` on the same line.
///
/// ## Corpus fix (2026-03-23): FP=1→0
///
/// FP was `escaped = Foo.const_get("Call").new(*Array.new(10)).send(...)`.
/// `is_preceded_by_assignment` scanned the entire line and found `=` from
/// the outer `escaped = ...` assignment. But the `*Array.new(10)` is inside
/// `.new(...)` — a parenthesized method call — and in RuboCop's AST the
/// grandparent is the outer `.send(...)` call, not an assignment. Fix:
/// `is_preceded_by_assignment_after` now accepts a lower bound (the bracket
/// position) so that for `(` brackets it only scans between the `(` and the
/// splat. An `=` before the enclosing `(` is an outer assignment context and
/// does not cause the splat to be flagged.
/// FN=1 (`end.call(*Array.new(5) { [] })`) is a corpus artifact — RuboCop's
/// grandparent check exempts this pattern (call node is not an assignment type).
pub struct RedundantSplatExpansion;

impl Cop for RedundantSplatExpansion {
    fn name(&self) -> &'static str {
        "Lint/RedundantSplatExpansion"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            FLOAT_NODE,
            INTEGER_NODE,
            INTERPOLATED_STRING_NODE,
            SPLAT_NODE,
            STRING_NODE,
        ]
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
        let allow_percent = config.get_bool("AllowPercentLiteralArrayArgument", true);

        let splat = match node.as_splat_node() {
            Some(s) => s,
            None => return,
        };

        let child = match splat.expression() {
            Some(e) => e,
            None => return,
        };

        // Check if the splat is on a literal: array, string, integer, float
        let is_literal = child.as_array_node().is_some()
            || child.as_string_node().is_some()
            || child.as_integer_node().is_some()
            || child.as_float_node().is_some()
            || child.as_interpolated_string_node().is_some();

        // Check if the splat is on Array.new(...) or Array.new(...) { block }
        let is_array_new = is_array_new_call(&child);

        if !is_literal && !is_array_new {
            return;
        }

        // Array.new has special exemption rules from RuboCop:
        // - NOT flagged in `when` or `rescue` clauses
        // - NOT flagged in multi-element array literals (`[1, *Array.new(n), 2]`)
        // - Flagged in assignments, method args, and single-element array literals
        if is_array_new {
            let bytes = source.as_bytes();
            let start = splat.location().start_offset();

            // Check if inside a multi-element array literal (exempt)
            if is_array_new_inside_multi_element_array(source, &splat) {
                return;
            }

            // Check if in when/rescue context (exempt)
            if is_preceded_by_keyword(bytes, start) {
                return;
            }

            // RuboCop's grandparent check: `return if grandparent &&
            // !ASSIGNMENT_TYPES.include?(grandparent.type)`. The grandparent
            // is only an assignment type for `x = *Array.new(...)`. In all
            // other contexts (method call args, [] method calls, paren-free
            // calls), the grandparent is a non-assignment node and the cop
            // does not fire.
            match find_enclosing_bracket(bytes, start) {
                Some((b'[', bracket_pos)) => {
                    if is_method_call_bracket(bytes, bracket_pos)
                        && !is_preceded_by_assignment(bytes, start)
                    {
                        return; // exempt: Foo[*Array.new(n)] not in assignment
                    }
                    // Inside array literal or assignment — fall through to flag
                }
                Some((b'(', bracket_pos)) => {
                    // Only look for `=` between the enclosing `(` and the splat.
                    // An `=` before the `(` is an outer assignment wrapping the
                    // whole call expression (e.g., `escaped = Foo.new(*Array.new(10))`),
                    // not the splat's assignment context.
                    if !is_preceded_by_assignment_after(bytes, start, Some(bracket_pos)) {
                        return; // exempt: method(*Array.new(n)) not in assignment
                    }
                    // assignment context like `a = (*Array.new(n))` — flag
                }
                None => {
                    if !is_preceded_by_assignment(bytes, start) {
                        return; // exempt: paren-free method call
                    }
                    // Preceded by = — fall through to flag
                }
                _ => return, // other bracket types
            }

            let loc = splat.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let message = "Replace splat expansion with comma separated values.";
            diagnostics.push(self.diagnostic(source, line, column, message.to_string()));
            return;
        }

        // Determine if this is an array splat (child is array) inside an
        // explicit array literal `[...]` — affects both the exemption and message.
        let is_array_splat = child.as_array_node().is_some();
        let in_array_literal = is_array_splat && is_inside_array_literal(source, &splat);

        // When AllowPercentLiteralArrayArgument is true (default), skip
        // percent literal arrays that are direct method arguments.
        // RuboCop checks: method_argument?(node) && percent_literal?
        // This means the splat's parent must be a call node. Examples:
        //   foo(*%w[a b])           → exempt (parent is call via `(`)
        //   Foo[*%w[a b]]           → exempt (parent is [] method call)
        //   method *%w[a b]         → exempt (paren-free method call)
        //   [*%w[a b]]              → NOT exempt (parent is array literal)
        //   when *%w[a b]           → NOT exempt (parent is when clause)
        let is_in_method_call = is_method_call_context(source, &splat);
        if allow_percent && is_array_splat && is_in_method_call {
            if let Some(array_node) = child.as_array_node() {
                if is_percent_literal(&array_node) {
                    return;
                }
            }
        }

        let loc = splat.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        // Use the "pass as separate arguments" message when the splat is
        // in a bracketed call context: `foo(*[...])` or `Foo[*[...]]`.
        // Paren-free calls like `method *[...]` use the default message.
        let is_bracketed_arg = is_array_splat && is_bracketed_call(source, &splat);
        let message = if is_bracketed_arg || in_array_literal {
            "Pass array contents as separate arguments."
        } else {
            "Replace splat expansion with comma separated values."
        };

        diagnostics.push(self.diagnostic(source, line, column, message.to_string()));
    }
}

/// Check if a node is an `Array.new(...)` or `::Array.new(...)` call,
/// including with a block: `Array.new(...) { ... }`.
/// In Prism, `Array.new(5) { block }` is a CallNode with a block child,
/// so both forms are CallNodes.
fn is_array_new_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        return is_array_new_send(&call);
    }
    false
}

/// Check if a CallNode is `Array.new` or `::Array.new`.
fn is_array_new_send(call: &ruby_prism::CallNode<'_>) -> bool {
    if call.name().as_slice() != b"new" {
        return false;
    }
    if let Some(receiver) = call.receiver() {
        // `Array.new` — receiver is ConstantReadNode "Array"
        if let Some(const_read) = receiver.as_constant_read_node() {
            return const_read.name().as_slice() == b"Array";
        }
        // `::Array.new` — receiver is ConstantPathNode with nil parent and "Array" name
        if let Some(const_path) = receiver.as_constant_path_node() {
            if const_path.parent().is_none() {
                return const_path.name().is_some_and(|n| n.as_slice() == b"Array");
            }
        }
    }
    false
}

/// Check if a splat containing Array.new is inside a multi-element array literal.
/// `[1, *Array.new(n), 2]` → exempt (true); `[*Array.new(n)]` → not exempt (false).
fn is_array_new_inside_multi_element_array(
    source: &SourceFile,
    splat: &ruby_prism::SplatNode<'_>,
) -> bool {
    let bytes = source.as_bytes();
    let start = splat.location().start_offset();
    match find_enclosing_bracket(bytes, start) {
        Some((b'[', bracket_pos)) => {
            if is_method_call_bracket(bytes, bracket_pos) {
                // This is Foo[*Array.new(n)] — a method call, not an array literal
                return false;
            }
            // It's an array literal. Check if there are other elements (commas outside the splat).
            has_sibling_elements(bytes, bracket_pos, start, splat.location().end_offset())
        }
        _ => false,
    }
}

/// Check if there are sibling elements in the array literal (i.e., commas outside the splat).
/// `bracket_pos` is the position of `[`, `splat_start`..`splat_end` is the splat range.
fn has_sibling_elements(
    bytes: &[u8],
    bracket_pos: usize,
    splat_start: usize,
    splat_end: usize,
) -> bool {
    // Look for a comma between `[` and the splat start
    let before = &bytes[bracket_pos + 1..splat_start];
    if before.contains(&b',') {
        return true;
    }
    // Look for a comma after the splat end, scanning until we find `]`
    let mut depth = 0i32;
    for &b in &bytes[splat_end..] {
        match b {
            b'[' => depth += 1,
            b']' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            b',' if depth == 0 => return true,
            _ => {}
        }
    }
    false
}

/// Check if an array node is a percent literal (%w, %W, %i, %I).
fn is_percent_literal(array_node: &ruby_prism::ArrayNode<'_>) -> bool {
    if let Some(open_loc) = array_node.opening_loc() {
        let open = open_loc.as_slice();
        return open.starts_with(b"%w")
            || open.starts_with(b"%W")
            || open.starts_with(b"%i")
            || open.starts_with(b"%I");
    }
    false
}

/// Check if the splat is inside an explicit array literal `[...]`.
/// Returns true when the nearest unmatched `[` is an array literal bracket
/// (not a `[]` method call bracket like `Foo[...]`).
fn is_inside_array_literal(source: &SourceFile, splat: &ruby_prism::SplatNode<'_>) -> bool {
    let bytes = source.as_bytes();
    let start = splat.location().start_offset();
    match find_enclosing_bracket(bytes, start) {
        Some((b'[', bracket_pos)) => !is_method_call_bracket(bytes, bracket_pos),
        _ => false,
    }
}

/// Check if the splat is inside a bracketed call: `foo(...)` or `Foo[...]`.
/// Used for choosing the ARRAY_PARAM_MSG message variant.
fn is_bracketed_call(source: &SourceFile, splat: &ruby_prism::SplatNode<'_>) -> bool {
    let bytes = source.as_bytes();
    let start = splat.location().start_offset();
    match find_enclosing_bracket(bytes, start) {
        Some((b'(', _)) => true,
        Some((b'[', bracket_pos)) => is_method_call_bracket(bytes, bracket_pos),
        _ => false,
    }
}

/// Check if the splat is in a method call context (direct arg, `[]` method call,
/// or paren-free method call). Returns false for `when`, `rescue`, and assignments.
fn is_method_call_context(source: &SourceFile, splat: &ruby_prism::SplatNode<'_>) -> bool {
    let bytes = source.as_bytes();
    let start = splat.location().start_offset();
    match find_enclosing_bracket(bytes, start) {
        Some((b'(', _)) => true,
        Some((b'[', bracket_pos)) => is_method_call_bracket(bytes, bracket_pos),
        _ => {
            // No enclosing bracket. Check if this is a paren-free method call
            // or a non-method context like `when`/`rescue`/assignment.
            !is_preceded_by_keyword(bytes, start)
        }
    }
}

/// Check if the `*` at `pos` is preceded (on the same line, skipping whitespace
/// and commas+args) by a `when` or `rescue` keyword, indicating a non-method context.
/// Also returns true for assignment operators (`=`).
fn is_preceded_by_keyword(bytes: &[u8], pos: usize) -> bool {
    // Scan backwards to find the start of the statement on this line
    let mut i = pos;
    while i > 0 {
        i -= 1;
        if bytes[i] == b'\n' {
            break;
        }
    }
    // Extract the text before the `*` on this line
    let start = if bytes.get(i) == Some(&b'\n') {
        i + 1
    } else {
        i
    };
    let before = &bytes[start..pos];
    // Trim leading whitespace
    let trimmed = trim_leading_whitespace(before);
    trimmed.starts_with(b"when ")
        || trimmed.starts_with(b"rescue ")
        || trimmed.starts_with(b"rescue\t")
}

/// Check if the `*` at `pos` is preceded (on the same line) by an assignment
/// operator (`=`), indicating `x = *Array.new(...)` context.
/// When `lower_bound` is provided, only scans between that position and `pos`,
/// so that an `=` before the enclosing bracket is not counted.
fn is_preceded_by_assignment(bytes: &[u8], pos: usize) -> bool {
    is_preceded_by_assignment_after(bytes, pos, None)
}

/// Like `is_preceded_by_assignment`, but only scans from `lower_bound` (exclusive)
/// to `pos`. If `lower_bound` is None, scans back to the start of the line.
fn is_preceded_by_assignment_after(bytes: &[u8], pos: usize, lower_bound: Option<usize>) -> bool {
    let stop = lower_bound.unwrap_or(0);
    let mut i = pos;
    while i > stop {
        i -= 1;
        match bytes[i] {
            b'\n' => break,
            b'=' => {
                // Make sure it's not `==`, `!=`, `<=`, `>=`, `=>`
                if i > 0 && matches!(bytes[i - 1], b'=' | b'!' | b'<' | b'>') {
                    continue;
                }
                // Make sure the next char isn't `=` or `>` (for `==` or `=>`)
                if i + 1 < bytes.len() && matches!(bytes[i + 1], b'=' | b'>') {
                    continue;
                }
                return true;
            }
            _ => {}
        }
    }
    false
}

fn trim_leading_whitespace(bytes: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    &bytes[i..]
}

/// Check if a `[` at the given position is a method call bracket (e.g., `Foo[`).
/// A method call bracket is preceded (ignoring whitespace) by an identifier char,
/// `)`, `]`, `?`, or `!` — indicating the `[` is the `[]` method on a receiver.
/// An array literal bracket is preceded by an operator, `,`, `(`, `[`, `=`, or
/// appears at the start of an expression.
fn is_method_call_bracket(bytes: &[u8], bracket_pos: usize) -> bool {
    // Scan backwards from the `[` to find the first non-whitespace character
    let mut i = bracket_pos;
    while i > 0 {
        i -= 1;
        match bytes[i] {
            b' ' | b'\t' => continue,
            // Identifier-like characters: the `[` is a method call on a receiver
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b')' | b']' | b'?' | b'!' => {
                return true;
            }
            // Instance/class variable prefix followed by `[` (e.g., @cmd[...])
            b'@' => return true,
            // Anything else (operator, comma, paren, newline, etc.) means array literal
            _ => return false,
        }
    }
    // Start of file — array literal
    false
}

/// Scan backwards from `pos` to find the nearest unmatched `[` or `(`,
/// tracking bracket nesting. Returns the bracket character and its position, or None.
fn find_enclosing_bracket(bytes: &[u8], pos: usize) -> Option<(u8, usize)> {
    let mut depth_square: i32 = 0;
    let mut depth_paren: i32 = 0;
    let mut i = pos;
    while i > 0 {
        i -= 1;
        match bytes[i] {
            b']' => depth_square += 1,
            b'[' => {
                if depth_square == 0 {
                    return Some((b'[', i));
                }
                depth_square -= 1;
            }
            b')' => depth_paren += 1,
            b'(' => {
                if depth_paren == 0 {
                    return Some((b'(', i));
                }
                depth_paren -= 1;
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantSplatExpansion,
        "cops/lint/redundant_splat_expansion"
    );
}
