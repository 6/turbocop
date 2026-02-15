use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyWhen;

impl Cop for EmptyWhen {
    fn name(&self) -> &'static str {
        "Lint/EmptyWhen"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let when_node = match node.as_when_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let body_empty = match when_node.statements() {
            None => true,
            Some(stmts) => stmts.body().is_empty(),
        };

        if !body_empty {
            return Vec::new();
        }

        // AllowComments: when true, `when` bodies containing only comments are not offenses
        let allow_comments = config.get_bool("AllowComments", false);
        if allow_comments {
            let kw_loc = when_node.keyword_loc();
            let (when_line, _) = source.offset_to_line_col(kw_loc.start_offset());
            // Find the end of this when clause by looking at the then_keyword or the next node
            let when_end_offset = when_node.location().end_offset();
            let (when_end_line, _) = source.offset_to_line_col(when_end_offset);
            let lines: Vec<&[u8]> = source.lines().collect();
            for line_num in (when_line + 1)..when_end_line {
                if let Some(line_bytes) = lines.get(line_num - 1) {
                    let trimmed = line_bytes
                        .iter()
                        .position(|&b| b != b' ' && b != b'\t')
                        .map(|start| &line_bytes[start..])
                        .unwrap_or(&[]);
                    if trimmed.starts_with(b"#") {
                        return Vec::new();
                    }
                }
            }
        }

        let kw_loc = when_node.keyword_loc();
        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Avoid empty `when` conditions.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyWhen, "cops/lint/empty_when");
}
