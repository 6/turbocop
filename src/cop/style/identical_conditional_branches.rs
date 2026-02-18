use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct IdenticalConditionalBranches;

impl IdenticalConditionalBranches {
    fn last_stmt_source(source: &SourceFile, stmts: &ruby_prism::StatementsNode<'_>) -> Option<(String, usize, usize)> {
        let body: Vec<_> = stmts.body().iter().collect();
        if body.is_empty() {
            return None;
        }
        let last = &body[body.len() - 1];
        let loc = last.location();
        let src = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
        let (line, col) = source.offset_to_line_col(loc.start_offset());
        Some((String::from_utf8_lossy(src).trim().to_string(), line, col))
    }
}

impl Cop for IdenticalConditionalBranches {
    fn name(&self) -> &'static str {
        "Style/IdenticalConditionalBranches"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        if let Some(if_node) = node.as_if_node() {
            // Must have an if keyword (skip ternaries)
            if if_node.if_keyword_loc().is_none() {
                return Vec::new();
            }

            let if_stmts = match if_node.statements() {
                Some(s) => s,
                None => return Vec::new(),
            };

            let else_clause = match if_node.subsequent() {
                Some(e) => e,
                None => return Vec::new(),
            };

            // Must be a direct else, not an elsif
            let else_stmts = if let Some(ec) = else_clause.as_else_node() {
                match ec.statements() {
                    Some(s) => s,
                    None => return Vec::new(),
                }
            } else {
                return Vec::new();
            };

            if let (Some((if_last, if_line, if_col)), Some((else_last, _, _))) = (
                Self::last_stmt_source(source, &if_stmts),
                Self::last_stmt_source(source, &else_stmts),
            ) {
                if if_last == else_last && !if_last.is_empty() {
                    return vec![self.diagnostic(
                        source,
                        if_line,
                        if_col,
                        format!("Move `{}` out of the conditional.", if_last),
                    )];
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IdenticalConditionalBranches, "cops/style/identical_conditional_branches");
}
