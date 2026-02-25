use crate::cop::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineMethodSignature;

impl Cop for MultilineMethodSignature {
    fn name(&self) -> &'static str {
        "Style/MultilineMethodSignature"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
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
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Must have parameters
        if def_node.parameters().is_none() {
            return;
        }

        // RuboCop requires explicit parens (loc.begin of arguments must exist)
        let rparen = match def_node.rparen_loc() {
            Some(rp) => rp,
            // No explicit parens — no offense per RuboCop
            None => return,
        };
        // If there's an rparen, there must be an lparen
        if def_node.lparen_loc().is_none() {
            return;
        }

        // Get the opening line (def keyword) and closing line (rparen)
        let def_loc = def_node.def_keyword_loc();
        let (def_line, _) = source.offset_to_line_col(def_loc.start_offset());
        let (rparen_line, _) = source.offset_to_line_col(rparen.start_offset());

        // Not multiline — no offense
        if def_line == rparen_line {
            return;
        }

        // Check if correction would exceed max line length.
        // RuboCop's definition_width = byte distance from start of `def` to end of rparen.
        // This serves as a proxy: if the raw span (including newlines/indentation) exceeds
        // max_line_length, the single-line form certainly would too.
        let max_line_length = config.get_usize("MaxLineLength", 120);
        let line_length_enabled = config.get_bool("LineLengthEnabled", max_line_length > 0);
        if line_length_enabled && max_line_length > 0 {
            let def_start = def_loc.start_offset();
            let rparen_end = rparen.end_offset();
            let definition_width = rparen_end - def_start;
            let (_, indentation_width) = source.offset_to_line_col(def_start);
            if indentation_width + definition_width > max_line_length {
                return;
            }
        }

        let (line, column) = source.offset_to_line_col(def_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Avoid multi-line method signatures.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        MultilineMethodSignature,
        "cops/style/multiline_method_signature"
    );
}
