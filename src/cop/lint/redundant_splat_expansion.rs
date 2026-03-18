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

        if !is_literal {
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
