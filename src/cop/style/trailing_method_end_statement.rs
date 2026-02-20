use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BEGIN_NODE, DEF_NODE};

pub struct TrailingMethodEndStatement;

/// Find the last content line of a body node. For BeginNode (implicit begin
/// wrapping rescue/ensure), the node's own end_offset includes the `end`
/// keyword, so we drill into the last clause to get the real content end.
fn body_last_line(source: &SourceFile, body: &ruby_prism::Node<'_>) -> usize {
    if let Some(begin_node) = body.as_begin_node() {
        // Try ensure clause first (it comes last)
        if let Some(ensure) = begin_node.ensure_clause() {
            if let Some(stmts) = ensure.statements() {
                let loc = stmts.location();
                let (line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
                return line;
            }
            // ensure with no body — use ensure keyword line
            let loc = ensure.ensure_keyword_loc();
            let (line, _) = source.offset_to_line_col(loc.start_offset());
            return line;
        }

        // Try else clause (between rescue and ensure).
        // ElseNode location may include the closing `end`, so use its statements.
        if let Some(else_clause) = begin_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                let loc = stmts.location();
                let (line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
                return line;
            }
            // else with no body — use else keyword line
            let loc = else_clause.else_keyword_loc();
            let (line, _) = source.offset_to_line_col(loc.start_offset());
            return line;
        }

        // Try last rescue clause
        if let Some(rescue) = begin_node.rescue_clause() {
            // Walk to the last rescue clause
            let mut current = rescue;
            while let Some(next) = current.subsequent() {
                current = next;
            }
            // Use the rescue clause's statements end, not the clause location
            // (which may extend to the next clause boundary)
            if let Some(stmts) = current.statements() {
                let loc = stmts.location();
                let (line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
                return line;
            }
            // Rescue with no body — use rescue keyword line
            let loc = current.keyword_loc();
            let (line, _) = source.offset_to_line_col(loc.start_offset());
            return line;
        }

        // Fall back to statements
        if let Some(stmts) = begin_node.statements() {
            let loc = stmts.location();
            let (line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
            return line;
        }
    }

    // Non-BeginNode: use body's own end offset
    let body_loc = body.location();
    let (line, _) = source.offset_to_line_col(body_loc.end_offset().saturating_sub(1));
    line
}

impl Cop for TrailingMethodEndStatement {
    fn name(&self) -> &'static str {
        "Style/TrailingMethodEndStatement"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE, DEF_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Skip endless methods (def foo = ...)
        if def_node.equal_loc().is_some() {
            return;
        }

        // Must have a body
        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };

        // Must be multiline
        let def_loc = def_node.location();
        let (def_start_line, _) = source.offset_to_line_col(def_loc.start_offset());
        let end_loc = match def_node.end_keyword_loc() {
            Some(loc) => loc,
            None => return,
        };
        let (end_line, end_column) = source.offset_to_line_col(end_loc.start_offset());

        if def_start_line == end_line {
            return;
        }

        // Check if body's last content line == end line
        let last_line = body_last_line(source, &body);

        if last_line == end_line {
            diagnostics.push(self.diagnostic(
                source,
                end_line,
                end_column,
                "Place the end statement of a multi-line method on its own line.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TrailingMethodEndStatement, "cops/style/trailing_method_end_statement");
}
