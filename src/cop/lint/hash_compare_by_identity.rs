use crate::cop::shared::node_type::{
    CALL_NODE, INDEX_AND_WRITE_NODE, INDEX_OPERATOR_WRITE_NODE, INDEX_OR_WRITE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for hashes keyed by `object_id`.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=0, FN=18.
///
/// FN:
/// - The implementation only matched `foo.object_id`, not bare `object_id`. Real-world code
///   often uses `hash[object_id]` inside instance methods, where Prism represents the key as a
///   zero-receiver `CallNode`.
/// - Residual corpus misses were `hash[key] ||= value` forms. Prism represents those as
///   `IndexOrWriteNode` / `IndexAndWriteNode` / `IndexOperatorWriteNode`, not normal `CallNode`s.
/// - Rerunning the corpus gate after handling both shapes matched RuboCop exactly:
///   expected 74, actual 74, with no potential FP/FN.
///
/// ## Extended corpus FP investigation (2026-03-24)
///
/// Extended corpus: 203 matches, 2 FP, 0 FN. Both FPs are in
/// `pitluga/supply_drop` at `examples/vendored-puppet/vendor/puppet-2.7.8/lib/puppet/util/zaml.rb`.
/// The file has syntax errors (invalid multibyte escapes at line 224) that cause RuboCop's parser
/// to fail, so RuboCop only reports `Lint/Syntax` and never runs `Lint/HashCompareByIdentity`.
/// Prism parses the file successfully, so nitrocop correctly detects the `object_id` key pattern.
/// This is a parser-level difference, not a cop detection bug — the cop logic is correct.
/// Fixing this would require skipping non-syntax cops on files with parse errors at the
/// engine/runner level, not in individual cop implementations.
pub struct HashCompareByIdentity;

const HASH_KEY_METHODS: &[&[u8]] = &[b"key?", b"has_key?", b"fetch", b"[]", b"[]="];

impl Cop for HashCompareByIdentity {
    fn name(&self) -> &'static str {
        "Lint/HashCompareByIdentity"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            INDEX_AND_WRITE_NODE,
            INDEX_OPERATOR_WRITE_NODE,
            INDEX_OR_WRITE_NODE,
        ]
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
        if let Some(call) = node.as_call_node() {
            let method_name = call.name().as_slice();
            if !HASH_KEY_METHODS.contains(&method_name) || call.receiver().is_none() {
                return;
            }

            if let Some(first_arg) = first_argument(call.arguments()) {
                self.add_offense_if_object_id_key(source, &call.as_node(), &first_arg, diagnostics);
            }
            return;
        }

        if let Some(write) = node.as_index_operator_write_node() {
            if let Some(first_arg) = first_argument(write.arguments()) {
                self.add_offense_if_object_id_key(
                    source,
                    &write.as_node(),
                    &first_arg,
                    diagnostics,
                );
            }
            return;
        }

        if let Some(write) = node.as_index_or_write_node() {
            if let Some(first_arg) = first_argument(write.arguments()) {
                self.add_offense_if_object_id_key(
                    source,
                    &write.as_node(),
                    &first_arg,
                    diagnostics,
                );
            }
            return;
        }

        if let Some(write) = node.as_index_and_write_node() {
            if let Some(first_arg) = first_argument(write.arguments()) {
                self.add_offense_if_object_id_key(
                    source,
                    &write.as_node(),
                    &first_arg,
                    diagnostics,
                );
            }
        }
    }
}

impl HashCompareByIdentity {
    fn add_offense_if_object_id_key(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        first_arg: &ruby_prism::Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if !is_object_id_call(first_arg) {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `Hash#compare_by_identity` instead of using `object_id` for keys.".to_string(),
        ));
    }
}

fn first_argument(
    arguments: Option<ruby_prism::ArgumentsNode<'_>>,
) -> Option<ruby_prism::Node<'_>> {
    arguments?.arguments().iter().next()
}

fn is_object_id_call(node: &ruby_prism::Node<'_>) -> bool {
    node.as_call_node()
        .is_some_and(|call| call.name().as_slice() == b"object_id")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashCompareByIdentity, "cops/lint/hash_compare_by_identity");
}
