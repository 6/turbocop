use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use super::hash_subset::{self, HashSubsetMode};

/// Detects `Hash#reject`, `Hash#select`, and `Hash#filter` calls that can be
/// replaced with `Hash#except`.
///
/// 2026-03 corpus fix:
/// - added missed `eql?` and `in?` shapes
/// - added support for implicit-receiver `reject { ... }` inside `except` helpers
/// - handled bare-argument calls like `keys.include? key`
/// - kept the fix narrow by skipping safe-navigation predicate calls like
///   `excluded_columns[table_name]&.include?(key)`, which RuboCop does not flag
pub struct HashExcept;

impl Cop for HashExcept {
    fn name(&self) -> &'static str {
        "Style/HashExcept"
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
        hash_subset::check_hash_subset(self, HashSubsetMode::Except, source, node, diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashExcept, "cops/style/hash_except");
}
