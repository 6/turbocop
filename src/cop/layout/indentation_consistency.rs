use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct IndentationConsistency;

impl IndentationConsistency {
    fn check_body_consistency(
        &self,
        source: &SourceFile,
        keyword_offset: usize,
        body: Option<ruby_prism::Node<'_>>,
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
        if children.len() < 2 {
            return Vec::new();
        }

        let (kw_line, _) = source.offset_to_line_col(keyword_offset);

        // Get first statement's column as the reference
        let first_loc = children[0].location();
        let (first_line, first_col) = source.offset_to_line_col(first_loc.start_offset());

        // Skip single-line bodies
        if first_line == kw_line {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        for child in &children[1..] {
            let loc = child.location();
            let (child_line, child_col) = source.offset_to_line_col(loc.start_offset());

            // Skip if on same line as first (shouldn't happen normally)
            if child_line == first_line {
                continue;
            }

            if child_col != first_col {
                diagnostics.push(Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location {
                        line: child_line,
                        column: child_col,
                    },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Inconsistent indentation detected.".to_string(),
                });
            }
        }

        diagnostics
    }
}

impl Cop for IndentationConsistency {
    fn name(&self) -> &'static str {
        "Layout/IndentationConsistency"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        if let Some(class_node) = node.as_class_node() {
            return self.check_body_consistency(
                source,
                class_node.class_keyword_loc().start_offset(),
                class_node.body(),
            );
        }

        if let Some(module_node) = node.as_module_node() {
            return self.check_body_consistency(
                source,
                module_node.module_keyword_loc().start_offset(),
                module_node.body(),
            );
        }

        if let Some(def_node) = node.as_def_node() {
            return self.check_body_consistency(
                source,
                def_node.def_keyword_loc().start_offset(),
                def_node.body(),
            );
        }

        if let Some(block_node) = node.as_block_node() {
            return self.check_body_consistency(
                source,
                block_node.opening_loc().start_offset(),
                block_node.body(),
            );
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full, run_cop_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &IndentationConsistency,
            include_bytes!("../../../testdata/cops/layout/indentation_consistency/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &IndentationConsistency,
            include_bytes!("../../../testdata/cops/layout/indentation_consistency/no_offense.rb"),
        );
    }

    #[test]
    fn single_statement_body() {
        let source = b"def foo\n  x = 1\nend\n";
        let diags = run_cop_full(&IndentationConsistency, source);
        assert!(diags.is_empty());
    }
}
