use crate::cop::util::expected_indent_for_body;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct IndentationWidth;

impl IndentationWidth {
    /// Check body indentation.
    /// `keyword_offset` is used to determine which line the keyword is on (for same-line skip).
    /// `base_col` is the column that expected indentation is relative to.
    fn check_body_indentation(
        &self,
        source: &SourceFile,
        keyword_offset: usize,
        base_col: usize,
        body: Option<ruby_prism::Node<'_>>,
        width: usize,
    ) -> Vec<Diagnostic> {
        let body = match body {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let children: Vec<_> = stmts.body().iter().collect();
        if children.is_empty() {
            return Vec::new();
        }

        let (kw_line, _) = source.offset_to_line_col(keyword_offset);
        let expected = expected_indent_for_body(base_col, width);

        for child in &children {
            let loc = child.location();
            let (child_line, child_col) = source.offset_to_line_col(loc.start_offset());

            // Skip if body is on same line as keyword (single-line construct)
            if child_line == kw_line {
                return Vec::new();
            }

            if child_col != expected {
                return vec![self.diagnostic(
                    source,
                    child_line,
                    child_col,
                    format!(
                        "Use {} (not {}) spaces for indentation.",
                        width, child_col.saturating_sub(base_col)
                    ),
                )];
            }
        }

        Vec::new()
    }

    fn check_statements_indentation(
        &self,
        source: &SourceFile,
        keyword_offset: usize,
        stmts: Option<ruby_prism::StatementsNode<'_>>,
        width: usize,
    ) -> Vec<Diagnostic> {
        let stmts = match stmts {
            Some(s) => s,
            None => return Vec::new(),
        };

        let children: Vec<_> = stmts.body().iter().collect();
        if children.is_empty() {
            return Vec::new();
        }

        let (kw_line, kw_col) = source.offset_to_line_col(keyword_offset);
        let expected = expected_indent_for_body(kw_col, width);

        for child in &children {
            let loc = child.location();
            let (child_line, child_col) = source.offset_to_line_col(loc.start_offset());

            if child_line == kw_line {
                return Vec::new();
            }

            if child_col != expected {
                return vec![self.diagnostic(
                    source,
                    child_line,
                    child_col,
                    format!(
                        "Use {} (not {}) spaces for indentation.",
                        width, child_col.saturating_sub(kw_col)
                    ),
                )];
            }
        }

        Vec::new()
    }
}

impl Cop for IndentationWidth {
    fn name(&self) -> &'static str {
        "Layout/IndentationWidth"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let width = config.get_usize("Width", 2);
        let align_style = config.get_str("EnforcedStyleAlignWith", "start_of_line");
        let allowed_patterns = config.get_string_array("AllowedPatterns").unwrap_or_default();

        // Skip if the node's source line matches any allowed pattern
        if !allowed_patterns.is_empty() {
            let (node_line, _) = source.offset_to_line_col(node.location().start_offset());
            if let Some(line_bytes) = source.lines().nth(node_line - 1) {
                if let Ok(line_str) = std::str::from_utf8(line_bytes) {
                    for pattern in &allowed_patterns {
                        if let Ok(re) = regex::Regex::new(pattern) {
                            if re.is_match(line_str) {
                                return Vec::new();
                            }
                        }
                    }
                }
            }
        }

        if let Some(class_node) = node.as_class_node() {
            let kw_offset = class_node.class_keyword_loc().start_offset();
            let (_, kw_col) = source.offset_to_line_col(kw_offset);
            return self.check_body_indentation(
                source,
                kw_offset,
                kw_col,
                class_node.body(),
                width,
            );
        }

        if let Some(module_node) = node.as_module_node() {
            let kw_offset = module_node.module_keyword_loc().start_offset();
            let (_, kw_col) = source.offset_to_line_col(kw_offset);
            return self.check_body_indentation(
                source,
                kw_offset,
                kw_col,
                module_node.body(),
                width,
            );
        }

        if let Some(def_node) = node.as_def_node() {
            let kw_offset = def_node.def_keyword_loc().start_offset();
            let base_col = if align_style == "keyword" {
                // EnforcedStyleAlignWith: keyword — indent relative to `def` keyword column
                source.offset_to_line_col(kw_offset).1
            } else {
                // EnforcedStyleAlignWith: start_of_line (default) — indent relative to the
                // start of the line, using `end` keyword column as proxy for line-start indent.
                if let Some(end_loc) = def_node.end_keyword_loc() {
                    source.offset_to_line_col(end_loc.start_offset()).1
                } else {
                    source.offset_to_line_col(kw_offset).1
                }
            };
            return self.check_body_indentation(
                source,
                kw_offset,
                base_col,
                def_node.body(),
                width,
            );
        }

        if let Some(if_node) = node.as_if_node() {
            if let Some(kw_loc) = if_node.if_keyword_loc() {
                return self.check_statements_indentation(
                    source,
                    kw_loc.start_offset(),
                    if_node.statements(),
                    width,
                );
            }
        }

        if let Some(block_node) = node.as_block_node() {
            let opening_offset = block_node.opening_loc().start_offset();
            // For blocks, indentation is relative to the `end`/`}` keyword
            // column, which aligns with the start of the expression (handles
            // multiline chained calls where `do` is on a continuation line).
            let closing_offset = block_node.closing_loc().start_offset();
            let (_, base_col) = source.offset_to_line_col(closing_offset);
            return self.check_body_indentation(
                source,
                opening_offset,
                base_col,
                block_node.body(),
                width,
            );
        }

        // Check body indentation inside when clauses (when keyword
        // positioning is handled by Layout/CaseIndentation, not here).
        if let Some(when_node) = node.as_when_node() {
            let kw_offset = when_node.keyword_loc().start_offset();
            return self.check_statements_indentation(
                source,
                kw_offset,
                when_node.statements(),
                width,
            );
        }

        if let Some(while_node) = node.as_while_node() {
            return self.check_statements_indentation(
                source,
                while_node.keyword_loc().start_offset(),
                while_node.statements(),
                width,
            );
        }

        if let Some(until_node) = node.as_until_node() {
            return self.check_statements_indentation(
                source,
                until_node.keyword_loc().start_offset(),
                until_node.statements(),
                width,
            );
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full_with_config;

    crate::cop_fixture_tests!(IndentationWidth, "cops/layout/indentation_width");

    #[test]
    fn custom_width() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([("Width".into(), serde_yml::Value::Number(4.into()))]),
            ..CopConfig::default()
        };
        // Body indented 2 instead of 4
        let source = b"def foo\n  x = 1\nend\n";
        let diags = run_cop_full_with_config(&IndentationWidth, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Use 4 (not 2) spaces"));
    }

    #[test]
    fn enforced_style_keyword_aligns_to_def() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyleAlignWith".into(), serde_yml::Value::String("keyword".into())),
            ]),
            ..CopConfig::default()
        };
        // Body indented 2 from column 0, but `def` is at column 8 (after `private `)
        // With keyword style, body should be at column 10 (8 + 2)
        let source = b"private def foo\n  bar\nend\n";
        let diags = run_cop_full_with_config(&IndentationWidth, source, config);
        assert_eq!(diags.len(), 1, "keyword style should flag body not aligned with def keyword");
        assert!(diags[0].message.contains("Use 2"));
    }

    #[test]
    fn allowed_patterns_skips_matching() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("AllowedPatterns".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("^\\s*module".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        // Module with wrong indentation but matches AllowedPatterns
        let source = b"module Foo\n      x = 1\nend\n";
        let diags = run_cop_full_with_config(&IndentationWidth, source, config);
        assert!(diags.is_empty(), "AllowedPatterns should skip matching lines");
    }
}
