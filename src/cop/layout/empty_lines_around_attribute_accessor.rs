use crate::cop::node_type::CALL_NODE;
use crate::cop::util::is_blank_or_whitespace_line;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Layout/EmptyLinesAroundAttributeAccessor
///
/// ## Investigation (2026-03-11)
///
/// **Root cause of 471 FPs:** RuboCop's `next_line_node` method returns nil when
/// `node.parent.if_type?` is true (line 118 of vendor source), meaning attr accessors
/// inside conditional branches (if/unless/case/when/elsif/else) never fire. RuboCop
/// also returns nil when there is no `right_sibling` (last statement before rescue/ensure/end).
///
/// Nitrocop's line-based approach had no equivalent check. It only skipped `end` on the
/// next line, but not `else`, `elsif`, `when`, `in`, `rescue`, or `ensure` — all of which
/// indicate the attr accessor is the last statement in a conditional or error-handling branch.
///
/// **Fix:** Added `is_block_boundary_keyword()` that checks the next trimmed line for any
/// of these branch-closing keywords (end, else, elsif, when, in, rescue, ensure). This
/// mirrors RuboCop's AST-level "no right sibling" check using line-level heuristics.
///
/// **Fix 2 (2026-03-13):** Changed `is_blank_line` to `is_blank_or_whitespace_line` so that
/// whitespace-only lines (e.g., `    \n` — indentation-only) are recognized as valid blank
/// separators. This was the root cause of 468 FPs: Ruby codebases commonly have indentation
/// whitespace on otherwise-blank lines between attr accessors.
///
/// **Fix 3 (2026-03-14):** Added standalone-statement check: the attr call must start at the
/// first non-whitespace position of its line. This filters out attr calls used as expressions
/// (`(attr :foo).should`), inside single-line block braces (`{ attr_reader :name }`), or
/// nested inside method calls (`mod.module_eval { attr(name) }`). RuboCop handles this
/// naturally via `node.right_sibling` which only exists for direct children of a statements
/// body. Eliminated 100 FPs across 38 repos.
///
/// **Fix 4 (2026-03-14):** Three additional FP fixes:
/// (a) Block boundary detection now handles `end.method` and `}.method` patterns,
///     where the attr is the last statement inside `Class.new { }` or `do...end` blocks
///     followed by `.new` or other method chains. Previously only `end` followed by
///     space/newline/nothing was recognized. (~11 FPs across decidim, factory_bot, etc.)
/// (b) Comment lookahead now checks for blank lines, allowed methods, and alias after
///     comments, not just attr methods. This fixes cases like `attr_reader :name` followed
///     by a comment then `public()`, or a comment then blank line. (~5 FPs)
/// (c) Added right-side expression check: if there is non-whitespace, non-comment content
///     after the call's end_offset on the same line (e.g., `attr_accessor :x unless cond`),
///     the call is part of a larger expression and should be skipped. (~2 FPs: travis, spreadsheet)
///
/// **Remaining gap (pre-fix-5):** 3 FNs — 2 from camping (minified Ruby, mid-line attr calls) and 1 from
/// CocoaPods (`attr_accessor name` followed by `alias_method ... if boolean` where the `if`
/// modifier makes RuboCop treat it as non-allowed, but nitrocop's line-based check sees
/// `alias_method` and allows it). Fixed in fix 6 below (except camping).
///
/// **Fix 5 (2026-03-15):** 18 FNs across 12 repos caused by comment lookahead incorrectly
/// suppressing offense when a blank line appeared after comments. The pattern:
///   `attr_accessor :foo` / `# YARD comment` / blank line / `def bar`
/// RuboCop uses AST `right_sibling` which completely ignores comments — the right sibling
/// of the attr is `def bar`, which is not an allowed successor, so it flags. Nitrocop's
/// comment lookahead was returning "no offense" upon hitting a blank line after comments.
/// Fix: changed blank-line handling in comment lookahead from `return` (suppress) to
/// `continue` (skip), so the loop scans past comments AND blank lines to find the actual
/// code line and checks whether it's an allowed successor (attr, alias, allowed method,
/// block boundary). Added EOF guard: if no code line is found after comments, no offense
/// (matches RuboCop's nil right_sibling for last-in-body attrs).
///
/// **Fix 6 (2026-03-16):** 3 of 5 remaining FNs:
/// (a) Trailing semicolons after attr calls (`attr_accessor :foo;`) were treated as
///     trailing code, causing the cop to skip the call. Added `;` to the allowed
///     trailing characters alongside whitespace, newline, and `#`. (2 FNs: jruby, rack)
/// (b) `alias_method` with `if`/`unless` modifier (`alias_method :x, :y if cond`) was
///     incorrectly treated as an allowed successor method. RuboCop's AST sees this as
///     an IfNode wrapping the send, not a plain SendNode. Added `has_modifier_conditional()`
///     check to detect ` if ` / ` unless ` in the line and skip the allowed-method
///     exemption. (1 FN: CocoaPods)
/// (c) Camping FNs (2) are unfixable — minified Ruby with mid-line attr calls separated
///     by semicolons. Prism parses these as CallNodes but they fail the standalone-statement
///     check (not at first non-whitespace position of the line).
///
/// **Fix 7 (2026-03-25):** Three fixes:
/// (a) `end)`, `end]`, `end,` not recognized as block boundaries — only `end` followed
///     by space/newline/comment/semicolon/dot was handled. Added `)`, `]`, `,` to allowed
///     chars after `end`. Fixes 5 FPs (httpx, yohasebe) where `attr_reader` inside
///     `Module.new do...end)` was flagged.
/// (b) `# rubocop:enable` directive comment followed by blank line now treated as valid
///     separator, matching RuboCop's `next_line_empty_or_enable_directive_comment?`.
///     Fixes 2 FPs (expertiza) where `attr_accessor` between `rubocop:disable/enable`
///     comments was flagged.
/// (c) `is_attr_method_line()` now accepts `:` after method name (e.g., `attr_accessor:name`
///     without space) and rejects attr lines with modifier `if`/`unless` (e.g.,
///     `attr_writer name if writer`). Fixes 1 FP (pangloss: `attr_accessor:to_emit`) and
///     1 FN (rcodetools: conditional attr_writer not treated as sibling).
///
/// **Fix 8 (2026-03-30):** Exact-location verification showed the 18 recorded FPs were
/// already fixed; the remaining oracle mismatches were 5 FNs. Root causes:
/// (a) the standalone-statement heuristic only allowed attr calls at the first non-whitespace
///     column, so minified class/module bodies (`class C;attr_accessor :x`) and case predicates
///     (`case attr 'x'`) were skipped.
/// (b) Prism's `CallNode` location for block-form accessors spans the whole block, while
///     RuboCop reasons about the send line before the block body. That made
///     `attr_accessor :x do ... end` and `attr_accessor(:x) { ... }` look like EOF/`end`
///     followers instead of offenses.
/// Fix: accept semicolon-separated and `case`-predicate statement prefixes, use the block
/// opening line as the separation point for block-form accessors, and avoid treating
/// `when`/`end` as allowed structural followers for those specific contexts.
///
/// **Fix 9 (2026-03-30):** Final FP/FN reconciliation for the remaining oracle cases:
/// (a) Prism stores both real block bodies and `&block` arguments in `call.block()`.
///     The cop treated any non-nil block slot as block-form accessor syntax, so DSL wrappers
///     like `attr(a, header, &block)` were always flagged. Fix: only treat `BlockNode`
///     entries as block-form accessors.
/// (b) `attr(...)` used as an array/parenthesized expression can still start at the first
///     non-whitespace column of its line, so the line-prefix heuristic alone was too weak.
///     Fix: treat `]`/`)` followers as structural closers for nested expression contexts
///     where RuboCop has no statement sibling to inspect.
/// (c) `has_modifier_conditional()` naively split at `#`, which truncated
///     `alias_method "#{name}?", name if boolean` at string interpolation and hid the
///     trailing `if` modifier. Fix: scan for `if`/`unless` outside strings and comments.
pub struct EmptyLinesAroundAttributeAccessor;

const ATTRIBUTE_METHODS: &[&[u8]] = &[b"attr_reader", b"attr_writer", b"attr_accessor", b"attr"];

const DEFAULT_ALLOWED_METHODS: &[&str] = &["alias_method", "public", "protected", "private"];

impl Cop for EmptyLinesAroundAttributeAccessor {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundAttributeAccessor"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let _allow_alias_syntax = config.get_bool("AllowAliasSyntax", true);
        let _allowed_methods = config.get_string_array("AllowedMethods");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be a bare call (no receiver)
        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name().as_slice();
        if !ATTRIBUTE_METHODS.contains(&method_name) {
            return;
        }

        // Must have arguments (e.g., `attr_reader :foo`)
        if call.arguments().is_none() {
            return;
        }

        let loc = call.location();

        // The attr call must be a statement-like expression on its line. RuboCop
        // allows same-line statements after `;` and in `case attr ...` predicates,
        // but should still skip nested expression uses like `(attr :foo).should`
        // or `{ attr_reader(:name) }`.
        let (start_line, start_col) = source.offset_to_line_col(loc.start_offset());
        let lines: Vec<&[u8]> = source.lines().collect();
        let mut is_case_predicate = false;
        if start_line > 0 && (start_line - 1) < lines.len() {
            let call_line = lines[start_line - 1];
            let prefix_end = start_col.min(call_line.len());
            let statement_prefix = trim_ascii_space(&call_line[..prefix_end]);
            is_case_predicate = statement_prefix == b"case";
            if !is_statement_prefix(statement_prefix) {
                return;
            }
        }
        let (call_end_line, last_col) =
            source.offset_to_line_col(loc.end_offset().saturating_sub(1));
        let block_node = call.block().and_then(|node| node.as_block_node());
        let has_attached_block = block_node.is_some();
        let separator_line = block_node
            .map(|block| {
                source
                    .offset_to_line_col(block.opening_loc().start_offset())
                    .0
            })
            .unwrap_or(call_end_line);

        // If there is non-whitespace, non-comment content after the call's end on the
        // same line, the attr call is part of a larger expression (e.g.,
        // `attr_accessor :parser unless method_defined? :parser`). Skip it.
        // RuboCop handles this via AST: the call is inside an UnlessModifierNode and
        // has no right_sibling in a statements body.
        if call_end_line > 0 && (call_end_line - 1) < lines.len() {
            let call_end_source_line = lines[call_end_line - 1];
            let after_call =
                &call_end_source_line[(last_col + 1).min(call_end_source_line.len())..];
            if let Some(next_same_line) = same_line_following_statement(after_call) {
                if is_block_boundary_keyword(next_same_line) || is_attr_method_line(next_same_line)
                {
                    return;
                }
                if _allow_alias_syntax && next_same_line.starts_with(b"alias ") {
                    return;
                }
                for allowed_method in
                    config
                        .get_string_array("AllowedMethods")
                        .unwrap_or_else(|| {
                            DEFAULT_ALLOWED_METHODS
                                .iter()
                                .map(|s| s.to_string())
                                .collect()
                        })
                {
                    let method_bytes = allowed_method.as_bytes();
                    if next_same_line.starts_with(method_bytes) {
                        let after = next_same_line.get(method_bytes.len());
                        if (after.is_none()
                            || matches!(after, Some(b' ') | Some(b'(') | Some(b'\n') | Some(b'\r')))
                            && !has_modifier_conditional(next_same_line)
                        {
                            return;
                        }
                    }
                }
            }
            let has_trailing_code = after_call
                .iter()
                .find(|&&b| b != b' ' && b != b'\t')
                .is_some_and(|&b| b != b'\n' && b != b'\r' && b != b'#' && b != b';');
            if has_trailing_code {
                return;
            }
        }

        // Check if the next line exists and is not empty
        if separator_line >= lines.len() {
            return; // End of file
        }

        let next_line = lines[separator_line]; // 0-indexed: line N (1-based) maps to lines[N] for next

        // If next line is blank, no offense
        if is_blank_or_whitespace_line(next_line) {
            return;
        }

        // RuboCop treats `# rubocop:enable ...` directive comments followed by a
        // blank line as valid separators (next_line_empty_or_enable_directive_comment?).
        if is_enable_directive_comment(next_line) {
            let line_after = separator_line + 1;
            if line_after >= lines.len() || is_blank_or_whitespace_line(lines[line_after]) {
                return;
            }
        }

        // RuboCop reasons about block-form accessors from the send line before the block
        // body; any non-blank next line after that line is an offense.
        if has_attached_block {
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            let mut diag = self.diagnostic(
                source,
                line,
                col,
                "Add an empty line after attribute accessor.".to_string(),
            );
            if let Some(ref mut corr) = corrections {
                if let Some(offset) = source.line_col_to_offset(separator_line + 1, 0) {
                    corr.push(crate::correction::Correction {
                        start: offset,
                        end: offset,
                        replacement: "\n".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diag.corrected = true;
                }
            }
            diagnostics.push(diag);
            return;
        }

        // If next line is end of class/module/block, or a branch keyword
        // (else/elsif/when/in/rescue/ensure), no offense.
        // RuboCop skips when node.parent.if_type? (no right_sibling to check).
        // We approximate this by detecting branch-closing keywords on the next line.
        let next_trimmed: Vec<u8> = next_line
            .iter()
            .copied()
            .skip_while(|&b| b == b' ' || b == b'\t')
            .collect();
        if !is_case_predicate
            && (is_block_boundary_keyword(&next_trimmed) || is_expression_closer(&next_trimmed))
        {
            return;
        }

        // If next line is another attribute accessor, no offense
        if is_attr_method_line(&next_trimmed) {
            return;
        }

        // If next line is a comment, look past comments (and blank lines) to find
        // the next code line. If it's another attribute accessor, an allowed method,
        // alias, or a block boundary, no offense. Otherwise offense — RuboCop uses
        // AST right_sibling which skips comments entirely; a blank line after comments
        // does NOT suppress the offense.
        // This allows YARD-style documented accessors:
        //   attr_reader :value
        //   # @return [Exception, nil]
        //   attr_reader :handled_error
        if next_trimmed.starts_with(b"#") {
            let allowed = config.get_string_array("AllowedMethods");
            let allowed_for_comment: Vec<String> = allowed.unwrap_or_else(|| {
                DEFAULT_ALLOWED_METHODS
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            });
            let mut idx = separator_line + 1;
            let mut found_code = false;
            while idx < lines.len() {
                let line_trimmed: Vec<u8> = lines[idx]
                    .iter()
                    .copied()
                    .skip_while(|&b| b == b' ' || b == b'\t')
                    .collect();
                if is_blank_or_whitespace_line(lines[idx]) {
                    idx += 1;
                    continue; // skip blank lines — they don't suppress offense
                }
                if line_trimmed.starts_with(b"#") {
                    idx += 1;
                    continue; // skip comments
                }
                // Found a code line — check if it's an allowed successor
                found_code = true;
                if is_attr_method_line(&line_trimmed) {
                    return;
                }
                if !is_case_predicate
                    && (is_block_boundary_keyword(&line_trimmed)
                        || is_expression_closer(&line_trimmed))
                {
                    return;
                }
                if _allow_alias_syntax && line_trimmed.starts_with(b"alias ") {
                    return;
                }
                for am in &allowed_for_comment {
                    let mb = am.as_bytes();
                    if line_trimmed.starts_with(mb) {
                        let after = line_trimmed.get(mb.len());
                        if (after.is_none()
                            || matches!(after, Some(b' ') | Some(b'(') | Some(b'\n') | Some(b'\r')))
                            && !has_modifier_conditional(&line_trimmed)
                        {
                            return;
                        }
                    }
                }
                break;
            }
            // If no code line was found after comments (EOF or only blank lines),
            // no right sibling exists — no offense (matches RuboCop's nil right_sibling).
            if !found_code {
                return;
            }
        }

        // Check if next line is an allowed method
        let allowed = config.get_string_array("AllowedMethods");
        let allowed_methods: Vec<String> = allowed.unwrap_or_else(|| {
            DEFAULT_ALLOWED_METHODS
                .iter()
                .map(|s| s.to_string())
                .collect()
        });

        for allowed_method in &allowed_methods {
            let method_bytes = allowed_method.as_bytes();
            if next_trimmed.starts_with(method_bytes) {
                let after = next_trimmed.get(method_bytes.len());
                // If the allowed method has a modifier `if`/`unless`, RuboCop's AST
                // sees an IfNode wrapping the send, not a plain allowed method call.
                if (after.is_none()
                    || matches!(after, Some(b' ') | Some(b'(') | Some(b'\n') | Some(b'\r')))
                    && !has_modifier_conditional(&next_trimmed)
                {
                    return;
                }
            }
        }

        // Check if next line is an alias
        if _allow_alias_syntax && next_trimmed.starts_with(b"alias ") {
            return;
        }

        let (line, col) = source.offset_to_line_col(loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            "Add an empty line after attribute accessor.".to_string(),
        );
        if let Some(ref mut corr) = corrections {
            // Insert blank line after the attribute accessor line
            if let Some(offset) = source.line_col_to_offset(separator_line + 1, 0) {
                corr.push(crate::correction::Correction {
                    start: offset,
                    end: offset,
                    replacement: "\n".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
        }
        diagnostics.push(diag);
    }
}

fn trim_ascii_space(line: &[u8]) -> &[u8] {
    let start = line
        .iter()
        .position(|&b| b != b' ' && b != b'\t' && b != b'\r' && b != b'\n')
        .unwrap_or(line.len());
    let end = line[..]
        .iter()
        .rposition(|&b| b != b' ' && b != b'\t' && b != b'\r' && b != b'\n')
        .map(|idx| idx + 1)
        .unwrap_or(start);
    &line[start..end]
}

fn trim_leading_ascii_space(line: &[u8]) -> &[u8] {
    let start = line
        .iter()
        .position(|&b| b != b' ' && b != b'\t' && b != b'\r' && b != b'\n')
        .unwrap_or(line.len());
    &line[start..]
}

/// Returns true when the text before the attr call still looks like the start of a
/// standalone statement. This keeps skipping nested expression uses such as
/// `(attr :foo).should` or `{ attr_reader(:name) }`, while allowing direct
/// statements in minified `foo;attr_reader :bar` code and `case attr 'x'`.
fn is_statement_prefix(prefix: &[u8]) -> bool {
    let trimmed = trim_ascii_space(prefix);
    trimmed.is_empty() || trimmed.ends_with(b";") || trimmed == b"case"
}

fn same_line_following_statement(after_call: &[u8]) -> Option<&[u8]> {
    let rest = trim_leading_ascii_space(after_call).strip_prefix(b";")?;
    Some(trim_leading_ascii_space(rest))
}

/// Returns true if the trimmed line starts with a keyword that ends a block branch,
/// meaning the attr accessor is the last statement in its branch and RuboCop would
/// not fire (no right_sibling in the AST).
fn is_block_boundary_keyword(trimmed: &[u8]) -> bool {
    // Keywords that terminate a branch: end, else, elsif, when, in, rescue, ensure
    const KEYWORDS: &[&[u8]] = &[
        b"end", b"else", b"elsif", b"when", b"in ", b"rescue", b"ensure",
    ];
    for &kw in KEYWORDS {
        if trimmed.starts_with(kw) {
            let after = trimmed.get(kw.len());
            // "in " already has trailing space in the keyword, so after could be anything
            if kw == b"in " {
                return true;
            }
            if after.is_none()
                || matches!(
                    after,
                    Some(b' ')
                        | Some(b'\n')
                        | Some(b'\r')
                        | Some(b'#')
                        | Some(b';')
                        | Some(b'.')
                        | Some(b')')
                        | Some(b']')
                        | Some(b',')
                )
            {
                return true;
            }
        }
    }
    // Also handle `}` or `}.method` — closing brace of a block
    if trimmed.first() == Some(&b'}') {
        return true;
    }
    false
}

fn is_expression_closer(trimmed: &[u8]) -> bool {
    matches!(trimmed.first(), Some(b')') | Some(b']'))
}

/// Returns true if the trimmed line contains a modifier `if` or `unless` keyword,
/// indicating the method call is conditional (e.g., `alias_method :foo, :bar if cond`).
/// RuboCop's AST sees this as an IfNode wrapping the send, not a plain allowed method.
fn has_modifier_conditional(trimmed: &[u8]) -> bool {
    contains_standalone_keyword_outside_strings_or_comments(trimmed, b"if")
        || contains_standalone_keyword_outside_strings_or_comments(trimmed, b"unless")
}

fn contains_standalone_keyword_outside_strings_or_comments(source: &[u8], keyword: &[u8]) -> bool {
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    while i < source.len() {
        let byte = source[i];

        if in_single {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }

        if in_double {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_double = false;
            }
            i += 1;
            continue;
        }

        match byte {
            b'#' => break,
            b'\'' => {
                in_single = true;
                i += 1;
                continue;
            }
            b'"' => {
                in_double = true;
                i += 1;
                continue;
            }
            _ => {}
        }

        if i + keyword.len() <= source.len() && &source[i..i + keyword.len()] == keyword {
            let before_ok = i == 0 || !is_identifier_char(source[i - 1]);
            let after_idx = i + keyword.len();
            let after_ok = after_idx == source.len() || !is_identifier_char(source[after_idx]);
            if before_ok && after_ok {
                return true;
            }
        }

        i += 1;
    }

    false
}

fn is_identifier_char(byte: u8) -> bool {
    matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')
}

/// Returns true if the line is a `# rubocop:enable ...` directive comment.
/// RuboCop treats enable directives followed by a blank line as valid separators.
fn is_enable_directive_comment(line: &[u8]) -> bool {
    let trimmed: Vec<u8> = line
        .iter()
        .copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect();
    trimmed.starts_with(b"# rubocop:enable ")
}

fn is_attr_method_line(trimmed: &[u8]) -> bool {
    for &attr in ATTRIBUTE_METHODS {
        if trimmed.starts_with(attr) {
            let after = trimmed.get(attr.len());
            if after.is_none()
                || matches!(after, Some(b' ') | Some(b'(') | Some(b'\n') | Some(b':'))
            {
                // If the attr call has a modifier `if`/`unless`, RuboCop's AST sees
                // it as an IfNode, not a plain attr call — not a sibling attr accessor.
                if has_modifier_conditional(trimmed) {
                    return false;
                }
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        EmptyLinesAroundAttributeAccessor,
        "cops/layout/empty_lines_around_attribute_accessor"
    );
    crate::cop_autocorrect_fixture_tests!(
        EmptyLinesAroundAttributeAccessor,
        "cops/layout/empty_lines_around_attribute_accessor"
    );
}
