use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Flags `if cond; body` when a semicolon separates the condition from the body.
///
/// RuboCop flags ALL `if`/`unless` statements where `loc.begin` is `;` (the "then"
/// keyword is a semicolon), regardless of whether the construct is single-line or
/// multi-line. The exceptions are:
/// - Modifier form (`body if cond`) — no begin/end keywords
/// - `node.parent&.if_type?` — the `if` is nested inside another `if` node's
///   branch (covers `else if` patterns)
/// - `part_of_ignored_node?` — after flagging an `if`/`unless` with semicolon,
///   RuboCop calls `ignore_node(node)` which suppresses all nested `if`/`unless`
///   nodes inside the flagged node's source range.
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
///
/// ## Corpus investigation (2026-03-24, round 3)
///
/// FP=5, FN=0. All 5 FPs in rubyworks/facets `work/consider/standard/quaternion.rb`.
/// These are nested `if cond;` inside an outer `if`/`elsif` that also uses semicolons.
/// RuboCop suppresses these via `ignore_node`/`part_of_ignored_node?`: once an `if`
/// with semicolon is flagged, all `if`/`unless` nodes within its source range are
/// skipped. Fixed by switching from `check_node` to `check_source` with a visitor
/// that tracks the end offset of flagged nodes, suppressing any nested semicolon
/// `if`/`unless` within that range.
pub struct IfWithSemicolon;

impl Cop for IfWithSemicolon {
    fn name(&self) -> &'static str {
        "Style/IfWithSemicolon"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = IfWithSemicolonVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            ignored_end_offset: 0,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct IfWithSemicolonVisitor<'a> {
    cop: &'a IfWithSemicolon,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// End offset of the most recently flagged `if`/`unless` node.
    /// Any node starting before this offset is inside a flagged node and should be skipped
    /// (replicates RuboCop's `ignore_node`/`part_of_ignored_node?` mechanism).
    ignored_end_offset: usize,
}

impl<'pr> Visit<'pr> for IfWithSemicolonVisitor<'_> {
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        self.check_if_node(node);
        // Continue visiting child nodes
        ruby_prism::visit_if_node(self, node);
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        self.check_unless_node(node);
        // Continue visiting child nodes
        ruby_prism::visit_unless_node(self, node);
    }
}

impl IfWithSemicolonVisitor<'_> {
    fn check_if_node(&mut self, if_node: &ruby_prism::IfNode<'_>) {
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
        if is_preceded_by_else(self.source, if_kw_loc.start_offset()) {
            return;
        }

        // Skip if inside a previously flagged node (RuboCop: part_of_ignored_node?)
        let loc = if_node.location();
        if loc.start_offset() < self.ignored_end_offset {
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
            has_semicolon_between(self.source, pred_end, body_start)
        };

        if !has_semicolon {
            return;
        }

        // Flag this node and mark its range as ignored for descendants
        self.ignored_end_offset = loc.end_offset();

        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        let cond_src =
            std::str::from_utf8(if_node.predicate().location().as_slice()).unwrap_or("...");

        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            format!("Do not use `if {};` - use a newline instead.", cond_src),
        ));
    }

    fn check_unless_node(&mut self, unless_node: &ruby_prism::UnlessNode<'_>) {
        // Must not be modifier form (modifier has no end keyword)
        if unless_node.end_keyword_loc().is_none() {
            return;
        }

        // Skip if inside a previously flagged node (RuboCop: part_of_ignored_node?)
        let loc = unless_node.location();
        if loc.start_offset() < self.ignored_end_offset {
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
            has_semicolon_between(self.source, pred_end, body_start)
        };

        if !has_semicolon {
            return;
        }

        // Flag this node and mark its range as ignored for descendants
        self.ignored_end_offset = loc.end_offset();

        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        let cond_src =
            std::str::from_utf8(unless_node.predicate().location().as_slice()).unwrap_or("...");

        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            format!("Do not use `unless {};` - use a newline instead.", cond_src),
        ));
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

    #[test]
    fn nested_if_with_semicolon_suppressed() {
        // Outer if with semicolon is flagged; inner if with semicolon is suppressed
        let source = b"if is_real?;\n  if @re>=0; return foo\n  else return bar\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&IfWithSemicolon, source);
        assert_eq!(
            diags.len(),
            1,
            "Should only flag outer 'if is_real?;', not nested 'if @re>=0;'"
        );
    }

    #[test]
    fn nested_if_inside_elsif_suppressed() {
        // Outer if with semicolon, elsif with semicolon, nested if with semicolon
        let source =
            b"if a; foo\nelsif b;\n  if c; bar\n  elsif d; baz\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&IfWithSemicolon, source);
        assert_eq!(
            diags.len(),
            1,
            "Should only flag outer 'if a;', nested ifs inside are suppressed"
        );
    }

    #[test]
    fn sibling_ifs_both_flagged() {
        // Two sequential (non-nested) if statements with semicolons should both be flagged
        let source = b"if a; foo end\nif b; bar end\n";
        let diags = crate::testutil::run_cop_full(&IfWithSemicolon, source);
        assert_eq!(diags.len(), 2, "Both sequential ifs should be flagged");
    }
}
