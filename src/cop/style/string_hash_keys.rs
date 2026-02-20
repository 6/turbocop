use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, HASH_NODE, KEYWORD_HASH_NODE, STRING_NODE};

pub struct StringHashKeys;

impl Cop for StringHashKeys {
    fn name(&self) -> &'static str {
        "Style/StringHashKeys"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, HASH_NODE, KEYWORD_HASH_NODE, STRING_NODE]
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

        let elements = if let Some(hash) = node.as_hash_node() {
            hash.elements().iter().collect::<Vec<_>>()
        } else if let Some(kw_hash) = node.as_keyword_hash_node() {
            kw_hash.elements().iter().collect::<Vec<_>>()
        } else {
            return;
        };

        for element in elements {
            if let Some(assoc) = element.as_assoc_node() {
                let key = assoc.key();
                if key.as_string_node().is_some() {
                    let loc = key.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Prefer symbols instead of strings as hash keys.".to_string(),
                    ));
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StringHashKeys, "cops/style/string_hash_keys");
}
