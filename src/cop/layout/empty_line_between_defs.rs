use crate::cop::node_type::{CALL_NODE, CLASS_NODE, DEF_NODE, MODULE_NODE};
use crate::cop::util::line_at;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLineBetweenDefs;

fn is_blank(line: &[u8]) -> bool {
    line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r')
}

/// Check if a line is a single-line def (def ... end on the same line).
fn is_single_line_def(line: &[u8]) -> bool {
    let trimmed: Vec<u8> = line
        .iter()
        .copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect();
    if !trimmed.starts_with(b"def ") && !trimmed.starts_with(b"def(") {
        return false;
    }
    // Check for `end` token at the end of the line (with possible trailing whitespace)
    let end_trimmed: Vec<u8> = trimmed
        .iter()
        .rev()
        .skip_while(|&&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r')
        .copied()
        .collect::<Vec<u8>>()
        .into_iter()
        .rev()
        .collect();
    end_trimmed.ends_with(b"end")
        && (end_trimmed.len() == 3
            || end_trimmed[end_trimmed.len() - 4] == b' '
            || end_trimmed[end_trimmed.len() - 4] == b';')
}

/// Check if a line is a scope-opening keyword line (class, module, def, do, begin, or `{`).
/// Single-line defs (`def foo; end`) are NOT considered scope openers — they are complete definitions.
fn is_opening_line(line: &[u8]) -> bool {
    let trimmed: Vec<u8> = line
        .iter()
        .copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect();
    // Strip trailing whitespace for end-of-line checks
    let end_trimmed = trimmed
        .iter()
        .rposition(|&b| b != b' ' && b != b'\t' && b != b'\n' && b != b'\r')
        .map_or(&[] as &[u8], |i| &trimmed[..=i]);

    // Single-line defs are complete definitions, not scope openers
    if (trimmed.starts_with(b"def ") || trimmed.starts_with(b"def(")) && is_single_line_def(line) {
        return false;
    }

    trimmed.starts_with(b"class ")
        || trimmed.starts_with(b"module ")
        || trimmed.starts_with(b"def ")
        || trimmed.starts_with(b"do")
        || trimmed.starts_with(b"begin")
        || trimmed.starts_with(b"{")
        // Lines ending with `do` or `do |...|` (block opening)
        || end_trimmed.ends_with(b" do")
        || end_trimmed.ends_with(b"|")
            && end_trimmed.windows(4).any(|w| w == b" do ")
}

/// Check if a line is a comment line.
fn is_comment_line(line: &[u8]) -> bool {
    let trimmed: Vec<u8> = line
        .iter()
        .copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect();
    trimmed.starts_with(b"#")
}

/// Check if an `end` keyword on the given line closes a definition
/// (def/class/module) rather than a block (do..end), conditional (if..end),
/// loop (while/until/for..end), begin..end, or case..end.
/// Scans backwards to find the matching opener by tracking nesting.
fn is_definition_end(source: &SourceFile, end_line: usize) -> bool {
    let lines: Vec<&[u8]> = source.lines().collect();
    if end_line < 1 || end_line > lines.len() {
        return false;
    }

    let end_indent = {
        let line = lines[end_line - 1];
        line.iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count()
    };

    // Scan backwards looking for the opener at the same indentation level
    let mut nesting: usize = 0;
    let mut line_num = end_line - 1; // start from line before `end`
    while line_num >= 1 {
        let line = lines[line_num - 1];
        let trimmed: Vec<u8> = line
            .iter()
            .copied()
            .skip_while(|&b| b == b' ' || b == b'\t')
            .collect();
        let indent = line
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();

        // Strip trailing whitespace for end-of-line checks
        let end_trimmed = trimmed
            .iter()
            .rposition(|&b| b != b' ' && b != b'\t' && b != b'\n' && b != b'\r')
            .map_or(&[] as &[u8], |i| &trimmed[..=i]);

        // Check if this line has `end` at the same indent level (nested end)
        if indent == end_indent
            && (trimmed == b"end"
                || trimmed.starts_with(b"end ")
                || trimmed.starts_with(b"end\t")
                || trimmed.starts_with(b"end\n")
                || trimmed.starts_with(b"end\r"))
        {
            nesting += 1;
            line_num -= 1;
            continue;
        }

        // Check for openers at matching indent level
        if indent == end_indent {
            let is_def_opener = trimmed.starts_with(b"def ")
                || trimmed.starts_with(b"class ")
                || trimmed.starts_with(b"module ");

            let is_non_def_opener = trimmed.starts_with(b"if ")
                || trimmed.starts_with(b"unless ")
                || trimmed.starts_with(b"while ")
                || trimmed.starts_with(b"until ")
                || trimmed.starts_with(b"for ")
                || trimmed.starts_with(b"case ")
                || trimmed.starts_with(b"case\n")
                || trimmed == b"case"
                || trimmed.starts_with(b"begin")
                || trimmed.starts_with(b"do")
                || end_trimmed.ends_with(b" do")
                || (end_trimmed.ends_with(b"|") && end_trimmed.windows(4).any(|w| w == b" do "));

            if is_def_opener || is_non_def_opener {
                if nesting == 0 {
                    return is_def_opener;
                }
                nesting -= 1;
            }
        }

        if line_num == 0 {
            break;
        }
        line_num -= 1;
    }

    // Couldn't find a matching opener — conservatively treat as definition
    true
}

impl Cop for EmptyLineBetweenDefs {
    fn name(&self) -> &'static str {
        "Layout/EmptyLineBetweenDefs"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CLASS_NODE, DEF_NODE, MODULE_NODE]
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
        let empty_between_methods = config.get_bool("EmptyLineBetweenMethodDefs", true);
        let empty_between_classes = config.get_bool("EmptyLineBetweenClassDefs", true);
        let empty_between_modules = config.get_bool("EmptyLineBetweenModuleDefs", true);
        let def_like_macros = config.get_string_array("DefLikeMacros").unwrap_or_default();
        let number_of_empty_lines = config.get_usize("NumberOfEmptyLines", 1);
        let allow_adjacent = config.get_bool("AllowAdjacentOneLineDefs", true);

        // Determine what kind of definition node this is
        let is_def_like_macro;
        let (def_line, def_col, is_single_line) = if let Some(def_node) = node.as_def_node() {
            if !empty_between_methods {
                return;
            }
            is_def_like_macro = false;
            let loc = def_node.def_keyword_loc();
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            let end_line = if let Some(el) = def_node.end_keyword_loc() {
                source.offset_to_line_col(el.start_offset()).0
            } else {
                line
            };
            (line, col, end_line == line)
        } else if let Some(class_node) = node.as_class_node() {
            if !empty_between_classes {
                return;
            }
            is_def_like_macro = false;
            let loc = class_node.class_keyword_loc();
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            let end_line = source
                .offset_to_line_col(class_node.end_keyword_loc().start_offset())
                .0;
            (line, col, end_line == line)
        } else if let Some(module_node) = node.as_module_node() {
            if !empty_between_modules {
                return;
            }
            is_def_like_macro = false;
            let loc = module_node.module_keyword_loc();
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            let end_line = source
                .offset_to_line_col(module_node.end_keyword_loc().start_offset())
                .0;
            (line, col, end_line == line)
        } else if let Some(call_node) = node.as_call_node() {
            // DefLikeMacros: treat matching call nodes as definition-like
            if def_like_macros.is_empty() || call_node.receiver().is_some() {
                return;
            }
            let name = std::str::from_utf8(call_node.name().as_slice()).unwrap_or("");
            if !def_like_macros.iter().any(|m| m == name) {
                return;
            }
            is_def_like_macro = true;
            let loc = call_node.location();
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            // Macro calls are always single-line definitions
            (line, col, true)
        } else {
            return;
        };

        // AllowAdjacentOneLineDefs: skip single-line defs (but not def-like macros,
        // which are always single-line and should still require blank lines)
        if allow_adjacent && is_single_line && !is_def_like_macro {
            return;
        }

        // Skip if def is on the first line
        if def_line <= 1 {
            return;
        }

        // Scan backwards from the def line, counting blank lines.
        // Comments are NOT skipped — they are treated as non-blank content.
        // When we hit a non-blank line (comment or code), we stop counting
        // blank lines and then determine whether it's a definition boundary.
        // If the non-blank line is a comment, we scan past the comment block
        // to find the actual boundary (end, opening line, etc.).
        let mut check_line = def_line - 1; // 1-indexed
        let mut blank_count = 0;

        // Phase 1: Count blank lines immediately above the def.
        loop {
            if check_line < 1 {
                return;
            }
            let line = match line_at(source, check_line) {
                Some(l) => l,
                None => return,
            };
            if is_blank(line) {
                blank_count += 1;
                check_line -= 1;
                continue;
            }
            // Hit a non-blank line — stop counting blanks.
            break;
        }

        // Phase 2: If the first non-blank line is a comment, scan past the
        // comment block (and any interleaved blank lines) to find the actual
        // boundary line.  Accumulate any additional blank lines found between
        // the comment block and the boundary into blank_count so that the
        // total reflects ALL blank lines in the gap between the two defs.
        loop {
            if check_line < 1 {
                return;
            }
            let line = match line_at(source, check_line) {
                Some(l) => l,
                None => return,
            };
            if is_comment_line(line) {
                check_line -= 1;
                continue;
            }
            if is_blank(line) {
                blank_count += 1;
                check_line -= 1;
                continue;
            }
            // Found a non-blank, non-comment line — this is the boundary.
            break;
        }

        // RuboCop's `multiple_blank_lines_groups?`: skip enforcement when blank
        // lines are interspersed with non-blank (comment) lines between the two
        // defs. Check: does any blank line appear AFTER any non-blank line?
        // (i.e., max blank index > min non-blank index in forward order)
        {
            let mut last_blank_idx: Option<usize> = None;
            let mut first_non_blank_idx: Option<usize> = None;
            for (i, line_num) in (check_line + 1..def_line).enumerate() {
                let line = match line_at(source, line_num) {
                    Some(l) => l,
                    None => continue,
                };
                if is_blank(line) {
                    last_blank_idx = Some(i);
                } else if first_non_blank_idx.is_none() {
                    first_non_blank_idx = Some(i);
                }
            }
            if let (Some(last_blank), Some(first_non_blank)) = (last_blank_idx, first_non_blank_idx)
            {
                if last_blank > first_non_blank {
                    return;
                }
            }
        }

        // Phase 3: Evaluate the boundary line.
        let boundary_line = match line_at(source, check_line) {
            Some(l) => l,
            None => return,
        };

        // Check if this is an opening line (class, module, def, etc.)
        if is_opening_line(boundary_line) {
            return;
        }

        // Determine if the boundary line is a definition boundary
        let is_def_boundary = if is_single_line_def(boundary_line) {
            // Single-line def (def ... end on same line) is a definition boundary
            true
        } else {
            let trimmed: Vec<u8> = boundary_line
                .iter()
                .copied()
                .skip_while(|&b| b == b' ' || b == b'\t')
                .collect();
            if trimmed == b"end" || trimmed.starts_with(b"end ") || trimmed.starts_with(b"end\t") {
                // Only treat as a previous definition boundary if the `end`
                // closes a def/class/module (not a block, conditional, etc.)
                if !is_definition_end(source, check_line) {
                    return;
                }
                true
            } else if !def_like_macros.is_empty() {
                let trimmed_str = std::str::from_utf8(&trimmed).unwrap_or("");
                let is_macro_line = def_like_macros.iter().any(|m| {
                    trimmed_str == m.as_str()
                        || trimmed_str.starts_with(&format!("{m} "))
                        || trimmed_str.starts_with(&format!("{m}("))
                });
                if !is_macro_line {
                    return;
                }
                true
            } else {
                false
            }
        };

        if !is_def_boundary {
            return;
        }

        // Previous definition ended here — check blank line count
        if blank_count == number_of_empty_lines {
            return;
        }

        let msg = if blank_count > number_of_empty_lines {
            format!(
                "Expected {number_of_empty_lines} empty line between method definitions; found {blank_count}."
            )
        } else if number_of_empty_lines == 1 {
            "Use empty lines between method definitions.".to_string()
        } else {
            format!("Use {number_of_empty_lines} empty lines between method definitions.")
        };

        let mut diag = self.diagnostic(source, def_line, def_col, msg);
        if let Some(ref mut corr) = corrections {
            if blank_count < number_of_empty_lines {
                // Insert missing blank lines before the def line
                let lines_to_add = number_of_empty_lines - blank_count;
                if let Some(offset) = source.line_col_to_offset(def_line, 0) {
                    corr.push(crate::correction::Correction {
                        start: offset,
                        end: offset,
                        replacement: "\n".repeat(lines_to_add),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diag.corrected = true;
                }
            }
            // TODO: autocorrect for excess blank lines (remove extra lines)
        }
        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(EmptyLineBetweenDefs, "cops/layout/empty_line_between_defs");
    crate::cop_autocorrect_fixture_tests!(
        EmptyLineBetweenDefs,
        "cops/layout/empty_line_between_defs"
    );

    #[test]
    fn single_def_no_offense() {
        let src = b"class Foo\n  def bar\n    1\n  end\nend\n";
        let diags = run_cop_full(&EmptyLineBetweenDefs, src);
        assert!(diags.is_empty(), "Single def should not trigger offense");
    }

    #[test]
    fn def_after_end_without_blank_line() {
        let src = b"class Foo\n  def bar\n    1\n  end\n  def baz\n    2\n  end\nend\n";
        let diags = run_cop_full(&EmptyLineBetweenDefs, src);
        assert_eq!(
            diags.len(),
            1,
            "Missing blank line between defs should trigger"
        );
        assert_eq!(diags[0].location.line, 5);
    }

    #[test]
    fn number_of_empty_lines_requires_multiple() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "NumberOfEmptyLines".into(),
                serde_yml::Value::Number(2.into()),
            )]),
            ..CopConfig::default()
        };
        // One blank line between defs should be flagged when 2 required
        let src = b"class Foo\n  def bar\n    1\n  end\n\n  def baz\n    2\n  end\nend\n";
        let diags = run_cop_full_with_config(&EmptyLineBetweenDefs, src, config.clone());
        assert_eq!(
            diags.len(),
            1,
            "Should flag when fewer than NumberOfEmptyLines blank lines"
        );

        // Two blank lines should be accepted
        let src2 = b"class Foo\n  def bar\n    1\n  end\n\n\n  def baz\n    2\n  end\nend\n";
        let diags2 = run_cop_full_with_config(&EmptyLineBetweenDefs, src2, config);
        assert!(
            diags2.is_empty(),
            "Should accept when NumberOfEmptyLines blank lines present"
        );
    }

    #[test]
    fn def_like_macros_flags_missing_blank_line() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "DefLikeMacros".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("scope".into())]),
            )]),
            ..CopConfig::default()
        };
        // Two scope macros without blank line
        let src = b"class Foo\n  scope :active, -> { where(active: true) }\n  scope :recent, -> { where(recent: true) }\nend\n";
        let diags = run_cop_full_with_config(&EmptyLineBetweenDefs, src, config.clone());
        assert_eq!(
            diags.len(),
            1,
            "Missing blank line between def-like macros should trigger"
        );

        // With blank line — no offense
        let src2 = b"class Foo\n  scope :active, -> { where(active: true) }\n\n  scope :recent, -> { where(recent: true) }\nend\n";
        let diags2 = run_cop_full_with_config(&EmptyLineBetweenDefs, src2, config);
        assert!(
            diags2.is_empty(),
            "Blank line between def-like macros should be accepted"
        );
    }

    #[test]
    fn empty_between_method_defs_false_skips_methods() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EmptyLineBetweenMethodDefs".into(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        let src = b"class Foo\n  def bar\n    1\n  end\n  def baz\n    2\n  end\nend\n";
        let diags = run_cop_full_with_config(&EmptyLineBetweenDefs, src, config);
        assert!(
            diags.is_empty(),
            "Should not flag when EmptyLineBetweenMethodDefs is false"
        );
    }
}
