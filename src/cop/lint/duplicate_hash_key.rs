use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, HASH_NODE, KEYWORD_HASH_NODE};

pub struct DuplicateHashKey;

impl Cop for DuplicateHashKey {
    fn name(&self) -> &'static str {
        "Lint/DuplicateHashKey"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, HASH_NODE, KEYWORD_HASH_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let elements = if let Some(hash_node) = node.as_hash_node() {
            hash_node.elements()
        } else if let Some(kw_hash) = node.as_keyword_hash_node() {
            kw_hash.elements()
        } else {
            return Vec::new();
        };

        let mut seen = HashSet::new();
        let mut diagnostics = Vec::new();

        for element in elements.iter() {
            let assoc = match element.as_assoc_node() {
                Some(a) => a,
                None => continue, // skip AssocSplatNode (**)
            };

            let key = assoc.key();
            let key_loc = key.location();
            let key_text = key_loc.as_slice();

            if !seen.insert(key_text.to_vec()) {
                let (line, column) = source.offset_to_line_col(key_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Duplicated key in hash literal.".to_string(),
                ));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateHashKey, "cops/lint/duplicate_hash_key");
}
