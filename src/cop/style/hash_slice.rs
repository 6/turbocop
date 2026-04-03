use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use super::hash_subset::{self, HashSubsetMode};

/// Detects `Hash#reject`, `Hash#select`, and `Hash#filter` calls that can be
/// replaced with `Hash#slice`.
///
/// 2026-04 corpus fix:
/// - added missed implicit-receiver `select`/`reject` calls inside hash helpers
/// - handled negated `include?`, `in?`, and `exclude?` membership predicates
/// - supported `eql?` and array-literal formatting for `slice(:a, :b)`
/// - kept the fix narrow by skipping range-backed membership checks and
///   safe-navigation predicates like `cached_methods_params&.include?(key)`,
///   which RuboCop does not flag
pub struct HashSlice;

impl Cop for HashSlice {
    fn name(&self) -> &'static str {
        "Style/HashSlice"
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
        hash_subset::check_hash_subset(self, HashSubsetMode::Slice, source, node, diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashSlice, "cops/style/hash_slice");
}
