use crate::cop::node_type::{HASH_NODE, KEYWORD_HASH_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

const MSG: &str = "Remove the redundant double splat and braces, use keyword arguments directly.";

/// Flags redundant double-splat hash braces in both method-call keyword hashes
/// and plain hash literals, including `**({ ... }.merge(...))` forms.
///
/// Corpus investigation (2026-03-30): 193 matches, 0 FP, 2 FN.
///
/// Previously missed patterns (now fixed):
/// 1. Prism wraps `**({ ... }.merge(args))` in `ParenthesesNode`/`StatementsNode`
///    before the merge call.
/// 2. Hash literals like `{ a: 1, **{ b: 2 } }` use `HashNode`, not
///    `KeywordHashNode`.
///
/// Fix: visit both hash node kinds and unwrap grouped expressions before
/// following `merge`/`merge!` receiver chains down to a non-empty braced hash
/// with only keyword-style pairs.
pub struct RedundantDoubleSplatHashBraces;

impl Cop for RedundantDoubleSplatHashBraces {
    fn name(&self) -> &'static str {
        "Style/RedundantDoubleSplatHashBraces"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[HASH_NODE, KEYWORD_HASH_NODE]
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
        if let Some(keyword_hash) = node.as_keyword_hash_node() {
            diagnostics.extend(self.check_hash_elements(source, keyword_hash.elements().iter()));
        } else if let Some(hash) = node.as_hash_node() {
            diagnostics.extend(self.check_hash_elements(source, hash.elements().iter()));
        }
    }
}

impl RedundantDoubleSplatHashBraces {
    fn redundant_hash(value: &ruby_prism::Node<'_>) -> bool {
        if let Some(parentheses) = value.as_parentheses_node() {
            return parentheses
                .body()
                .is_some_and(|body| Self::redundant_hash(&body));
        }

        if let Some(statements) = value.as_statements_node() {
            let mut body = statements.body().iter();
            let Some(first) = body.next() else {
                return false;
            };
            if body.next().is_some() {
                return false;
            }
            return Self::redundant_hash(&first);
        }

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
