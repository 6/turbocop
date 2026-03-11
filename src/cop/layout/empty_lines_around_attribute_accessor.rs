use crate::cop::node_type::CALL_NODE;
use crate::cop::util::is_blank_line;
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
/// **Remaining gap:** 1 FN (nitrocop misses an offense RuboCop catches). Not investigated.
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
        let (last_line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
        let lines: Vec<&[u8]> = source.lines().collect();

        // Check if the next line exists and is not empty
        if last_line >= lines.len() {
            return; // End of file
        }

        let next_line = lines[last_line]; // 0-indexed: last_line (1-based) maps to lines[last_line] for next

        // If next line is blank, no offense
        if is_blank_line(next_line) {
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
        // is another attribute accessor. This allows YARD-style documented accessors:
        //   attr_reader :value
        //   # @return [Exception, nil]
        //   attr_reader :handled_error
        if next_trimmed.starts_with(b"#") {
            let mut idx = last_line + 1;
            while idx < lines.len() {
                let line_trimmed: Vec<u8> = lines[idx]
                    .iter()
                    .copied()
                    .skip_while(|&b| b == b' ' || b == b'\t')
                    .collect();
                if line_trimmed.is_empty() || line_trimmed == b"\n" || line_trimmed == b"\r\n" {
                    break; // blank line means end of group
                }
                if line_trimmed.starts_with(b"#") {
                    idx += 1;
                    continue; // skip comments
                }
                if is_attr_method_line(&line_trimmed) {
                    return;
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
                    Some(b' ') | Some(b'\n') | Some(b'\r') | Some(b'#') | Some(b';')
                )
            {
                return true;
            }
        }
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
