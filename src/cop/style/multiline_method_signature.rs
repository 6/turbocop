use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineMethodSignature;

impl Cop for MultilineMethodSignature {
    fn name(&self) -> &'static str {
        "Style/MultilineMethodSignature"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        // Must have parameters
        let params = match def_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        // Get the opening line (def keyword)
        let def_loc = def_node.def_keyword_loc();
        let (def_line, _) = source.offset_to_line_col(def_loc.start_offset());

        // Get the closing line of the params (rparen or last param)
        let params_loc = params.location();
        let params_end = params_loc.end_offset().saturating_sub(1);
        let (params_end_line, _) = source.offset_to_line_col(params_end);

        // Also check if there is a closing paren after params
        if let Some(rparen) = def_node.rparen_loc() {
            let (rparen_line, _) = source.offset_to_line_col(rparen.start_offset());
            if def_line != rparen_line {
                let (line, column) = source.offset_to_line_col(def_loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid multi-line method signatures.".to_string(),
                )];
            }
        } else if def_line != params_end_line {
            let (line, column) = source.offset_to_line_col(def_loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Avoid multi-line method signatures.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultilineMethodSignature, "cops/style/multiline_method_signature");
}
