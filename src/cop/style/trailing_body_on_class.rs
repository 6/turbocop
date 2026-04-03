use crate::cop::shared::node_type::{CLASS_NODE, SINGLETON_CLASS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus FP fix: When a class/module body contains rescue/ensure, Prism wraps
/// the body in a BeginNode whose location starts on the class keyword line
/// (since the begin is implicit). We must drill into the first actual statement
/// inside the BeginNode to get the real body start line, matching the pattern
/// used in TrailingBodyOnMethodDefinition.
pub struct TrailingBodyOnClass;

/// Get the line and column of the first actual statement in a body node.
/// For BeginNode (implicit begin wrapping rescue/ensure), the node's own
/// location may start on the class/module keyword line, so we drill into
/// the statements to find the real first line.
fn first_body_line_col(source: &SourceFile, body: &ruby_prism::Node<'_>) -> (usize, usize) {
    if let Some(begin_node) = body.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            let stmts_body = stmts.body();
            if let Some(first_stmt) = stmts_body.iter().next() {
                let loc = first_stmt.location();
                return source.offset_to_line_col(loc.start_offset());
            }
        }
        // No statements — check rescue clause location
        if let Some(rescue) = begin_node.rescue_clause() {
            let loc = rescue.location();
            return source.offset_to_line_col(loc.start_offset());
        }
        // ensure with no statements or rescue
        if let Some(ensure) = begin_node.ensure_clause() {
            let loc = ensure.ensure_keyword_loc();
            return source.offset_to_line_col(loc.start_offset());
        }
    }
    let loc = body.location();
    source.offset_to_line_col(loc.start_offset())
}

impl Cop for TrailingBodyOnClass {
    fn name(&self) -> &'static str {
        "Style/TrailingBodyOnClass"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE, SINGLETON_CLASS_NODE]
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
        // Check class ... ; body
        if let Some(class_node) = node.as_class_node() {
            let body = match class_node.body() {
                Some(b) => b,
                None => return,
            };

            let class_loc = class_node.constant_path().location();
            let (class_line, _) = source.offset_to_line_col(class_loc.start_offset());
            let (body_line, body_column) = first_body_line_col(source, &body);

            if class_line == body_line {
                // Single-line class definition (end on same line as class) — not an offense
                let end_loc = class_node.end_keyword_loc();
                let (end_line, _) = source.offset_to_line_col(end_loc.start_offset());
                if class_line == end_line {
                    return;
                }

                diagnostics.push(self.diagnostic(
                    source,
                    body_line,
                    body_column,
                    "Place the first line of class body on its own line.".to_string(),
                ));
            }
        }

        // Check sclass (singleton class) `class << self; body`
        if let Some(sclass_node) = node.as_singleton_class_node() {
            let body = match sclass_node.body() {
                Some(b) => b,
                None => return,
            };

            let kw_loc = sclass_node.class_keyword_loc();
            let (kw_line, _) = source.offset_to_line_col(kw_loc.start_offset());
            let (body_line, body_column) = first_body_line_col(source, &body);

            if kw_line == body_line {
                // Single-line singleton class (end on same line as class) — not an offense
                let end_loc = sclass_node.end_keyword_loc();
                let (end_line, _) = source.offset_to_line_col(end_loc.start_offset());
                if kw_line == end_line {
                    return;
                }

                diagnostics.push(self.diagnostic(
                    source,
                    body_line,
                    body_column,
                    "Place the first line of class body on its own line.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TrailingBodyOnClass, "cops/style/trailing_body_on_class");
}
