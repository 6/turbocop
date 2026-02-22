use crate::cop::node_type::WHEN_NODE;
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[WHEN_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let when_node = match node.as_when_node() {
            Some(n) => n,
            None => return,
        };

        let body_empty = match when_node.statements() {
            None => true,
            Some(stmts) => stmts.body().is_empty(),
        };

        if !body_empty {
            return;
        }

        // AllowComments: when true, `when` bodies containing only comments are not offenses
        let allow_comments = config.get_bool("AllowComments", true);
        if allow_comments {
            let kw_loc = when_node.keyword_loc();
            let (when_line, _) = source.offset_to_line_col(kw_loc.start_offset());
            // Scan lines after the when clause looking for comments.
            // The when_node location may end at the conditions (before any comment),
            // so we scan forward from the when line until we hit the next keyword
            // (when/else/end) or non-blank/non-comment content.
            let lines: Vec<&[u8]> = source.lines().collect();
            for line_idx in when_line..lines.len() {
                // line_idx is 0-based (when_line is 1-based, so when_line as index = line after when)
                if let Some(line_bytes) = lines.get(line_idx) {
                    let trimmed = line_bytes
                        .iter()
                        .position(|&b| b != b' ' && b != b'\t')
                        .map(|start| &line_bytes[start..])
                        .unwrap_or(&[]);
                    if trimmed.is_empty() {
                        continue;
                    }
                    // Stop at the next when/else/end keyword
                    if trimmed.starts_with(b"when ")
                        || trimmed.starts_with(b"when\n")
                        || trimmed.starts_with(b"else")
                        || trimmed.starts_with(b"end")
                    {
                        break;
                    }
                    if trimmed.starts_with(b"#") {
                        return;
                    }
                    // Non-comment content found — shouldn't happen for empty when
                    break;
                }
            }
        }

        let kw_loc = when_node.keyword_loc();
        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Avoid empty `when` conditions.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyWhen, "cops/lint/empty_when");
}
