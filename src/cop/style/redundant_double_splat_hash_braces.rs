use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantDoubleSplatHashBraces;

impl Cop for RedundantDoubleSplatHashBraces {
    fn name(&self) -> &'static str {
        "Style/RedundantDoubleSplatHashBraces"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Look for **{key: val, ...} in keyword arguments (KeywordHashNode in method calls)
        // Only check KeywordHashNode (method call keyword args), not plain HashNode
        let keyword_hash = match node.as_keyword_hash_node() {
            Some(kh) => kh,
            None => return Vec::new(),
        };

        self.check_hash_elements(source, keyword_hash.elements().iter())
    }
}

impl RedundantDoubleSplatHashBraces {
    fn check_hash_elements<'a, I>(&self, source: &SourceFile, elements: I) -> Vec<Diagnostic>
    where
        I: Iterator<Item = ruby_prism::Node<'a>>,
    {
        let mut diagnostics = Vec::new();

        for element in elements {
            if let Some(splat) = element.as_assoc_splat_node() {
                // Check if the splatted value is a hash literal with elements
                if let Some(value) = splat.value() {
                    if let Some(hash) = value.as_hash_node() {
                        // Don't flag empty hashes: **{}
                        if hash.elements().iter().next().is_none() {
                            continue;
                        }
                        // Skip if any pair uses hash rocket (=>) syntax
                        // because non-symbol keys can't be keyword arguments
                        let has_hash_rocket = hash.elements().iter().any(|e| {
                            if let Some(assoc) = e.as_assoc_node() {
                                assoc.operator_loc().is_some()
                                    && assoc.operator_loc().unwrap().as_slice() == b"=>"
                            } else {
                                false
                            }
                        });
                        if has_hash_rocket {
                            continue;
                        }
                        let loc = element.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Remove the redundant double splat and braces, use keyword arguments directly.".to_string(),
                        ));
                    }
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantDoubleSplatHashBraces, "cops/style/redundant_double_splat_hash_braces");
}
