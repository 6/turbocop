use crate::cop::shared::node_type::MODULE_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus FP fix: When a module body contains rescue/ensure, Prism wraps the
/// body in a BeginNode whose location starts on the module keyword line (since
/// the begin is implicit). We drill into the first actual statement inside the
/// BeginNode to get the real body start line. Same root cause and fix as
/// TrailingBodyOnClass.
pub struct TrailingBodyOnModule;

/// Get the line and column of the first actual statement in a body node.
/// For BeginNode (implicit begin wrapping rescue/ensure), the node's own
/// location may start on the module keyword line, so we drill into
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
        if let Some(rescue) = begin_node.rescue_clause() {
            let loc = rescue.location();
            return source.offset_to_line_col(loc.start_offset());
        }
        if let Some(ensure) = begin_node.ensure_clause() {
            let loc = ensure.ensure_keyword_loc();
            return source.offset_to_line_col(loc.start_offset());
        }
    }
    let loc = body.location();
    source.offset_to_line_col(loc.start_offset())
}

impl Cop for TrailingBodyOnModule {
    fn name(&self) -> &'static str {
        "Style/TrailingBodyOnModule"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[MODULE_NODE]
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
        let module_node = match node.as_module_node() {
            Some(m) => m,
            None => return,
        };

        let body = match module_node.body() {
            Some(b) => b,
            None => return,
        };

        let mod_loc = module_node.constant_path().location();
        let (mod_line, _) = source.offset_to_line_col(mod_loc.start_offset());
        let (body_line, body_column) = first_body_line_col(source, &body);

        // Only flag multiline modules (RuboCop's `multiline?` check).
        // Single-line modules like `module Foo; def bar; end; end` are fine.
        let end_loc = module_node.end_keyword_loc();
        let (end_line, _) = source.offset_to_line_col(end_loc.start_offset());
        if end_line == mod_line {
            return;
        }

        if mod_line == body_line {
            diagnostics.push(self.diagnostic(
                source,
                body_line,
                body_column,
                "Place the first line of module body on its own line.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TrailingBodyOnModule, "cops/style/trailing_body_on_module");
}
