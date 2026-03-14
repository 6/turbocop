use crate::cop::metrics::method_length::{body_has_heredoc, max_descendant_end_line};
use crate::cop::node_type::CALL_NODE;
use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE, is_rspec_example};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// FP=12, FN=27.
///
/// ### FP root causes (fixed):
/// 1. Missing receiver check — calls like `obj.it { ... }` or `config.specify { ... }`
///    with blocks were being counted as RSpec examples. RuboCop's `example?` matcher
///    uses nil receiver only. Added receiver guard.
/// 2. Numblock/itblock handling — RuboCop's `on_block` does NOT fire for `numblock`
///    (numbered params like `_1`) or `itblock` (Ruby 3.4 `it` keyword param). In Prism
///    these are still BlockNode but with NumberedParametersNode or ItParametersNode as
///    parameters. Added guard to skip these block types.
/// 3. Heredoc body lines counted in nested block `end` (FP=11, 2026-03-14): When the
///    body is a single block call (e.g., `Dir.chdir do ... end`) containing heredocs,
///    RuboCop switches from `body.source.lines` to `source_from_node_with_heredoc(body)`.
///    The latter tracks `descendant.last_line` (not the container node's `last_line`),
///    so the nested block's closing `end` keyword is EXCLUDED from the count. Nitrocop
///    was counting the nested `end` line, producing [6/5] where RuboCop counts [5/5].
///    Fix: when the body has heredocs, use `max_descendant_end_line` to compute the
///    effective end offset, matching RuboCop's `source_from_node_with_heredoc` behavior.
///
/// ### FN root causes (fixed):
/// 1. CountAsOne reduction was using line span instead of code length. RuboCop counts
///    non-blank, non-comment lines in foldable constructs (`code_length`), subtracts
///    `code_length - 1`. Nitrocop was subtracting `line_span - 1`, which over-reduces
///    when foldable constructs contain blank/comment lines.
/// 2. CountAsOne only checked top-level statements, missing arrays/hashes nested inside
///    assignments or other expressions. RuboCop's `each_top_level_descendant` recursively
///    descends into all descendants. Rewrote using Visit trait with skip_depth tracking.
pub struct ExampleLength;

impl Cop for ExampleLength {
    fn name(&self) -> &'static str {
        "RSpec/ExampleLength"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if !is_rspec_example(method_name) {
            return;
        }

        // RuboCop's example? matcher requires nil receiver (bare `it`, not `obj.it`)
        if call.receiver().is_some() {
            return;
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        // RuboCop's `on_block` does NOT fire for numblock (numbered params like _1)
        // or itblock (Ruby 3.4 `it` keyword param). In Prism these are still BlockNode
        // but with NumberedParametersNode or ItParametersNode as the parameters.
        // Skip them to match RuboCop behavior.
        if let Some(params) = block.parameters() {
            if params.as_numbered_parameters_node().is_some()
                || params.as_it_parameters_node().is_some()
            {
                return;
            }
        }

        let max = config.get_usize("Max", 5);

        // Count body lines, skipping blank lines and comment lines.
        // RuboCop's CodeLength mixin uses CountComments config (default false for
        // RSpec/ExampleLength), meaning comment-only lines are NOT counted.
        let count_comments = config.get_bool("CountComments", false);
        let block_loc = block.location();
        let default_end = block_loc
            .end_offset()
            .saturating_sub(1)
            .max(block_loc.start_offset());

        // When the body contains heredocs, RuboCop switches from `body.source.lines` to
        // `source_from_node_with_heredoc(body)`. The latter uses `descendant.last_line`
        // (not the body node's own `last_line`), so the outermost nested block's closing
        // `end` keyword is excluded. Replicate by adjusting the effective end offset.
        let effective_end = if let Some(ref body) = block.body() {
            if body_has_heredoc(source, body) {
                let max_line = max_descendant_end_line(source, body);
                if max_line > 0 {
                    source
                        .line_col_to_offset(max_line + 1, 0)
                        .unwrap_or(default_end)
                } else {
                    default_end
                }
            } else {
                default_end
            }
        } else {
            default_end
        };

        let count = util::count_body_lines(
            source,
            block_loc.start_offset(),
            effective_end,
            count_comments,
        );

        // Adjust for CountAsOne: multi-line arrays/hashes/heredocs count as 1 line
        let count_as_one = config.get_string_array("CountAsOne").unwrap_or_default();
        let adjusted = if !count_as_one.is_empty() {
            let reduction =
                count_multiline_reductions(source, &block, &count_as_one, count_comments);
            count.saturating_sub(reduction)
        } else {
            count
        };

        if adjusted > max {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Example has too many lines. [{adjusted}/{max}]"),
            ));
        }
    }
}

/// Count how many extra lines multi-line constructs add.
/// RuboCop replaces each foldable construct with 1 line: `length - code_length(node) + 1`.
/// So the reduction per construct is `code_length(node) - 1`, where `code_length` counts
/// non-blank, non-comment lines in the construct's source (matching `irrelevant_line?`).
///
/// Uses `each_top_level_descendant` logic via Visit trait: recursively descends into
/// all child nodes looking for foldable types. When found, counts reduction and does
/// NOT recurse further (only top-level foldable nodes are folded).
fn count_multiline_reductions(
    source: &SourceFile,
    block: &ruby_prism::BlockNode<'_>,
    count_as_one: &[String],
    count_comments: bool,
) -> usize {
    use ruby_prism::Visit;

    struct FoldableVisitor<'a> {
        source: &'a SourceFile,
        count_as_one: &'a [String],
        count_comments: bool,
        reduction: usize,
        /// When > 0, we're inside a foldable node and should NOT recurse further
        skip_depth: usize,
    }

    impl FoldableVisitor<'_> {
        fn is_foldable(&self, node: &ruby_prism::Node<'_>) -> bool {
            if self.count_as_one.iter().any(|s| s == "array") && node.as_array_node().is_some() {
                return true;
            }
            if self.count_as_one.iter().any(|s| s == "hash")
                && (node.as_hash_node().is_some() || node.as_keyword_hash_node().is_some())
            {
                return true;
            }
            if self.count_as_one.iter().any(|s| s == "heredoc")
                && (node.as_interpolated_string_node().is_some() || node.as_string_node().is_some())
            {
                return true;
            }
            if self.count_as_one.iter().any(|s| s == "method_call") {
                if let Some(call) = node.as_call_node() {
                    if call.block().is_none() {
                        return true;
                    }
                }
            }
            false
        }
    }

    impl<'pr> Visit<'pr> for FoldableVisitor<'_> {
        fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
            if self.skip_depth > 0 {
                self.skip_depth += 1;
                return;
            }
            if self.is_foldable(&node) {
                let code_len = node_code_length(self.source, &node.location(), self.count_comments);
                if code_len > 1 {
                    self.reduction += code_len - 1;
                }
                // Skip recursing into this foldable node
                self.skip_depth = 1;
            }
        }

        fn visit_branch_node_leave(&mut self) {
            if self.skip_depth > 0 {
                self.skip_depth -= 1;
            }
        }

        fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
            if self.skip_depth > 0 {
                return;
            }
            // Leaf nodes that are foldable (e.g., single-line string)
            if self.is_foldable(&node) {
                let code_len = node_code_length(self.source, &node.location(), self.count_comments);
                if code_len > 1 {
                    self.reduction += code_len - 1;
                }
            }
        }
    }

    let body = match block.body() {
        Some(b) => b,
        None => return 0,
    };

    let mut visitor = FoldableVisitor {
        source,
        count_as_one,
        count_comments,
        reduction: 0,
        skip_depth: 0,
    };
    visitor.visit(&body);
    visitor.reduction
}

/// Trim leading and trailing whitespace (space, tab, CR) from a byte slice.
fn trim_ws(b: &[u8]) -> &[u8] {
    let start = b
        .iter()
        .position(|&c| c != b' ' && c != b'\t' && c != b'\r');
    match start {
        Some(s) => {
            let end = b
                .iter()
                .rposition(|&c| c != b' ' && c != b'\t' && c != b'\r')
                .unwrap();
            &b[s..=end]
        }
        None => &[],
    }
}

/// Count non-blank, non-comment lines within a node's source range.
/// Matches RuboCop's `CodeLengthCalculator#code_length` for non-classlike nodes:
/// `body.source.lines.count { |line| !irrelevant_line?(line) }`.
fn node_code_length(
    source: &SourceFile,
    loc: &ruby_prism::Location<'_>,
    count_comments: bool,
) -> usize {
    let (start_line, _) = source.offset_to_line_col(loc.start_offset());
    let end_off = loc.end_offset().saturating_sub(1).max(loc.start_offset());
    let (end_line, _) = source.offset_to_line_col(end_off);

    let lines: Vec<&[u8]> = source.lines().collect();
    let mut count = 0;
    for line_num in start_line..=end_line {
        if line_num > lines.len() {
            break;
        }
        let line = lines[line_num - 1];
        let trimmed = trim_ws(line);
        if trimmed.is_empty() {
            continue;
        }
        if !count_comments && trimmed.starts_with(b"#") {
            continue;
        }
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExampleLength, "cops/rspec/example_length");

    use crate::testutil;

    fn offenses(source: &str) -> Vec<crate::diagnostic::Diagnostic> {
        testutil::run_cop_full_internal(
            &ExampleLength,
            source.as_bytes(),
            CopConfig::default(),
            "spec/test_spec.rb",
        )
    }

    #[test]
    fn does_not_fire_on_numblock() {
        // Numbered parameters create numblock in Parser gem, on_block doesn't match
        let src = "RSpec.describe Foo do\n  it do\n    _1.a\n    _1.b\n    _1.c\n    _1.d\n    _1.e\n    _1.f\n  end\nend\n";
        assert!(offenses(src).is_empty(), "Should not fire on numblock");
    }

    #[test]
    fn fires_on_regular_block_over_max() {
        let src = "RSpec.describe Foo do\n  it do\n    a = 1\n    b = 2\n    c = 3\n    d = 4\n    e = 5\n    f = 6\n  end\nend\n";
        let diags = offenses(src);
        assert_eq!(diags.len(), 1, "Should fire once for 6-line example");
        assert!(
            diags[0].message.contains("[6/5]"),
            "Expected [6/5] in message, got: {}",
            diags[0].message
        );
    }

    #[test]
    fn blank_lines_not_counted() {
        // 5 code lines + 2 blank lines = only 5 should count
        let src = "RSpec.describe Foo do\n  it do\n    a = 1\n\n    b = 2\n    c = 3\n\n    d = 4\n    e = 5\n  end\nend\n";
        assert!(offenses(src).is_empty(), "Blank lines should not count");
    }

    #[test]
    fn comment_lines_not_counted_by_default() {
        // 5 code lines + 3 comment lines = only 5 should count
        let src = "RSpec.describe Foo do\n  it do\n    # comment 1\n    a = 1\n    # comment 2\n    b = 2\n    c = 3\n    # comment 3\n    d = 4\n    e = 5\n  end\nend\n";
        assert!(
            offenses(src).is_empty(),
            "Comment lines should not count by default"
        );
    }

    #[test]
    fn count_as_one_array_with_blanks() {
        // Array spans 7 lines (including blank line inside).
        // RuboCop: code_length(array) = 6 non-blank lines, reduction = 6-1 = 5.
        // Total body: a=1 (1) + array collapsed to 1 = 2 lines. <= Max(5), no offense.
        use std::collections::HashMap;
        let mut options = HashMap::new();
        options.insert(
            "CountAsOne".to_string(),
            serde_yml::Value::Sequence(vec![serde_yml::Value::String("array".to_string())]),
        );
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        let src = b"RSpec.describe Foo do\n  it do\n    a = 1\n    arr = [\n      1,\n\n      2,\n      3,\n      4\n    ]\n  end\nend\n";
        let diags =
            testutil::run_cop_full_internal(&ExampleLength, src, config, "spec/test_spec.rb");
        // 2 code lines after folding (a=1 + array-as-1) — no offense with Max 5
        assert!(
            diags.is_empty(),
            "CountAsOne array with blanks should fold correctly, got: {:?}",
            diags
        );
    }
}
