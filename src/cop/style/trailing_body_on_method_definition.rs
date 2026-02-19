use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BEGIN_NODE, DEF_NODE};

pub struct TrailingBodyOnMethodDefinition;

impl Cop for TrailingBodyOnMethodDefinition {
    fn name(&self) -> &'static str {
        "Style/TrailingBodyOnMethodDefinition"
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
    ) -> Vec<Diagnostic> {
        if let Some(def_node) = node.as_def_node() {
            // Skip endless methods (def foo = ...)
            if def_node.equal_loc().is_some() {
                return Vec::new();
            }

            let body = match def_node.body() {
                Some(b) => b,
                None => return Vec::new(),
            };

            // Method must be multiline (def on different line than end)
            let def_loc = def_node.def_keyword_loc();
            let (def_line, _) = source.offset_to_line_col(def_loc.start_offset());
            let end_loc = match def_node.end_keyword_loc() {
                Some(loc) => loc,
                None => return Vec::new(),
            };
            let (end_line, _) = source.offset_to_line_col(end_loc.start_offset());
            if def_line == end_line {
                return Vec::new();
            }

            // When body is a BeginNode (implicit begin wrapping rescue/ensure),
            // look at the first statement inside, not the BeginNode itself
            // (whose location may start on the def line in Prism).
            let (body_line, body_column) = if let Some(begin_node) = body.as_begin_node() {
                if let Some(stmts) = begin_node.statements() {
                    let stmts_body = stmts.body();
                    if let Some(first_stmt) = stmts_body.iter().next() {
                        let loc = first_stmt.location();
                        source.offset_to_line_col(loc.start_offset())
                    } else {
                        // No statements in begin body — check rescue clause
                        if let Some(rescue) = begin_node.rescue_clause() {
                            let loc = rescue.location();
                            source.offset_to_line_col(loc.start_offset())
                        } else {
                            return Vec::new();
                        }
                    }
                } else {
                    // No statements — check rescue clause location
                    if let Some(rescue) = begin_node.rescue_clause() {
                        let loc = rescue.location();
                        source.offset_to_line_col(loc.start_offset())
                    } else {
                        return Vec::new();
                    }
                }
            } else {
                let body_loc = body.location();
                source.offset_to_line_col(body_loc.start_offset())
            };

            if def_line == body_line {
                return vec![self.diagnostic(
                    source,
                    body_line,
                    body_column,
                    "Place the first line of a multi-line method definition's body on its own line."
                        .to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TrailingBodyOnMethodDefinition, "cops/style/trailing_body_on_method_definition");
}
