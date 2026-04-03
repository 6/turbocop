use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use super::hash_transform_method::{self, TransformMode};

/// Style/HashTransformValues detects hash iteration patterns that can be
/// replaced with `transform_values`.
///
/// ## Patterns detected (matching RuboCop)
///
/// 1. `each_with_object({}) { |(k, v), h| h[k] = expr(v) }`
/// 2. `Hash[x.map { |k, v| [k, expr(v)] }]`
/// 3. `x.map { |k, v| [k, expr(v)] }.to_h`
/// 4. `x.to_h { |k, v| [k, expr(v)] }`
///
/// ## Investigation findings (corpus: 47 FP, 104 FN)
///
/// **FP root causes:**
/// - Missing destructured-params check: the cop fired on `|item, memo|` single-param
///   blocks (e.g. `items.each_with_object({}) { |item, result| result[item] = true }`).
///   RuboCop requires `|(k, v), h|` destructured params, confirming the receiver yields
///   key-value pairs (i.e., is a hash). Without this, array-to-hash patterns were falsely
///   flagged.
/// - Missing memo-variable check: value expressions referencing the memo hash (e.g.
///   `h[k] = h.size + v`) can't use transform_values.
///
/// **FN root causes:**
/// - Only `each_with_object` was implemented. The three other patterns
///   (`Hash[_.map]`, `_.map.to_h`, `_.to_h`) were completely missing.
///
/// **Fixes applied:**
/// - Added destructured block parameter validation (must be `|(k, v), h|` with
///   MultiTargetNode) for `each_with_object`.
/// - Added memo-variable check for `each_with_object` value expressions.
/// - Implemented `Hash[_.map/collect]`, `_.map/collect.to_h`, and `_.to_h` patterns.
/// - Added `array_receiver?` check to exclude array literals.
/// - All four patterns share common validation: key must pass through unchanged,
///   value must be transformed (not noop), value transformation must not reference the key.
/// - Fixed `::Hash[...]` (ConstantPathNode) not being recognized — the receiver check
///   compared raw source bytes which included the `::` prefix. Replaced with
///   `is_simple_constant` which handles both `Hash` and `::Hash`.
/// - Multi-line blocks and `do...end` syntax already worked correctly with the
///   existing Prism-based detection (no code change needed for those patterns).
/// - Replaced text-based `contains_identifier` with AST-based `node_contains_lvar_read`
///   for checking whether value expressions reference the key or memo parameter.
///   The text-based approach falsely matched key param names appearing as Ruby symbols
///   (`:name`, `&:label`) or keyword arguments (`name: nil`), causing false negatives
///   in patterns like `x.map { |name, attr| [name, Param.new(name: nil)] }.to_h`
///   where `name:` is a keyword arg, not a local variable reference to `name`.
pub struct HashTransformValues;

impl Cop for HashTransformValues {
    fn name(&self) -> &'static str {
        "Style/HashTransformValues"
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
            TransformMode::Values,
            source,
            node,
            diagnostics,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashTransformValues, "cops/style/hash_transform_values");
}
