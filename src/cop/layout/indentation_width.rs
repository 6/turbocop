use crate::cop::util::expected_indent_for_body;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct IndentationWidth;

impl IndentationWidth {
    fn check_body_indentation(
        &self,
        source: &SourceFile,
        keyword_offset: usize,
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

        let (kw_line, kw_col) = source.offset_to_line_col(keyword_offset);
        let expected = expected_indent_for_body(kw_col, width);

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
                        width, child_col.saturating_sub(kw_col)
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

        if let Some(class_node) = node.as_class_node() {
            return self.check_body_indentation(
                source,
                class_node.class_keyword_loc().start_offset(),
                class_node.body(),
                width,
            );
        }

        if let Some(module_node) = node.as_module_node() {
            return self.check_body_indentation(
                source,
                module_node.module_keyword_loc().start_offset(),
                module_node.body(),
                width,
            );
        }

        if let Some(def_node) = node.as_def_node() {
            return self.check_body_indentation(
                source,
                def_node.def_keyword_loc().start_offset(),
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
            return self.check_body_indentation(
                source,
                block_node.opening_loc().start_offset(),
                block_node.body(),
                width,
            );
        }

        if let Some(case_node) = node.as_case_node() {
            // Check that when clauses are indented from the case keyword
            let kw_offset = case_node.case_keyword_loc().start_offset();
            let (kw_line, kw_col) = source.offset_to_line_col(kw_offset);
            let expected = expected_indent_for_body(kw_col, width);

            for condition in case_node.conditions().iter() {
                let loc = condition.location();
                let (cond_line, cond_col) = source.offset_to_line_col(loc.start_offset());

                if cond_line == kw_line {
                    return Vec::new();
                }

                if cond_col != expected {
                    return vec![self.diagnostic(
                        source,
                        cond_line,
                        cond_col,
                        format!(
                            "Use {} (not {}) spaces for indentation.",
                            width, cond_col.saturating_sub(kw_col)
                        ),
                    )];
                }
            }
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
}
