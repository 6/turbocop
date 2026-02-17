use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TrailingBodyOnMethodDefinition;

impl Cop for TrailingBodyOnMethodDefinition {
    fn name(&self) -> &'static str {
        "Style/TrailingBodyOnMethodDefinition"
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

            let def_loc = def_node.def_keyword_loc();
            let (def_line, _) = source.offset_to_line_col(def_loc.start_offset());
            let body_loc = body.location();
            let (body_line, body_column) = source.offset_to_line_col(body_loc.start_offset());

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
