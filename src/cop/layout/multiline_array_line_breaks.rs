use crate::cop::shared::node_type::ARRAY_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Layout/MultilineArrayLineBreaks — each item in a multi-line array must start
/// on a separate line.
///
/// ## Investigation (2026-03-11)
///
/// **Root cause of 1,119 FPs:** The `AllowMultilineFinalElement` config option was
/// read but ignored (stored in `_allow_multiline_final` with underscore prefix).
/// When `AllowMultilineFinalElement: true`, the last element of a multiline array
/// is allowed to span multiple lines without triggering an offense. Many corpus
/// projects enable this option.
///
/// **Additional bug:** The `all_on_same_line?` guard compared bracket positions
/// (`open_line == close_line`) instead of element positions. RuboCop checks
/// whether all elements occupy the same line range, not whether the brackets do.
/// This caused false positives on arrays like `[\n  1, 2, 3,\n]` where elements
/// are on one line but brackets span multiple.
///
/// **Fix:** Rewrote to match the RuboCop `MultilineElementLineBreaks` mixin:
/// 1. `all_on_same_line?` guard checks element line ranges, not bracket lines
/// 2. `AllowMultilineFinalElement` changes the guard to compare first.start_line
///    vs last.start_line (ignoring the last element's span)
/// 3. Uses `last_seen_line` tracking algorithm (only updates on non-offending
///    elements) matching RuboCop exactly
///
/// ## Investigation (2026-03-14)
///
/// **Root cause of 69 FNs:** Rescue exception lists (`rescue FooError, BarError`)
/// are represented as `array` nodes in RuboCop's Parser gem AST, but Prism stores
/// them as individual exception nodes within `RescueNode.exceptions()` — NOT as
/// an `ArrayNode`. The cop only listened for `ARRAY_NODE` via `check_node`, so
/// rescue exception lists were never checked.
///
/// Additionally, Prism's `RescueNode` is NOT visited via `visit_branch_node_enter`
/// or `visit_leaf_node_enter` — it requires a dedicated `visit_rescue_node`
/// override in the Visit trait. This is the same issue found in
/// `Layout/ArrayAlignment`.
///
/// **Fix:** Added a `check_source` method with a dedicated `RescueVisitor` that
/// implements `visit_rescue_node` to find rescue exception lists and apply the
/// same multiline element check (same `all_on_same_line?` guard and
/// `last_seen_line` algorithm).
///
/// ## Investigation (2026-03-16)
///
/// **Root cause of 43 FNs:** Implicit arrays (no brackets) were skipped by the
/// `check_node` handler. The cop had a guard `if opening_loc().is_none()` that
/// returned early for implicit arrays — e.g. multi-assignment RHS
/// (`a, b = val1, val2`), method calls with implicit array args
/// (`config.cache_store = :redis, { ... }`), and constant assignments
/// (`ITEMS = :a, :b`). RuboCop's `on_array` fires on ALL array nodes including
/// implicit ones and applies the same line-break check.
///
/// **Fix:** Removed the implicit array skip. Prism's `ArrayNode.elements()`
/// returns the correct element list for both explicit `[...]` and implicit
/// arrays, so the `check_elements` logic works unchanged.
pub struct MultilineArrayLineBreaks;

impl MultilineArrayLineBreaks {
    /// Shared logic for checking that each element in a multi-line list starts on
    /// its own line. Used for both array elements and rescue exception lists.
    fn check_elements(
        &self,
        source: &SourceFile,
        elements: &[ruby_prism::Node<'_>],
        allow_multiline_final: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if elements.len() < 2 {
            return;
        }

        // RuboCop's all_on_same_line? guard — checks elements, not brackets
        let first_start_line = source
            .offset_to_line_col(elements[0].location().start_offset())
            .0;
        let last = elements.last().unwrap();

        if allow_multiline_final {
            let last_start_line = source.offset_to_line_col(last.location().start_offset()).0;
            if first_start_line == last_start_line {
                return;
            }
        } else {
            let last_end_line = source
                .offset_to_line_col(last.location().end_offset().saturating_sub(1))
                .0;
            if first_start_line == last_end_line {
                return;
            }
        }

        // Track last_line of the most recent non-offending element (matches RuboCop's
        // last_seen_line algorithm). When an element is flagged, last_seen_line is NOT
        // updated, so subsequent elements are compared against the last "good" element.
        let mut last_seen_line: isize = -1;
        for elem in elements {
            let (start_line, start_col) = source.offset_to_line_col(elem.location().start_offset());
            if last_seen_line >= start_line as isize {
                diagnostics.push(self.diagnostic(
                    source,
                    start_line,
                    start_col,
                    "Each item in a multi-line array must start on a separate line.".to_string(),
                ));
            } else {
                let end_line = source
                    .offset_to_line_col(elem.location().end_offset().saturating_sub(1))
                    .0;
                last_seen_line = end_line as isize;
            }
        }
    }
}

/// Dedicated visitor for rescue exception lists. Prism's `RescueNode` is not
/// dispatched through the generic `visit_branch_node_enter`/`visit_leaf_node_enter`
/// callbacks, so we need an explicit `visit_rescue_node` override.
struct RescueVisitor<'a> {
    cop: &'a MultilineArrayLineBreaks,
    source: &'a SourceFile,
    allow_multiline_final: bool,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for RescueVisitor<'_> {
    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        let exceptions: Vec<ruby_prism::Node<'pr>> = node.exceptions().iter().collect();
        self.cop.check_elements(
            self.source,
            &exceptions,
            self.allow_multiline_final,
            self.diagnostics,
        );
        ruby_prism::visit_rescue_node(self, node);
    }
}

impl Cop for MultilineArrayLineBreaks {
    fn name(&self) -> &'static str {
        "Layout/MultilineArrayLineBreaks"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_multiline_final = config.get_bool("AllowMultilineFinalElement", false);

        let array = match node.as_array_node() {
            Some(a) => a,
            None => return,
        };

        let elements: Vec<ruby_prism::Node<'_>> = array.elements().iter().collect();
        self.check_elements(source, &elements, allow_multiline_final, diagnostics);
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_multiline_final = config.get_bool("AllowMultilineFinalElement", false);

        // RescueNode is not dispatched through visit_branch_node_enter in Prism,
        // so we need a dedicated visitor to find rescue nodes for exception list
        // line break checking.
        let mut visitor = RescueVisitor {
            cop: self,
            source,
            allow_multiline_final,
            diagnostics,
        };
        visitor.visit(&parse_result.node());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full_with_config, run_cop_full_with_config};
    use std::collections::HashMap;

    crate::cop_fixture_tests!(
        MultilineArrayLineBreaks,
        "cops/layout/multiline_array_line_breaks"
    );

    #[test]
    fn rescue_exception_list_multiline() {
        let diags = run_cop_full_with_config(
            &MultilineArrayLineBreaks,
            b"begin\n  something\nrescue FooError, BarError,\n       BazError\n  retry\nend\n",
            CopConfig::default(),
        );
        // BarError is on same line as FooError → 1 offense
        assert_eq!(diags.len(), 1, "Expected 1 offense, got: {:?}", diags);
    }

    #[test]
    fn allow_multiline_final_element_ignores_multiline_last_hash() {
        let config = CopConfig {
            options: HashMap::from([(
                "AllowMultilineFinalElement".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };

        // Last element is a multiline hash — should be allowed
        assert_cop_no_offenses_full_with_config(
            &MultilineArrayLineBreaks,
            b"[1, 2, 3, {\n  a: 1\n}]\n",
            config,
        );
    }

    #[test]
    fn allow_multiline_final_element_still_flags_non_last() {
        let config = CopConfig {
            options: HashMap::from([(
                "AllowMultilineFinalElement".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };

        // Non-last elements on same line should still be flagged
        let diags = run_cop_full_with_config(
            &MultilineArrayLineBreaks,
            b"[1, 2, 3, {\n  a: 1\n}, 4]\n",
            config,
        );

        // 2, 3, and { are all on same line as 1 → 3 offenses
        assert_eq!(diags.len(), 3);
    }

    #[test]
    fn allow_multiline_final_element_no_offense_when_each_on_own_line() {
        let config = CopConfig {
            options: HashMap::from([(
                "AllowMultilineFinalElement".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };

        assert_cop_no_offenses_full_with_config(
            &MultilineArrayLineBreaks,
            b"[\n  1,\n  2,\n  foo(\n    bar\n  )\n]\n",
            config,
        );
    }
}
