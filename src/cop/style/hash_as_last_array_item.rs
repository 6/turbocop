use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct HashAsLastArrayItem;

impl Cop for HashAsLastArrayItem {
    fn name(&self) -> &'static str {
        "Style/HashAsLastArrayItem"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let array = match node.as_array_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Only check explicit array literals (those with `[` opening)
        // Skip implicit arrays (e.g., method arguments)
        if array.opening_loc().is_none() {
            return Vec::new();
        }

        let style = config.get_str("EnforcedStyle", "braces");

        let elements: Vec<_> = array.elements().iter().collect();
        if elements.is_empty() {
            return Vec::new();
        }

        let last = &elements[elements.len() - 1];

        match style {
            "braces" => {
                // Flag keyword hash (no braces) as last array item
                if last.as_keyword_hash_node().is_some() {
                    // RuboCop skips when ALL elements are hashes in the expected style.
                    // In "braces" mode, that means all elements must be HashNode (with braces).
                    let all_expected = elements.iter().all(|e| e.as_hash_node().is_some());
                    if all_expected {
                        return Vec::new();
                    }
                    // Don't flag if second-to-last element is also a hash
                    if elements.len() >= 2 {
                        let second_last = &elements[elements.len() - 2];
                        if second_last.as_keyword_hash_node().is_some() || second_last.as_hash_node().is_some() {
                            return Vec::new();
                        }
                    }
                    let loc = last.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Wrap hash in `{` and `}`.".to_string(),
                    )];
                }
            }
            "no_braces" => {
                // Flag hash literal (with braces) as last array item
                if let Some(hash) = last.as_hash_node() {
                    // Don't flag empty hashes
                    if hash.elements().iter().next().is_none() {
                        return Vec::new();
                    }
                    // RuboCop skips when ALL elements are hashes in the expected style.
                    // In "no_braces" mode, that means all elements must be KeywordHashNode (without braces).
                    let all_expected = elements.iter().all(|e| e.as_keyword_hash_node().is_some());
                    if all_expected {
                        return Vec::new();
                    }
                    let loc = hash.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Omit the braces around the hash.".to_string(),
                    )];
                }
            }
            _ => {}
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashAsLastArrayItem, "cops/style/hash_as_last_array_item");
}
