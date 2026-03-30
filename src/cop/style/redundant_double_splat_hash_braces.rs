use crate::cop::node_type::KEYWORD_HASH_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

const MSG: &str = "Remove the redundant double splat and braces, use keyword arguments directly.";

/// Flags keyword splats whose value is a braced hash literal, including
/// `**{...}.merge(...)` and `**{...}.merge!(...)` chains.
///
/// Corpus investigation (2026-03-30): 0 FP, 59 FN, 136 matches.
///
/// Nitrocop only handled a direct `HashNode` value under `**`, so it missed
/// RuboCop offenses where Prism wraps the braced hash in a `CallNode` for
/// `.merge`/`.merge!`. Fix: walk merge-call receivers from the keyword splat
/// value and only flag when the chain bottoms out at a non-empty braced hash
/// without hash-rocket pairs.
pub struct RedundantDoubleSplatHashBraces;

impl Cop for RedundantDoubleSplatHashBraces {
    fn name(&self) -> &'static str {
        "Style/RedundantDoubleSplatHashBraces"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[KEYWORD_HASH_NODE]
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
        // Look for **{key: val, ...} in keyword arguments (KeywordHashNode in method calls)
        // Only check KeywordHashNode (method call keyword args), not plain HashNode
        let keyword_hash = match node.as_keyword_hash_node() {
            Some(kh) => kh,
            None => return,
        };

        diagnostics.extend(self.check_hash_elements(source, keyword_hash.elements().iter()));
    }
}

impl RedundantDoubleSplatHashBraces {
    fn redundant_hash(value: &ruby_prism::Node<'_>) -> bool {
        if let Some(hash) = value.as_hash_node() {
            return Self::convertible_hash(&hash);
        }

        let Some(call) = value.as_call_node() else {
            return false;
        };
        if !Self::is_merge_call(&call) {
            return false;
        }

        let Some(receiver) = call.receiver() else {
            return false;
        };
        Self::redundant_hash(&receiver)
    }

    fn is_merge_call(call: &ruby_prism::CallNode<'_>) -> bool {
        matches!(call.name().as_slice(), b"merge" | b"merge!")
    }

    fn convertible_hash(hash: &ruby_prism::HashNode<'_>) -> bool {
        if hash.elements().iter().next().is_none() {
            return false;
        }

        !hash.elements().iter().any(|element| {
            element.as_assoc_node().is_some_and(|assoc| {
                assoc
                    .operator_loc()
                    .is_some_and(|operator| operator.as_slice() == b"=>")
            })
        })
    }

    fn check_hash_elements<'a, I>(&self, source: &SourceFile, elements: I) -> Vec<Diagnostic>
    where
        I: Iterator<Item = ruby_prism::Node<'a>>,
    {
        let mut diagnostics = Vec::new();

        for element in elements {
            if let Some(splat) = element.as_assoc_splat_node() {
                let Some(value) = splat.value() else {
                    continue;
                };
                if !Self::redundant_hash(&value) {
                    continue;
                }

                let loc = element.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantDoubleSplatHashBraces,
        "cops/style/redundant_double_splat_hash_braces"
    );
}
