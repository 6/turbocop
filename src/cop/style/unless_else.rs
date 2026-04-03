use crate::cop::shared::node_type::UNLESS_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct UnlessElse;

impl Cop for UnlessElse {
    fn name(&self) -> &'static str {
        "Style/UnlessElse"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[UNLESS_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let unless_node = match node.as_unless_node() {
            Some(n) => n,
            None => return,
        };

        // Must have an else clause
        let else_clause = match unless_node.else_clause() {
            Some(e) => e,
            None => return,
        };

        let kw_loc = unless_node.keyword_loc();
        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            "Do not use `unless` with `else`. Rewrite these with the positive case first."
                .to_string(),
        );

        // Autocorrect: rewrite `unless cond\n  body1\nelse\n  body2\nend`
        //           to `if cond\n  body2\nelse\n  body1\nend`
        if let Some(ref mut corr) = corrections {
            let src = source.as_bytes();

            // 1. Replace `unless` with `if`
            corr.push(crate::correction::Correction {
                start: kw_loc.start_offset(),
                end: kw_loc.end_offset(),
                replacement: "if".to_string(),
                cop_name: self.name(),
                cop_index: 0,
            });

            // 2. Swap the bodies: unless body becomes else body and vice versa
            // The unless body is between end of condition line and `else` keyword
            // The else body is between `else` keyword line end and `end` keyword
            if let Some(unless_stmts) = unless_node.statements() {
                let else_kw_loc = else_clause.else_keyword_loc();

                // Unless body: from unless_stmts start to else keyword line start
                let unless_body_start = unless_stmts.location().start_offset();
                let mut unless_body_end = else_kw_loc.start_offset();
                // Walk back to trim trailing whitespace/newline before else
                while unless_body_end > unless_body_start
                    && (src[unless_body_end - 1] == b' '
                        || src[unless_body_end - 1] == b'\t'
                        || src[unless_body_end - 1] == b'\n'
                        || src[unless_body_end - 1] == b'\r')
                {
                    unless_body_end -= 1;
                }

                // Else body
                let else_body_start;
                let mut else_body_end;
                if let Some(else_stmts) = else_clause.statements() {
                    else_body_start = else_stmts.location().start_offset();
                    else_body_end = else_stmts.location().end_offset();
                    // Trim trailing whitespace
                    while else_body_end > else_body_start
                        && (src[else_body_end - 1] == b' '
                            || src[else_body_end - 1] == b'\t'
                            || src[else_body_end - 1] == b'\n'
                            || src[else_body_end - 1] == b'\r')
                    {
                        else_body_end -= 1;
                    }
                } else {
                    // Empty else body — shouldn't happen for UnlessElse but handle gracefully
                    else_body_start = else_kw_loc.end_offset();
                    else_body_end = else_body_start;
                }

                let unless_body =
                    String::from_utf8_lossy(&src[unless_body_start..unless_body_end]).to_string();
                let else_body =
                    String::from_utf8_lossy(&src[else_body_start..else_body_end]).to_string();

                // Replace unless body with else body
                corr.push(crate::correction::Correction {
                    start: unless_body_start,
                    end: unless_body_end,
                    replacement: else_body,
                    cop_name: self.name(),
                    cop_index: 0,
                });

                // Replace else body with unless body
                corr.push(crate::correction::Correction {
                    start: else_body_start,
                    end: else_body_end,
                    replacement: unless_body,
                    cop_name: self.name(),
                    cop_index: 0,
                });
            }

            diag.corrected = true;
        }

        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnlessElse, "cops/style/unless_else");
    crate::cop_autocorrect_fixture_tests!(UnlessElse, "cops/style/unless_else");
}
