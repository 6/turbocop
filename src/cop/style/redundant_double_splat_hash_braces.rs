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
        // Look for **{...} in keyword arguments
        let keyword_hash = if let Some(kh) = node.as_keyword_hash_node() {
            kh
        } else if let Some(h) = node.as_hash_node() {
            // Also check plain hashes
            return self.check_hash_elements(source, h.elements().iter());
        } else {
            return Vec::new();
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
                // Check if the splatted value is a hash literal
                if let Some(value) = splat.value() {
                    if value.as_hash_node().is_some() {
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
