use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EndAlignment;

impl Cop for EndAlignment {
    fn name(&self) -> &'static str {
        "Layout/EndAlignment"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyleAlignWith", "keyword");
        if let Some(class_node) = node.as_class_node() {
            return self.check_keyword_end(
                source,
                class_node.class_keyword_loc().start_offset(),
                class_node.end_keyword_loc().start_offset(),
                "class",
                style,
            );
        }

        if let Some(module_node) = node.as_module_node() {
            return self.check_keyword_end(
                source,
                module_node.module_keyword_loc().start_offset(),
                module_node.end_keyword_loc().start_offset(),
                "module",
                style,
            );
        }

        if let Some(if_node) = node.as_if_node() {
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return Vec::new(),
            };
            // Only check top-level if/unless, not elsif
            let kw_slice = kw_loc.as_slice();
            if kw_slice != b"if" && kw_slice != b"unless" {
                return Vec::new();
            }
            let end_kw_loc = match if_node.end_keyword_loc() {
                Some(loc) => loc,
                None => return Vec::new(),
            };
            let keyword = if kw_slice == b"if" { "if" } else { "unless" };
            return self.check_keyword_end(
                source,
                kw_loc.start_offset(),
                end_kw_loc.start_offset(),
                keyword,
                style,
            );
        }

        if let Some(while_node) = node.as_while_node() {
            let kw_loc = while_node.keyword_loc();
            if let Some(end_loc) = while_node.closing_loc() {
                return self.check_keyword_end(
                    source,
                    kw_loc.start_offset(),
                    end_loc.start_offset(),
                    "while",
                    style,
                );
            }
        }

        if let Some(until_node) = node.as_until_node() {
            let kw_loc = until_node.keyword_loc();
            if let Some(end_loc) = until_node.closing_loc() {
                return self.check_keyword_end(
                    source,
                    kw_loc.start_offset(),
                    end_loc.start_offset(),
                    "until",
                    style,
                );
            }
        }

        if let Some(case_node) = node.as_case_node() {
            let kw_loc = case_node.case_keyword_loc();
            let end_loc = case_node.end_keyword_loc();
            return self.check_keyword_end(
                source,
                kw_loc.start_offset(),
                end_loc.start_offset(),
                "case",
                style,
            );
        }

        // NOTE: `begin` blocks are not checked here — that's handled by
        // Layout/BeginEndAlignment which supports variable-aligned `end`.

        Vec::new()
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
                // For variable alignment: align with start of the assignment line
                // Walk back from keyword to find start of line
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
}
