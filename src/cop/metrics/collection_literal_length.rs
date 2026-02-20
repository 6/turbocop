use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ARRAY_NODE, HASH_NODE, KEYWORD_HASH_NODE};

pub struct CollectionLiteralLength;

impl Cop for CollectionLiteralLength {
    fn name(&self) -> &'static str {
        "Metrics/CollectionLiteralLength"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, HASH_NODE, KEYWORD_HASH_NODE]
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
        let max = config.get_usize("LengthThreshold", 250);

        // Check ArrayNode
        if let Some(array) = node.as_array_node() {
            let count = array.elements().len();
            if count > max {
                let loc = array.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Collection literal is too long. [{count}/{max}]"),
                ));
            }
        }

        // Check HashNode
        if let Some(hash) = node.as_hash_node() {
            let count = hash.elements().len();
            if count > max {
                let loc = hash.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Collection literal is too long. [{count}/{max}]"),
                ));
            }
        }

        // Check KeywordHashNode
        if let Some(hash) = node.as_keyword_hash_node() {
            let count = hash.elements().len();
            if count > max {
                let loc = hash.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Collection literal is too long. [{count}/{max}]"),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_scenario_fixture_tests!(
        CollectionLiteralLength,
        "cops/metrics/collection_literal_length",
        large_array = "large_array.rb",
        large_hash = "large_hash.rb",
        larger_array = "larger_array.rb",
    );
}
