use crate::cop::util::assignment_context_base_col;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CASE_NODE, CLASS_NODE, IF_NODE, MODULE_NODE, UNTIL_NODE, WHILE_NODE};

pub struct EndAlignment;

/// Check if a specific operator (like `<<`) appears on the same line before `keyword_offset`.
fn has_operator_before_keyword(source: &SourceFile, keyword_offset: usize, op: &[u8]) -> bool {
    let bytes = source.as_bytes();
    let mut line_start = keyword_offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    let before = &bytes[line_start..keyword_offset];
    before.windows(op.len()).any(|w| w == op)
}

/// Get the indentation level (first non-whitespace column) of the line containing `offset`.
fn line_indent(source: &SourceFile, offset: usize) -> usize {
    let bytes = source.as_bytes();
    let mut line_start = offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    let mut indent = 0;
    while line_start + indent < bytes.len()
        && (bytes[line_start + indent] == b' ' || bytes[line_start + indent] == b'\t')
    {
        indent += 1;
    }
    indent
}

impl Cop for EndAlignment {
    fn name(&self) -> &'static str {
        "Layout/EndAlignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CASE_NODE, CLASS_NODE, IF_NODE, MODULE_NODE, UNTIL_NODE, WHILE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let style = config.get_str("EnforcedStyleAlignWith", "keyword");
        if let Some(class_node) = node.as_class_node() {
            diagnostics.extend(self.check_keyword_end(
                source,
                class_node.class_keyword_loc().start_offset(),
                class_node.end_keyword_loc().start_offset(),
                "class",
                style,
            ));
            return;
        }

        if let Some(module_node) = node.as_module_node() {
            diagnostics.extend(self.check_keyword_end(
                source,
                module_node.module_keyword_loc().start_offset(),
                module_node.end_keyword_loc().start_offset(),
                "module",
                style,
            ));
            return;
        }

        if let Some(if_node) = node.as_if_node() {
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };
            // Only check top-level if/unless, not elsif
            let kw_slice = kw_loc.as_slice();
            if kw_slice != b"if" && kw_slice != b"unless" {
                return;
            }
            let end_kw_loc = match if_node.end_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };
            let keyword = if kw_slice == b"if" { "if" } else { "unless" };
            diagnostics.extend(self.check_keyword_end(
                source,
                kw_loc.start_offset(),
                end_kw_loc.start_offset(),
                keyword,
                style,
            ));
            return;
        }

        if let Some(while_node) = node.as_while_node() {
            let kw_loc = while_node.keyword_loc();
            if let Some(end_loc) = while_node.closing_loc() {
                diagnostics.extend(self.check_keyword_end(
                    source,
                    kw_loc.start_offset(),
                    end_loc.start_offset(),
                    "while",
                    style,
                ));
                return;
            }
        }

        if let Some(until_node) = node.as_until_node() {
            let kw_loc = until_node.keyword_loc();
            if let Some(end_loc) = until_node.closing_loc() {
                diagnostics.extend(self.check_keyword_end(
                    source,
                    kw_loc.start_offset(),
                    end_loc.start_offset(),
                    "until",
                    style,
                ));
                return;
            }
        }

        if let Some(case_node) = node.as_case_node() {
            let kw_loc = case_node.case_keyword_loc();
            let end_loc = case_node.end_keyword_loc();
            diagnostics.extend(self.check_keyword_end(
                source,
                kw_loc.start_offset(),
                end_loc.start_offset(),
                "case",
                style,
            ));
            return;
        }

        // NOTE: `begin` blocks are not checked here — that's handled by
        // Layout/BeginEndAlignment which supports variable-aligned `end`.

    }
}

impl EndAlignment {
    fn check_keyword_end(
        &self,
        source: &SourceFile,
        kw_offset: usize,
        end_offset: usize,
        keyword: &str,
        style: &str,
    ) -> Vec<Diagnostic> {
        let (kw_line, kw_col) = source.offset_to_line_col(kw_offset);
        let (end_line, end_col) = source.offset_to_line_col(end_offset);

        // Skip single-line constructs (e.g., `class Foo; end`)
        if kw_line == end_line {
            return Vec::new();
        }

        let expected_col = match style {
            "variable" => {
                // Variable alignment: if keyword is RHS of an assignment
                // or operator like `<<`, align end with the line start
                // (the variable). Otherwise fall back to keyword alignment.
                if let Some(base_col) = assignment_context_base_col(source, kw_offset) {
                    base_col
                } else if has_operator_before_keyword(source, kw_offset, b"<<") {
                    line_indent(source, kw_offset)
                } else {
                    kw_col
                }
            }
            "start_of_line" => {
                // Align with the start of the line where the keyword appears
                let bytes = source.as_bytes();
                let mut line_start = kw_offset;
                while line_start > 0 && bytes[line_start - 1] != b'\n' {
                    line_start -= 1;
                }
                let mut indent = 0;
                while line_start + indent < bytes.len() && bytes[line_start + indent] == b' ' {
                    indent += 1;
                }
                indent
            }
            _ => kw_col, // "keyword" (default): align with keyword
        };

        if end_col != expected_col {
            let msg = match style {
                "variable" | "start_of_line" => {
                    format!("Align `end` with `{keyword}`.")
                }
                _ => format!("Align `end` with `{keyword}`."),
            };
            return vec![self.diagnostic(source, end_line, end_col, msg)];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(EndAlignment, "cops/layout/end_alignment");

    #[test]
    fn modifier_if_no_offense() {
        let source = b"x = 1 if true\n";
        let diags = run_cop_full(&EndAlignment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn variable_style_aligns_with_assignment() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyleAlignWith".into(), serde_yml::Value::String("variable".into())),
            ]),
            ..CopConfig::default()
        };
        // `x = if ...` with `end` at column 0 (start of line)
        let src = b"x = if true\n  1\nend\n";
        let diags = run_cop_full_with_config(&EndAlignment, src, config);
        assert!(diags.is_empty(), "variable style should accept end at start of line");
    }

    #[test]
    fn variable_style_no_assignment_falls_back_to_keyword() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyleAlignWith".into(), serde_yml::Value::String("variable".into())),
            ]),
            ..CopConfig::default()
        };
        // `super || if ...` — not an assignment, `end` should align with `if` (col 13)
        let src = b"  def foo\n    super || if true\n                 1\n             end\n  end\n";
        let diags = run_cop_full_with_config(&EndAlignment, src, config);
        assert!(diags.is_empty(), "variable style without assignment should align end with keyword: {:?}", diags);
    }

    #[test]
    fn variable_style_shovel_operator_aligns_with_line_start() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyleAlignWith".into(), serde_yml::Value::String("variable".into())),
            ]),
            ..CopConfig::default()
        };
        // `buf << if ...` — end aligns with line start (buf), not with `if`
        let src = b"        buf << if foo\n          bar\n        end\n";
        let diags = run_cop_full_with_config(&EndAlignment, src, config);
        assert!(diags.is_empty(), "variable style should accept end at line start for << operator: {:?}", diags);
    }

    #[test]
    fn variable_style_shovel_case_aligns_with_line_start() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyleAlignWith".into(), serde_yml::Value::String("variable".into())),
            ]),
            ..CopConfig::default()
        };
        // `memo << case key` — end aligns with line indent (col 4)
        let src = b"    memo << case key\n              when :a\n                1\n    end\n";
        let diags = run_cop_full_with_config(&EndAlignment, src, config);
        assert!(diags.is_empty(), "variable style should accept end at line start for << case: {:?}", diags);
    }

    #[test]
    fn variable_style_no_assignment_flags_misaligned() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyleAlignWith".into(), serde_yml::Value::String("variable".into())),
            ]),
            ..CopConfig::default()
        };
        // `super || if ...` — end NOT aligned with if (should flag)
        let src = b"  def foo\n    super || if true\n                   1\n  end\n  end\n";
        let diags = run_cop_full_with_config(&EndAlignment, src, config);
        assert_eq!(diags.len(), 1, "variable style should flag end not aligned with keyword when no assignment");
    }
}
