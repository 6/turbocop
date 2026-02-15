use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SuppressedException;

impl Cop for SuppressedException {
    fn name(&self) -> &'static str {
        "Lint/SuppressedException"
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
        // RescueNode is visited via visit_begin_node's specific method,
        // not via the generic visit() dispatch. So we match BeginNode
        // and check its rescue_clause.
        let begin_node = match node.as_begin_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let rescue_node = match begin_node.rescue_clause() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // AllowNil: when true, allow `rescue => e; nil; end`
        let allow_nil = config.get_bool("AllowNil", false);

        let body_stmts = rescue_node.statements();
        let body_empty = match &body_stmts {
            None => true,
            Some(stmts) => stmts.body().is_empty(),
        };

        if !body_empty {
            // If AllowNil and body is a single `nil` literal, allow it
            if allow_nil {
                if let Some(stmts) = &body_stmts {
                    let body_nodes: Vec<_> = stmts.body().iter().collect();
                    if body_nodes.len() == 1 && body_nodes[0].as_nil_node().is_some() {
                        return Vec::new();
                    }
                }
            }
            return Vec::new();
        }

        // AllowComments: if true (default), skip rescue bodies that contain only comments
        let allow_comments = config.get_bool("AllowComments", true);
        if allow_comments {
            // Find the line range between rescue keyword and the next keyword (end/else/ensure/rescue)
            let (rescue_line, _) = source.offset_to_line_col(rescue_node.keyword_loc().start_offset());
            let clause_end_line = if let Some(sub) = rescue_node.subsequent() {
                source.offset_to_line_col(sub.keyword_loc().start_offset()).0
            } else if let Some(else_clause) = begin_node.else_clause() {
                source.offset_to_line_col(else_clause.location().start_offset()).0
            } else if let Some(ensure_clause) = begin_node.ensure_clause() {
                source.offset_to_line_col(ensure_clause.location().start_offset()).0
            } else if let Some(end_loc) = begin_node.end_keyword_loc() {
                source.offset_to_line_col(end_loc.start_offset()).0
            } else {
                rescue_line + 1 // no end keyword found, can't check
            };

            let lines: Vec<&[u8]> = source.lines().collect();
            // Check lines between rescue keyword and end of clause (1-indexed)
            for line_num in (rescue_line + 1)..clause_end_line {
                if let Some(line) = lines.get(line_num - 1) {
                    let trimmed = line.iter()
                        .position(|&b| b != b' ' && b != b'\t')
                        .map(|start| &line[start..])
                        .unwrap_or(&[]);
                    if trimmed.starts_with(b"#") {
                        return Vec::new(); // Has a comment, allow it
                    }
                }
            }
        }

        let kw_loc = rescue_node.keyword_loc();
        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Do not suppress exceptions.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SuppressedException, "cops/lint/suppressed_exception");
}
