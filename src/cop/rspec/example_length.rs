use crate::cop::metrics::method_length::{body_has_heredoc, max_descendant_end_line};
use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::{
    RSPEC_DEFAULT_INCLUDE, collect_foldable_ranges, collect_heredoc_ranges, count_body_lines_ex,
    is_rspec_example,
};
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
///
/// ## 2026-03-30 investigation
///
/// Added fixture coverage for the two YARD heredoc/comment transition examples that
/// were reported as corpus false negatives. The direct cop path (`run_cop_full_internal`
/// and the fixture harness) already matches RuboCop on both `[7/5]` and `[10/5]`
/// reproductions, so the remaining discrepancy is outside this detector.
///
/// The mismatch only appears through the full CLI/config path for plugin cops:
/// `--force-default-config` builds an empty resolved config, which disables the
/// `RSpec` plugin department before this cop's AST walk runs. That config-layer
/// behavior is not fixed here; keep validating this cop with `check_cop.py` and the
/// fixture reproductions rather than assuming an isolated absolute-path CLI run
/// exercises the detector.
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
        let count_as_one = config.get_string_array("CountAsOne").unwrap_or_default();
        let adjusted = count_example_lines(source, &block, count_comments, &count_as_one);

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

/// Count body lines of an RSpec example block, matching RuboCop's CodeLengthCalculator.
///
/// Key behaviors:
///
/// 1. **Body-based start**: uses body.location().start_offset() to avoid including
///    `do` or `{` delimiter lines. For brace blocks where body starts on the same
///    line as `{`, shifts effective_start back one line so body_start_line is counted.
///
/// 2. **Heredoc descendants**: when the body contains heredocs, uses
///    max_descendant_end_line to compute the effective end (matching RuboCop's
///    `source_from_node_with_heredoc` which excludes the body root's `end` keyword).
///
/// 3. **Brace block closing line**: for `{ }` blocks where body ends on same line
///    as `}`, extends effective_end to include that line (matches `body.source.lines`).
///
/// 4. **CountAsOne folding**: applies collect_foldable_ranges for arrays/hashes/etc.
fn count_example_lines(
    source: &SourceFile,
    block: &ruby_prism::BlockNode<'_>,
    count_comments: bool,
    count_as_one: &[String],
) -> usize {
    let body = match block.body() {
        Some(b) => b,
        None => return 0,
    };

    // Use body start offset (not block opening) to match RuboCop's `body.source.lines`.
    // For BeginNode (rescue/ensure), use statements().start_offset() to skip the
    // BeginNode's own location which points to the opening keyword.
    let body_start_offset = body
        .as_begin_node()
        .and_then(|b| b.statements())
        .map(|s| s.location().start_offset())
        .unwrap_or_else(|| body.location().start_offset());
    let (body_start_line, _) = source.offset_to_line_col(body_start_offset);

    // Effective start: shift back one line so count_body_lines_ex (which counts
    // from start_line+1) starts at body_start_line.
    let opening_offset = block.opening_loc().start_offset();
    let (opening_line, _) = source.offset_to_line_col(opening_offset);
    let effective_start_offset = if body_start_line > 1 {
        source
            .line_col_to_offset(body_start_line - 1, 0)
            .unwrap_or(opening_offset)
    } else {
        opening_offset
    };

    // Determine effective end offset.
    let closing_offset = block.closing_loc().start_offset();
    let (closing_line, _) = source.offset_to_line_col(closing_offset);

    let mut effective_end_offset = closing_offset;

    if body_has_heredoc(source, &body) {
        // When body contains heredocs, RuboCop's source_from_node_with_heredoc
        // computes end line as max across descendants (NOT the body root itself),
        // so the inner block's `end` keyword is excluded. Match that behavior.
        let max_line = max_descendant_end_line(source, &body);
        if max_line > 0 && max_line <= closing_line {
            if let Some(next_line_start) = source.line_col_to_offset(max_line + 1, 0) {
                effective_end_offset = next_line_start;
            }
        }
    } else if body.as_begin_node().is_none() {
        // For brace blocks: when body ends on same line as `}`, the body's last
        // line content must be included. RuboCop's `body.source.lines` includes it
        // but count_body_lines_ex excludes the end line. Extend to include it.
        let (body_end_line, _) =
            source.offset_to_line_col(body.location().end_offset().saturating_sub(1));
        if body_end_line == closing_line {
            if let Some(next_line_start) = source.line_col_to_offset(closing_line + 1, 0) {
                effective_end_offset = next_line_start;
            } else {
                effective_end_offset = block.closing_loc().end_offset();
            }
        }
    }

    // Collect foldable ranges from CountAsOne config.
    let mut all_foldable: Vec<(usize, usize)> = Vec::new();
    if !count_as_one.is_empty() {
        all_foldable.extend(collect_foldable_ranges(source, &body, count_as_one));
        if count_as_one.iter().any(|s| s == "heredoc") {
            all_foldable.extend(collect_heredoc_ranges(source, &body));
        }
    }
    all_foldable.sort();
    all_foldable.dedup();

    let mut count = count_body_lines_ex(
        source,
        effective_start_offset,
        effective_end_offset,
        count_comments,
        &all_foldable,
    );

    // When body_start_line == 1 AND opening is on line 1, count_body_lines_ex
    // starts from line 2, missing line 1 body content. Add 1 to compensate.
    if body_start_line == 1 && opening_line == 1 {
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
