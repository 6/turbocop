use crate::cop::node_type::{IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Flags `if cond; body` when a semicolon separates the condition from the body.
///
/// RuboCop flags ALL `if`/`unless` statements where `loc.begin` is `;` (the "then"
/// keyword is a semicolon), regardless of whether the construct is single-line or
/// multi-line. The only exceptions are:
/// - Modifier form (`body if cond`) — no begin/end keywords
/// - `node.parent&.if_type?` — the `if` is nested inside another `if` node's
///   branch (covers `else if` patterns)
///
/// ## Corpus investigation (2026-03-23)
///
/// Corpus oracle reported FP=3, FN=39.
///
/// FP=3: All in rubyworks/facets — `else if @im>0;` patterns where the inner `if`
/// is nested inside another if's else branch. RuboCop skips these via
/// `node.parent&.if_type?`. Fixed by checking if `else` precedes the `if` keyword
/// on the same source line.
///
/// FN=39: The cop previously required `end` on same line as `if` (single-line only).
/// RuboCop flags ALL `if/unless` with semicolon then-keyword, including multi-line
/// `if cond;\n  body\nend`. Fixed by removing the same-line `end` check. Also added
/// UNLESS_NODE to interested_node_types to handle `unless cond;` patterns.
///
/// ## Corpus investigation (2026-03-23, round 2)
///
/// FP=16, FN=0. All FPs were multi-line `if`/`unless` where a comment after the
/// condition contained a semicolon (e.g., `if cond # comment; more comment`).
/// The fallback `has_semicolon_between` scan was including comment text. Fixed by
/// stopping the scan at `#` (Ruby comment start) in addition to newline.
pub struct IfWithSemicolon;

impl Cop for IfWithSemicolon {
    fn name(&self) -> &'static str {
        "Style/IfWithSemicolon"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if let Some(if_node) = node.as_if_node() {
            check_if_node(self, source, &if_node, diagnostics);
        } else if let Some(unless_node) = node.as_unless_node() {
            check_unless_node(self, source, &unless_node, diagnostics);
        }
    }
}

fn has_semicolon_between(source: &SourceFile, pred_end: usize, body_start: usize) -> bool {
    if pred_end < body_start {
        let between = &source.content[pred_end..body_start];
        // Only check up to first newline, and stop at `#` (comment start) —
        // semicolons inside comments should not trigger this cop.
        between
            .iter()
            .take_while(|&&b| b != b'\n' && b != b'#')
            .any(|&b| b == b';')
    } else {
        false
    }
}

fn check_if_node(
    cop: &IfWithSemicolon,
    source: &SourceFile,
    if_node: &ruby_prism::IfNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Must have an `if` keyword (not ternary)
    let if_kw_loc = match if_node.if_keyword_loc() {
        Some(loc) => loc,
        None => return,
    };

    let kw_bytes = if_kw_loc.as_slice();
    if kw_bytes != b"if" {
        return;
    }

    // Must not be modifier form (modifier has no end keyword)
    if if_node.end_keyword_loc().is_none() {
        return;
    }

    // Skip `else if` patterns (RuboCop: node.parent&.if_type?)
    if is_preceded_by_else(source, if_kw_loc.start_offset()) {
        return;
    }

    // Check for semicolon: Prism's then_keyword_loc is ";" or "then".
    // Fallback: scan between predicate and body for semicolons.
    let has_semicolon = if let Some(then_loc) = if_node.then_keyword_loc() {
        then_loc.as_slice() == b";"
    } else {
        let pred_end = if_node.predicate().location().end_offset();
        let body_start = if let Some(stmts) = if_node.statements() {
            stmts.location().start_offset()
        } else if let Some(sub) = if_node.subsequent() {
            sub.location().start_offset()
        } else if let Some(end_loc) = if_node.end_keyword_loc() {
            end_loc.start_offset()
        } else {
            return;
        };
        has_semicolon_between(source, pred_end, body_start)
    };

    if !has_semicolon {
        return;
    }

    let loc = if_node.location();
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    let cond_src = std::str::from_utf8(if_node.predicate().location().as_slice()).unwrap_or("...");

    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        format!("Do not use `if {};` - use a newline instead.", cond_src),
    ));
}

fn check_unless_node(
    cop: &IfWithSemicolon,
    source: &SourceFile,
    unless_node: &ruby_prism::UnlessNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Must not be modifier form (modifier has no end keyword)
    if unless_node.end_keyword_loc().is_none() {
        return;
    }

    // Check for semicolon
    let has_semicolon = if let Some(then_loc) = unless_node.then_keyword_loc() {
        then_loc.as_slice() == b";"
    } else {
        let pred_end = unless_node.predicate().location().end_offset();
        let body_start = if let Some(stmts) = unless_node.statements() {
            stmts.location().start_offset()
        } else if let Some(end_loc) = unless_node.end_keyword_loc() {
            end_loc.start_offset()
        } else {
            return;
        };
        has_semicolon_between(source, pred_end, body_start)
    };

    if !has_semicolon {
        return;
    }

    let loc = unless_node.location();
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    let cond_src =
        std::str::from_utf8(unless_node.predicate().location().as_slice()).unwrap_or("...");

    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        format!("Do not use `unless {};` - use a newline instead.", cond_src),
    ));
}

/// Check if the `if`/`unless` keyword at the given offset is preceded by `else`
/// on the same source line (indicating an `else if` pattern).
fn is_preceded_by_else(source: &SourceFile, if_kw_offset: usize) -> bool {
    let content = &source.content;
    if if_kw_offset == 0 {
        return false;
    }

    let mut line_start = if_kw_offset;
    while line_start > 0 && content[line_start - 1] != b'\n' {
        line_start -= 1;
    }

    let before_kw = &content[line_start..if_kw_offset];
    let trimmed = before_kw
        .iter()
        .rev()
        .skip_while(|&&b| b == b' ' || b == b'\t')
        .collect::<Vec<_>>();

    if trimmed.len() >= 4 {
        let last4: Vec<u8> = trimmed[..4].iter().rev().map(|&&b| b).collect();
        if &last4 == b"else" {
            if trimmed.len() == 4 {
                return true;
            }
            let before_else = *trimmed[4];
            return before_else == b' '
                || before_else == b'\t'
                || before_else == b';'
                || before_else == b'\n';
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IfWithSemicolon, "cops/style/if_with_semicolon");

    #[test]
    fn single_line_if_semicolon() {
        let source = b"if foo; bar end\n";
        let diags = crate::testutil::run_cop_full(&IfWithSemicolon, source);
        assert_eq!(diags.len(), 1, "Should flag 'if foo; bar end'");
    }

    #[test]
    fn multiline_unless_semicolon() {
        let source = b"unless done;\n  process\nend\n";
        let diags = crate::testutil::run_cop_full(&IfWithSemicolon, source);
        assert_eq!(diags.len(), 1, "Should flag 'unless done;'");
    }
}
