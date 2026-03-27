use std::collections::HashSet;

use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Style/Semicolon — flags unnecessary semicolons used as expression separators
/// or statement terminators.
///
/// Investigation findings (2026-03-19):
///
/// Root causes of false positives (FP=34):
/// - `is_trailing_semicolon` treated `; # comment` as trailing (only comment follows).
///   RuboCop's token-based approach sees the comment token as the last token, masking
///   the semicolon. Fix: do NOT treat `; # comment` as trailing — only flag when
///   literally nothing follows except whitespace/newline.
/// - Previous fix: `$;` (Ruby's `$FIELD_SEPARATOR` global variable) was misidentified
///   as a statement-terminating semicolon. Fixed by checking if the preceding byte is `$`.
///
/// Root causes of false negatives (FN=51):
/// - Semicolons before `}` in blocks (`foo { bar; }`) were not detected. RuboCop's
///   token-based approach checks `tokens[-2].right_curly_brace? && tokens[-3].semicolon?`
///   which catches these because `tNL` (newline) is the last token. Fix: added
///   `is_semicolon_before_closing_brace` check.
/// - Semicolons in string interpolation (`"#{foo;}"`) were not detected. The semicolon
///   inside `#{}` is code but falls through all checks. Fix: covered by the
///   `is_semicolon_before_closing_brace` check since the interpolation `}` is also code.
/// - On expression separator lines, RuboCop scans raw source text for `;` characters
///   (including inside strings on the same line). nitrocop was filtering by `is_code()`,
///   missing semicolons inside strings on expression separator lines. Fix: on expr_sep
///   lines, scan raw source for all `;` characters, matching RuboCop's `find_semicolon_positions`.
/// - Semicolons after `#{` in string interpolation (`"#{;foo}"`) were not detected.
///   Fix: added `is_semicolon_after_interpolation_open` check.
/// - Semicolons after opening `{` in blocks (`foo {; bar }`) are only caught by RuboCop
///   when `{` is at specific token positions (position 1 for regular blocks, position 2
///   for lambda blocks). This is inherently positional and hard to replicate with byte
///   scanning without introducing FPs (e.g., `items.each {; bar }` is NOT flagged by
///   RuboCop). These cases remain as known FN gaps.
///
/// Investigation findings (2026-03-20):
///
/// Root causes of remaining false positives (FP=12, 6 are begin...end):
/// - Semicolons inside explicit `begin...end` blocks were flagged as expression
///   separators. In Parser AST, explicit `begin...end` creates `kwbegin` (not `begin`),
///   so RuboCop's `on_begin` callback does NOT fire for them. But in Prism, both
///   explicit `begin...end` (BeginNode) and implicit multi-statement wrappers use
///   StatementsNode, so the ExprSeparatorVisitor was incorrectly visiting them.
///   Fix: override `visit_begin_node` to set `inside_explicit_begin` flag, and skip
///   expression separator detection for StatementsNodes inside BeginNode.
/// - 4 FPs are from `.rb.spec` files in the rufo repo — file discovery issues, not cop bugs.
/// - 2 FPs are from `begin; 1; 2; end` style patterns in rufo (also begin...end).
///
/// Investigation findings (2026-03-27):
///
/// Root causes of remaining false negatives (FN=2):
/// - Implicit `BeginNode` wrappers for `def ... rescue ... end` bodies were treated the
///   same as explicit `begin...end`. Prism uses `BeginNode` for both, but only explicit
///   begin blocks have `begin_keyword_loc()`. The previous visitor skipped ALL BeginNode
///   bodies, so expression-separator semicolons on rescue lines were missed. Fix: only
///   suppress StatementsNode scanning for explicit begin blocks.
/// - Semicolons before `}` with a trailing comma at end-of-line were missed, e.g.
///   `parameters: foo { bar; },`. RuboCop still flags this because the brace/comma pair
///   is the end of the line. Fix: allow `; } ,` when the comma is the last non-whitespace
///   token on the line, while still ignoring cases like `foo { bar; }, baz`.
pub struct Semicolon;

impl Cop for Semicolon {
    fn name(&self) -> &'static str {
        "Style/Semicolon"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let bytes = source.as_bytes();
        if !bytes.contains(&b';') {
            return;
        }

        let allow_separator = config.get_bool("AllowAsExpressionSeparator", false);

        // Phase 1: Walk the AST to find lines where a StatementsNode has 2+ children
        // sharing the same last_line. These lines have expression separator semicolons.
        // RuboCop's on_begin fires for these and flags ALL semicolons on such lines.
        let expr_sep_lines = if !allow_separator {
            let mut visitor = ExprSeparatorVisitor {
                source,
                lines: HashSet::new(),
                inside_explicit_begin: false,
            };
            visitor.visit(&parse_result.node());
            visitor.lines
        } else {
            HashSet::new()
        };

        // Phase 2: For expression separator lines, scan raw source for ALL semicolons
        // (including inside strings on the line), matching RuboCop's find_semicolon_positions.
        // Track which offsets have already been reported to avoid duplicates.
        let mut reported: HashSet<usize> = HashSet::new();

        for &line in &expr_sep_lines {
            let line_start = source.line_start_offset(line);
            // Find end of line (next newline or end of file)
            let line_end = bytes[line_start..]
                .iter()
                .position(|&b| b == b'\n')
                .map_or(bytes.len(), |p| line_start + p);
            let line_bytes = &bytes[line_start..line_end];
            for (j, &ch) in line_bytes.iter().enumerate() {
                if ch == b';' {
                    let offset = line_start + j;
                    // Skip $; — Ruby's $FIELD_SEPARATOR global variable
                    if offset > 0 && bytes[offset - 1] == b'$' {
                        continue;
                    }
                    let (l, column) = source.offset_to_line_col(offset);
                    diagnostics.push(self.diagnostic(
                        source,
                        l,
                        column,
                        "Do not use semicolons to terminate expressions.".to_string(),
                    ));
                    reported.insert(offset);
                }
            }
        }

        // Phase 3: Scan for code semicolons and classify each.
        for (i, &byte) in bytes.iter().enumerate() {
            if byte != b';' || !code_map.is_code(i) {
                continue;
            }

            // Skip already-reported expression separator semicolons
            if reported.contains(&i) {
                continue;
            }

            // Skip $; — Ruby's $FIELD_SEPARATOR global variable, not a semicolon
            if i > 0 && bytes[i - 1] == b'$' {
                continue;
            }

            let (line, column) = source.offset_to_line_col(i);

            // Check if trailing: no non-whitespace content after the semicolon on this line.
            // Note: comments after the semicolon do NOT make it trailing — RuboCop's token-based
            // approach sees the comment token as the last token, masking the semicolon.
            if is_trailing_semicolon(bytes, i) {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not use semicolons to terminate expressions.".to_string(),
                ));
                continue;
            }

            // Check if leading: nothing meaningful before the semicolon on this line.
            if is_leading_semicolon(bytes, i) {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not use semicolons to terminate expressions.".to_string(),
                ));
                continue;
            }

            // Check if semicolon is directly before a closing brace `}` on the same line
            // (only whitespace between `;` and `}`). Catches:
            // - Block trailing semicolons: `foo { bar; }`
            // - String interpolation: `"#{foo;}"`
            // RuboCop catches these via token position checks (tokens[-2] is `}`, tokens[-3] is `;`).
            if is_semicolon_before_closing_brace(bytes, i, code_map) {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not use semicolons to terminate expressions.".to_string(),
                ));
                continue;
            }

            // Check if semicolon is directly after `#{` in string interpolation
            // (only whitespace between `{` and `;`). Catches `"#{;foo}"`.
            if is_semicolon_after_interpolation_open(bytes, i) {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not use semicolons to terminate expressions.".to_string(),
                ));
                continue;
            }
        }
    }
}

/// Check if a semicolon at byte position `pos` is trailing:
/// nothing non-whitespace follows it on the same line.
///
/// Note: a comment after the semicolon (`; # comment`) does NOT make it trailing.
/// RuboCop's token-based approach sees the comment token as the last token on the
/// line, so the semicolon is not detected as the "last token".
fn is_trailing_semicolon(bytes: &[u8], pos: usize) -> bool {
    for &ch in &bytes[pos + 1..] {
        if ch == b'\n' || ch == b'\r' {
            return true;
        }
        if ch == b' ' || ch == b'\t' {
            continue;
        }
        // Any non-whitespace character means it's not trailing.
        // This includes `#` (comments) — RuboCop does not flag `; # comment` as trailing.
        return false;
    }
    // Reached end of file without newline
    true
}

/// Check if a semicolon at byte position `pos` is leading:
/// nothing meaningful before it on this line (only whitespace).
fn is_leading_semicolon(bytes: &[u8], pos: usize) -> bool {
    if pos == 0 {
        return true;
    }
    for &ch in bytes[..pos].iter().rev() {
        if ch == b'\n' || ch == b'\r' {
            return true;
        }
        if ch == b' ' || ch == b'\t' {
            continue;
        }
        return false;
    }
    // Reached start of file
    true
}

/// Check if a semicolon at byte position `pos` is directly before a closing
/// brace `}` on the same line (only whitespace between `;` and `}`).
/// The `}` must also be in code (not inside a string/comment).
///
/// To match RuboCop's token-based behavior, we require one of:
/// 1. Block pattern: `}` is the last thing on the line (only whitespace/newline follows).
///    This matches `tokens[-2] == }` with `tokens[-1] == NL`.
/// 2. String interpolation pattern: `}` is immediately followed by the closing string
///    delimiter (a non-code byte like `"`). This matches `tokens[-3] == DEND`.
///
/// When a comment follows `}` on the same line (`foo { bar; } # comment`), or code
/// follows (`foo { bar; }.baz`), RuboCop's token positions shift and the check fails.
///
/// This catches patterns like:
/// - `foo { bar; }` — block trailing semicolon
/// - `"#{foo;}"` — string interpolation trailing semicolon
fn is_semicolon_before_closing_brace(bytes: &[u8], pos: usize, code_map: &CodeMap) -> bool {
    // Find the `}` after the semicolon (skipping whitespace)
    let mut brace_offset = None;
    for (j, &ch) in bytes[pos + 1..].iter().enumerate() {
        if ch == b'\n' || ch == b'\r' {
            return false;
        }
        if ch == b' ' || ch == b'\t' {
            continue;
        }
        if ch == b'}' {
            brace_offset = Some(pos + 1 + j);
            break;
        }
        return false;
    }

    let brace_off = match brace_offset {
        Some(off) => off,
        None => return false,
    };

    if !code_map.is_code(brace_off) {
        return false;
    }

    // Check what follows the `}` on the same line.
    if brace_off + 1 >= bytes.len() {
        // `}` is the last byte in the file
        return true;
    }

    let next_byte = bytes[brace_off + 1];

    // Pattern 1: `}` is at end of line (only whitespace follows)
    if next_byte == b'\n' || next_byte == b'\r' {
        return true;
    }

    // Pattern 2: String interpolation close — `}` is immediately followed by
    // the string's closing delimiter (`"`, `'`, `` ` ``, etc.).
    // RuboCop checks tokens[-3] == tSTRING_DEND && tokens[-4] == tSEMI, which
    // only matches when nothing follows `}` before the string close.
    if (next_byte == b'"' || next_byte == b'\'' || next_byte == b'`')
        && !code_map.is_code(brace_off + 1)
    {
        return true;
    }

    // Pattern 3: `}` is followed by a trailing comma at end of line.
    // RuboCop still flags `; } ,` when the comma is the last non-whitespace token
    // on the line (e.g. a hash pair in a multiline literal), but not when more
    // code follows after the comma (`x = [foo { bar; }, baz]`).
    let mut saw_trailing_comma = false;
    for &ch in &bytes[brace_off + 1..] {
        if ch == b'\n' || ch == b'\r' {
            return true;
        }
        if ch == b' ' || ch == b'\t' {
            continue;
        }
        if ch == b',' && !saw_trailing_comma {
            saw_trailing_comma = true;
            continue;
        }
        // Non-whitespace code follows `}` (or follows the trailing comma) —
        // positions shift in RuboCop, so this is not an offense.
        return false;
    }
    // End of file after whitespace or a trailing comma
    true
}

/// Check if a semicolon at byte position `pos` is directly after `#{` in
/// string interpolation (only whitespace between `{` and `;`).
///
/// This catches patterns like `"#{;foo}"`.
/// Only matches when the `{` is preceded by `#` (string interpolation opener).
fn is_semicolon_after_interpolation_open(bytes: &[u8], pos: usize) -> bool {
    if pos < 2 {
        return false;
    }
    for (j, &ch) in bytes[..pos].iter().rev().enumerate() {
        if ch == b'\n' || ch == b'\r' {
            return false;
        }
        if ch == b' ' || ch == b'\t' {
            continue;
        }
        if ch == b'{' {
            let brace_offset = pos - 1 - j;
            // Check that `#` precedes the `{` to confirm it's `#{`
            return brace_offset > 0 && bytes[brace_offset - 1] == b'#';
        }
        return false;
    }
    false
}

/// AST visitor that collects line numbers where a StatementsNode has 2+ children
/// sharing the same last_line (expression separator lines).
///
/// Skips StatementsNode inside explicit `begin...end` (BeginNode in Prism).
/// In Parser AST, explicit `begin...end` creates `kwbegin`, not `begin`.
/// RuboCop's `on_begin` only fires for implicit `begin` (multi-statement wrappers),
/// so semicolons inside explicit `begin...end` are NOT expression separators.
struct ExprSeparatorVisitor<'a> {
    source: &'a SourceFile,
    lines: HashSet<usize>,
    inside_explicit_begin: bool,
}

impl<'pr> Visit<'pr> for ExprSeparatorVisitor<'_> {
    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        if node.begin_keyword_loc().is_some() {
            // Explicit begin...end maps to Parser's kwbegin. RuboCop's on_begin
            // does not fire for it, so skip expression-separator detection for
            // the direct StatementsNode body only in this case.
            let prev = self.inside_explicit_begin;
            self.inside_explicit_begin = true;
            ruby_prism::visit_begin_node(self, node);
            self.inside_explicit_begin = prev;
        } else {
            // Implicit BeginNode wrappers come from rescue/ensure bodies in defs,
            // blocks, and lambdas. RuboCop still treats their statements like
            // regular begin bodies for Style/Semicolon.
            ruby_prism::visit_begin_node(self, node);
        }
    }

    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        if !self.inside_explicit_begin {
            let body: Vec<ruby_prism::Node<'pr>> = node.body().iter().collect();
            if body.len() >= 2 {
                // Group expressions by their last line (matching RuboCop's expressions_per_line)
                let mut line_counts: Vec<(usize, usize)> = Vec::new();
                for expr in &body {
                    let end_offset = expr.location().end_offset();
                    // Use end_offset - 1 to get the line of the last byte of the expression
                    let (last_line, _) =
                        self.source.offset_to_line_col(end_offset.saturating_sub(1));
                    if let Some(entry) = line_counts.last_mut() {
                        if entry.0 == last_line {
                            entry.1 += 1;
                            continue;
                        }
                    }
                    line_counts.push((last_line, 1));
                }

                for &(line, count) in &line_counts {
                    if count >= 2 {
                        self.lines.insert(line);
                    }
                }
            }
        }

        // Continue visiting children (reset flag so nested non-begin statements work)
        let prev = self.inside_explicit_begin;
        self.inside_explicit_begin = false;
        ruby_prism::visit_statements_node(self, node);
        self.inside_explicit_begin = prev;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Semicolon, "cops/style/semicolon");
}
