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
/// **Remaining gap:** 3 FNs — 2 from camping (minified Ruby, mid-line attr calls) and 1 from
/// CocoaPods (`attr_accessor name` followed by `alias_method ... if boolean` where the `if`
/// modifier makes RuboCop treat it as non-allowed, but nitrocop's line-based check sees
/// `alias_method` and allows it).
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

        // The attr call must be a standalone statement on its line. If the call
        // is part of a larger expression (e.g., `(attr :foo).should`, or inside
        // single-line block braces `{ attr_reader :name }`), skip it.
        // RuboCop handles this via `node.right_sibling` which only exists when
        // the node is a direct child of a statements body. We approximate by
        // checking that the call starts at the first non-whitespace position
        // of its line.
        let (start_line, start_col) = source.offset_to_line_col(loc.start_offset());
        let lines: Vec<&[u8]> = source.lines().collect();
        if start_line > 0 && (start_line - 1) < lines.len() {
            let call_line = lines[start_line - 1];
            let indent = call_line
                .iter()
                .take_while(|&&b| b == b' ' || b == b'\t')
                .count();
            if start_col != indent {
                return;
            }
        }
        let (last_line, last_col) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));

        // If there is non-whitespace, non-comment content after the call's end on the
        // same line, the attr call is part of a larger expression (e.g.,
        // `attr_accessor :parser unless method_defined? :parser`). Skip it.
        // RuboCop handles this via AST: the call is inside an UnlessModifierNode and
        // has no right_sibling in a statements body.
        if last_line > 0 && (last_line - 1) < lines.len() {
            let call_end_line = lines[last_line - 1];
            let after_call = &call_end_line[(last_col + 1).min(call_end_line.len())..];
            let has_trailing_code = after_call
                .iter()
                .find(|&&b| b != b' ' && b != b'\t')
                .is_some_and(|&b| b != b'\n' && b != b'\r' && b != b'#');
            if has_trailing_code {
                return;
            }
        }

        // Check if the next line exists and is not empty
        if last_line >= lines.len() {
            return; // End of file
        }

        let next_line = lines[last_line]; // 0-indexed: last_line (1-based) maps to lines[last_line] for next

        // If next line is blank, no offense
        if is_blank_or_whitespace_line(next_line) {
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
        if is_block_boundary_keyword(&next_trimmed) {
            return;
        }

        // If next line is another attribute accessor, no offense
        if is_attr_method_line(&next_trimmed) {
            return;
        }

        // If next line is a comment, look past comments to see if the next code line
        // is another attribute accessor, an allowed method, alias, a block boundary,
        // or if there's a blank line after the comments (no offense needed).
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
            let mut idx = last_line + 1;
            while idx < lines.len() {
                let line_trimmed: Vec<u8> = lines[idx]
                    .iter()
                    .copied()
                    .skip_while(|&b| b == b' ' || b == b'\t')
                    .collect();
                if is_blank_or_whitespace_line(lines[idx]) {
                    return; // blank line after comments means no offense
                }
                if line_trimmed.starts_with(b"#") {
                    idx += 1;
                    continue; // skip comments
                }
                if is_attr_method_line(&line_trimmed) {
                    return;
                }
                if is_block_boundary_keyword(&line_trimmed) {
                    return;
                }
                if _allow_alias_syntax && line_trimmed.starts_with(b"alias ") {
                    return;
                }
                for am in &allowed_for_comment {
                    let mb = am.as_bytes();
                    if line_trimmed.starts_with(mb) {
                        let after = line_trimmed.get(mb.len());
                        if after.is_none()
                            || matches!(after, Some(b' ') | Some(b'(') | Some(b'\n') | Some(b'\r'))
                        {
                            return;
                        }
                    }
                }
                break;
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
                if after.is_none()
                    || matches!(after, Some(b' ') | Some(b'(') | Some(b'\n') | Some(b'\r'))
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
            if let Some(offset) = source.line_col_to_offset(last_line + 1, 0) {
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
                    Some(b' ') | Some(b'\n') | Some(b'\r') | Some(b'#') | Some(b';') | Some(b'.')
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

fn is_attr_method_line(trimmed: &[u8]) -> bool {
    for &attr in ATTRIBUTE_METHODS {
        if trimmed.starts_with(attr) {
            let after = trimmed.get(attr.len());
            if after.is_none() || matches!(after, Some(b' ') | Some(b'(') | Some(b'\n')) {
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
