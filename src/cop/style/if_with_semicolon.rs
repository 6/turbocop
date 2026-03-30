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
///
/// ## Corpus investigation (2026-03-25, round 4)
///
/// FP=5, FN=2.
///
/// FN=1 (real): floraison/fugit `cron.rb:875` — `else if at; zt = tt; else; at = tt; end`
/// inside a `case/when/else` block. The `else` belongs to the `case`, not to an `if`.
/// The old text-based `is_preceded_by_else` check incorrectly treated this as an
/// `else if` pattern and skipped it. Fixed by replacing the text-based check with an
/// AST-based approach: during visitation, each if/unless node registers its else-branch
/// child if-node's start offset in `else_if_offsets`. Only if-nodes in that set are
/// skipped, correctly distinguishing `case else if` from `if else if`.
///
/// FN=1 (not real): waagsociety/citysdk-ld `filters.rb:181` — `if cond\n  ;\nelse`.
/// The `;` is on a separate line as the body, not between condition and body. Tested
/// with RuboCop directly: RuboCop does NOT flag this. Corpus artifact.
///
/// ## Corpus investigation (2026-03-27, round 5)
///
/// FP=5, FN=1.
///
/// FP=5: All in victords/minigl (3) and jjyg/metasm (2). These are `if cond; body`
/// patterns that are the sole statement in another if/unless node's branch. In the
/// parser gem (used by RuboCop), a sole branch statement's parent IS the if node,
/// so `node.parent&.if_type?` returns true and RuboCop skips them. With multiple
/// statements, they're wrapped in a `begin` node and the check is false.
/// Previously we only handled the else-branch case (`else if` patterns). Fixed by
/// generalizing `else_if_offsets` → `parent_is_if_offsets` to register sole-child
/// if/unless nodes in ALL branches (if-branch, elsif-branches, else-branch) of
/// parent if/unless nodes.
///
/// FN=1 (not real): waagsociety/citysdk-ld `filters.rb:181` — same as round 4.
/// Corpus artifact (`;` is body, not then-keyword).
///
/// ## Corpus reinvestigation (2026-03-30)
///
/// Cached oracle data reports waagsociety/citysdk-ld `filters.rb:181`
/// (`if cond\n  ;\nelse`) as an FN. Prior investigation concluded this was a
/// corpus artifact because the classic parser gem returns `loc.begin = nil`.
/// However, the oracle uses `TargetRubyVersion: 4.0` which activates Prism's
/// Translation Parser — and Prism::Translation::Parser sets `loc.begin` to the
/// standalone `;` on the next line (even though Prism's native `then_keyword_loc`
/// is nil). This is a real FN. Fixed by adding `has_semicolon_multiline` which
/// scans across newlines (skipping comments) when `statements` is nil (empty
/// if-body), catching the standalone `;` that Prism Translation treats as the
/// then-keyword.
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
            parent_is_if_offsets: Vec::new(),
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
    /// Start offsets of `if`/`unless` nodes that are the sole statement in any branch
    /// of another `if`/`unless` node. These should be skipped per RuboCop's
    /// `node.parent&.if_type?` check — in the parser gem, a sole branch statement's
    /// parent is the if node itself, while multiple statements are wrapped in `begin`.
    parent_is_if_offsets: Vec<usize>,
}

impl<'pr> Visit<'pr> for IfWithSemicolonVisitor<'_> {
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        // Before checking/visiting, register sole-child if/unless nodes in
        // all branches so they get skipped (mirrors RuboCop's `node.parent&.if_type?`).
        self.register_children_of_if(node);
        self.check_if_node(node);
        // Continue visiting child nodes
        ruby_prism::visit_if_node(self, node);
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        self.register_children_of_unless(node);
        self.check_unless_node(node);
        // Continue visiting child nodes
        ruby_prism::visit_unless_node(self, node);
    }
}

impl IfWithSemicolonVisitor<'_> {
    /// Register sole-child if/unless nodes in ALL branches of an if node.
    /// This mirrors RuboCop's `node.parent&.if_type?` check: in the parser gem,
    /// when a branch has only one statement, that statement's parent is the if node
    /// itself (if_type? → true). With multiple statements, they're wrapped in a
    /// `begin` node (if_type? → false).
    fn register_children_of_if(&mut self, if_node: &ruby_prism::IfNode<'_>) {
        // Register sole child in if-branch
        self.register_sole_if_unless_child(if_node.statements());

        // Walk elsif chain and else
        let mut subsequent = if_node.subsequent();
        while let Some(sub) = subsequent {
            if let Some(elsif_node) = sub.as_if_node() {
                // Register sole child in elsif-branch
                self.register_sole_if_unless_child(elsif_node.statements());
                subsequent = elsif_node.subsequent();
            } else if let Some(else_node) = sub.as_else_node() {
                // Register sole child in else-branch
                self.register_sole_if_unless_child(else_node.statements());
                break;
            } else {
                break;
            }
        }
    }

    /// Same as above but for unless nodes.
    fn register_children_of_unless(&mut self, unless_node: &ruby_prism::UnlessNode<'_>) {
        // Register sole child in unless-branch
        self.register_sole_if_unless_child(unless_node.statements());

        // Register sole child in else-branch
        if let Some(else_node) = unless_node.else_clause() {
            self.register_sole_if_unless_child(else_node.statements());
        }
    }

    /// If the statements node contains exactly one if or unless node, register
    /// its start offset so it gets skipped.
    fn register_sole_if_unless_child(&mut self, stmts: Option<ruby_prism::StatementsNode<'_>>) {
        if let Some(stmts) = stmts {
            let body = stmts.body();
            if body.len() == 1 {
                let child = body.iter().next().unwrap();
                if child.as_if_node().is_some() || child.as_unless_node().is_some() {
                    self.parent_is_if_offsets
                        .push(child.location().start_offset());
                }
            }
        }
    }

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

        // Skip if this node is the sole statement in another if/unless branch
        // (RuboCop: node.parent&.if_type?). Covers else-if, sole child in if-branch, etc.
        let loc = if_node.location();
        if self.parent_is_if_offsets.contains(&loc.start_offset()) {
            return;
        }

        // Skip if inside a previously flagged node (RuboCop: part_of_ignored_node?)
        if loc.start_offset() < self.ignored_end_offset {
            return;
        }

        // Check for semicolon: Prism's then_keyword_loc is ";" or "then".
        // Fallback: scan between predicate and body on the same line.
        let has_semicolon = if let Some(then_loc) = if_node.then_keyword_loc() {
            then_loc.as_slice() == b";"
        } else {
            let stmts = if_node.statements();
            let pred_end = if_node.predicate().location().end_offset();
            let body_start = if let Some(s) = stmts.as_ref() {
                s.location().start_offset()
            } else if let Some(sub) = if_node.subsequent() {
                sub.location().start_offset()
            } else if let Some(end_loc) = if_node.end_keyword_loc() {
                end_loc.start_offset()
            } else {
                return;
            };
            if stmts.is_none() {
                // Empty body: a standalone `;` on the next line is treated as
                // the then-keyword by Prism's Translation Parser (used by
                // RuboCop with TargetRubyVersion >= 3.4). Scan across newlines.
                has_semicolon_multiline(self.source, pred_end, body_start)
            } else {
                has_semicolon_between(self.source, pred_end, body_start)
            }
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

        // Skip if this node is the sole statement in another if/unless branch
        // (RuboCop: node.parent&.if_type?)
        let loc = unless_node.location();
        if self.parent_is_if_offsets.contains(&loc.start_offset()) {
            return;
        }

        // Skip if inside a previously flagged node (RuboCop: part_of_ignored_node?)
        if loc.start_offset() < self.ignored_end_offset {
            return;
        }

        // Check for semicolon
        let has_semicolon = if let Some(then_loc) = unless_node.then_keyword_loc() {
            then_loc.as_slice() == b";"
        } else {
            let stmts = unless_node.statements();
            let pred_end = unless_node.predicate().location().end_offset();
            let body_start = if let Some(s) = stmts.as_ref() {
                s.location().start_offset()
            } else if let Some(else_clause) = unless_node.else_clause() {
                else_clause.location().start_offset()
            } else if let Some(end_loc) = unless_node.end_keyword_loc() {
                end_loc.start_offset()
            } else {
                return;
            };
            if stmts.is_none() {
                has_semicolon_multiline(self.source, pred_end, body_start)
            } else {
                has_semicolon_between(self.source, pred_end, body_start)
            }
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

/// Scan for a semicolon across multiple lines, skipping Ruby comments.
/// Used when `statements` is nil (empty body) and the `;` may be on a
/// subsequent line — Prism's Translation Parser treats it as `loc.begin`.
fn has_semicolon_multiline(source: &SourceFile, pred_end: usize, body_start: usize) -> bool {
    if pred_end < body_start {
        let between = &source.content[pred_end..body_start];
        let mut in_comment = false;
        for &b in between {
            if b == b'\n' {
                in_comment = false;
            } else if b == b'#' {
                in_comment = true;
            } else if !in_comment && b == b';' {
                return true;
            }
        }
    }
    false
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
        let source = b"if a; foo\nelsif b;\n  if c; bar\n  elsif d; baz\n  end\nend\n";
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

    #[test]
    fn case_else_if_with_semicolon_flagged() {
        // `else if` inside a case statement should be flagged — the `else` belongs to
        // the case, not to an if, so `node.parent&.if_type?` is false.
        let source =
            b"case tt\nwhen :slash then slt = tt\nelse if at; zt = tt; else; at = tt; end\nend\n";
        let diags = crate::testutil::run_cop_full(&IfWithSemicolon, source);
        assert_eq!(diags.len(), 1, "Should flag 'if at;' inside case else");
    }

    #[test]
    fn else_if_inside_if_still_suppressed() {
        // `else if` inside an if statement should still be suppressed
        let source = b"if x > 0\n  foo\nelse if y > 0; bar else baz end\nend\n";
        let diags = crate::testutil::run_cop_full(&IfWithSemicolon, source);
        assert_eq!(
            diags.len(),
            0,
            "else if inside if's else should be suppressed"
        );
    }

    #[test]
    fn citysdk_semicolon_body_on_next_line_flagged() {
        // A bare `;` on the next line with empty body — Prism Translation Parser
        // treats it as the then-keyword (`loc.begin`), so RuboCop flags it.
        let source =
            b"if params[:layer] == '*' and query[:resource] == :objects\n  ;\nelse\n  foo\nend\n";
        let diags = crate::testutil::run_cop_full(&IfWithSemicolon, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag bare semicolon on next line with empty body"
        );
    }

    #[test]
    fn multiline_if_semicolon_with_else_flagged() {
        // Multiline if with semicolon on first line, else on next line
        let source = b"if @flip.nil?; @flip = :horiz\nelse; @flip = nil; end\n";
        let diags = crate::testutil::run_cop_full(&IfWithSemicolon, source);
        assert_eq!(
            diags.len(),
            1,
            "Multiline if with semicolon should be flagged"
        );
    }
}
