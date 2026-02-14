use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct Loop;

impl Cop for Loop {
    fn name(&self) -> &'static str {
        "Lint/Loop"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check WhileNode for begin..end while form
        if let Some(while_node) = node.as_while_node() {
            let kw_loc = while_node.keyword_loc();
            // In a post-condition loop (begin..end while), the keyword comes
            // after the body. Check if statements exist and start before keyword.
            if let Some(stmts) = while_node.statements() {
                let body = stmts.body();
                if let Some(first_stmt) = body.first() {
                    let first_stmt_offset = first_stmt.location().start_offset();
                    if kw_loc.start_offset() > first_stmt_offset {
                        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                        return vec![Diagnostic {
                            path: source.path_str().to_string(),
                            location: Location { line, column },
                            severity: self.default_severity(),
                            cop_name: self.name().to_string(),
                            message: "Use `Kernel#loop` with `break` rather than `begin/end/while(until)`.".to_string(),
                        }];
                    }
                }
            }
        }

        // Check UntilNode for begin..end until form
        if let Some(until_node) = node.as_until_node() {
            let kw_loc = until_node.keyword_loc();
            if let Some(stmts) = until_node.statements() {
                let body = stmts.body();
                if let Some(first_stmt) = body.first() {
                    let first_stmt_offset = first_stmt.location().start_offset();
                    if kw_loc.start_offset() > first_stmt_offset {
                        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                        return vec![Diagnostic {
                            path: source.path_str().to_string(),
                            location: Location { line, column },
                            severity: self.default_severity(),
                            cop_name: self.name().to_string(),
                            message: "Use `Kernel#loop` with `break` rather than `begin/end/while(until)`.".to_string(),
                        }];
                    }
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &Loop,
            include_bytes!("../../../testdata/cops/lint/loop_cop/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &Loop,
            include_bytes!("../../../testdata/cops/lint/loop_cop/no_offense.rb"),
        );
    }
}
