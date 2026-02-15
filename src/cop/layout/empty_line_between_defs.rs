use crate::cop::util::line_at;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLineBetweenDefs;

fn is_blank(line: &[u8]) -> bool {
    line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r')
}

/// Check if a line is a scope-opening keyword line (class, module, def, do, begin, or `{`).
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

impl Cop for EmptyLineBetweenDefs {
    fn name(&self) -> &'static str {
        "Layout/EmptyLineBetweenDefs"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
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
                return Vec::new();
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
                return Vec::new();
            }
            is_def_like_macro = false;
            let loc = class_node.class_keyword_loc();
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            let end_line = source.offset_to_line_col(class_node.end_keyword_loc().start_offset()).0;
            (line, col, end_line == line)
        } else if let Some(module_node) = node.as_module_node() {
            if !empty_between_modules {
                return Vec::new();
            }
            is_def_like_macro = false;
            let loc = module_node.module_keyword_loc();
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            let end_line = source.offset_to_line_col(module_node.end_keyword_loc().start_offset()).0;
            (line, col, end_line == line)
        } else if let Some(call_node) = node.as_call_node() {
            // DefLikeMacros: treat matching call nodes as definition-like
            if def_like_macros.is_empty() || call_node.receiver().is_some() {
                return Vec::new();
            }
            let name = std::str::from_utf8(call_node.name().as_slice()).unwrap_or("");
            if !def_like_macros.iter().any(|m| m == name) {
                return Vec::new();
            }
            is_def_like_macro = true;
            let loc = call_node.location();
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            // Macro calls are always single-line definitions
            (line, col, true)
        } else {
            return Vec::new();
        };

        // AllowAdjacentOneLineDefs: skip single-line defs (but not def-like macros,
        // which are always single-line and should still require blank lines)
        if allow_adjacent && is_single_line && !is_def_like_macro {
            return Vec::new();
        }

        // Skip if def is on the first line
        if def_line <= 1 {
            return Vec::new();
        }

        // Scan backwards from the def line, counting blank lines.
        // Only flag if we hit `end` or a previous def-like macro without enough blank lines.
        let mut check_line = def_line - 1; // 1-indexed
        let mut blank_count = 0;
        loop {
            if check_line < 1 {
                return Vec::new();
            }
            let line = match line_at(source, check_line) {
                Some(l) => l,
                None => return Vec::new(),
            };
            if is_blank(line) {
                blank_count += 1;
                check_line -= 1;
                continue;
            }
            if is_comment_line(line) {
                check_line -= 1;
                continue;
            }
            // Check if this is an opening line (class, module, def, etc.)
            if is_opening_line(line) {
                return Vec::new();
            }
            // Check if this line is `end` (with optional leading whitespace)
            let trimmed: Vec<u8> = line
                .iter()
                .copied()
                .skip_while(|&b| b == b' ' || b == b'\t')
                .collect();
            if trimmed == b"end" || trimmed.starts_with(b"end ") || trimmed.starts_with(b"end\t") {
                // Previous definition ended here — check blank line count
                if blank_count >= number_of_empty_lines {
                    return Vec::new();
                }
                break;
            }
            // Check if this line is a def-like macro call
            if !def_like_macros.is_empty() {
                let trimmed_str = std::str::from_utf8(&trimmed).unwrap_or("");
                let is_macro_line = def_like_macros.iter().any(|m| {
                    trimmed_str == m.as_str()
                        || trimmed_str.starts_with(&format!("{m} "))
                        || trimmed_str.starts_with(&format!("{m}("))
                });
                if is_macro_line {
                    if blank_count >= number_of_empty_lines {
                        return Vec::new();
                    }
                    break;
                }
            }
            // Something else (e.g., LONG_DESC, attr_accessor, etc.) — don't flag
            return Vec::new();
        }

        let msg = if number_of_empty_lines == 1 {
            "Use empty lines between method definitions.".to_string()
        } else {
            format!("Use {number_of_empty_lines} empty lines between method definitions.")
        };

        vec![self.diagnostic(source, def_line, def_col, msg)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(EmptyLineBetweenDefs, "cops/layout/empty_line_between_defs");

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
        assert_eq!(diags.len(), 1, "Missing blank line between defs should trigger");
        assert_eq!(diags[0].location.line, 5);
    }

    #[test]
    fn number_of_empty_lines_requires_multiple() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("NumberOfEmptyLines".into(), serde_yml::Value::Number(2.into())),
            ]),
            ..CopConfig::default()
        };
        // One blank line between defs should be flagged when 2 required
        let src = b"class Foo\n  def bar\n    1\n  end\n\n  def baz\n    2\n  end\nend\n";
        let diags = run_cop_full_with_config(&EmptyLineBetweenDefs, src, config.clone());
        assert_eq!(diags.len(), 1, "Should flag when fewer than NumberOfEmptyLines blank lines");

        // Two blank lines should be accepted
        let src2 = b"class Foo\n  def bar\n    1\n  end\n\n\n  def baz\n    2\n  end\nend\n";
        let diags2 = run_cop_full_with_config(&EmptyLineBetweenDefs, src2, config);
        assert!(diags2.is_empty(), "Should accept when NumberOfEmptyLines blank lines present");
    }

    #[test]
    fn def_like_macros_flags_missing_blank_line() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("DefLikeMacros".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("scope".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        // Two scope macros without blank line
        let src = b"class Foo\n  scope :active, -> { where(active: true) }\n  scope :recent, -> { where(recent: true) }\nend\n";
        let diags = run_cop_full_with_config(&EmptyLineBetweenDefs, src, config.clone());
        assert_eq!(diags.len(), 1, "Missing blank line between def-like macros should trigger");

        // With blank line — no offense
        let src2 = b"class Foo\n  scope :active, -> { where(active: true) }\n\n  scope :recent, -> { where(recent: true) }\nend\n";
        let diags2 = run_cop_full_with_config(&EmptyLineBetweenDefs, src2, config);
        assert!(diags2.is_empty(), "Blank line between def-like macros should be accepted");
    }

    #[test]
    fn empty_between_method_defs_false_skips_methods() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EmptyLineBetweenMethodDefs".into(), serde_yml::Value::Bool(false)),
            ]),
            ..CopConfig::default()
        };
        let src = b"class Foo\n  def bar\n    1\n  end\n  def baz\n    2\n  end\nend\n";
        let diags = run_cop_full_with_config(&EmptyLineBetweenDefs, src, config);
        assert!(diags.is_empty(), "Should not flag when EmptyLineBetweenMethodDefs is false");
    }
}
