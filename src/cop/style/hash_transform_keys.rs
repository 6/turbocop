use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use super::hash_transform_method::{self, TransformMode};

/// Detects hash key transformations that can use `transform_keys` instead.
///
/// Handles these RuboCop-compatible patterns:
/// - `each_with_object({}) { |(k, v), h| h[expr(k)] = v }` → `transform_keys`
/// - `Hash[_.map { |k, v| [expr(k), v] }]` → `transform_keys`
/// - `_.map { |k, v| [expr(k), v] }.to_h` → `transform_keys`
/// - `_.to_h { |k, v| [expr(k), v] }` → `transform_keys`
///
/// Corpus investigation found two root causes:
/// - false negatives came from the missing `map/collect ... .to_h` and `to_h { ... }`
///   branches;
/// - false positives came from treating array-like receivers (`each_with_index`,
///   `with_index`, `zip`) as hashes, from accepting key expressions derived
///   from the value or memo variable instead of the original key, and from
///   accepting destructured rest params like `|(idx, value, *)|` as if they
///   were exact two-element hash pairs.
pub struct HashTransformKeys;

impl Cop for HashTransformKeys {
    fn name(&self) -> &'static str {
        "Style/HashTransformKeys"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        hash_transform_method::check_hash_transform(
            self,
            TransformMode::Keys,
            source,
            node,
            diagnostics,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashTransformKeys, "cops/style/hash_transform_keys");
}
