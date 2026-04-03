use crate::cop::shared::node_type::RANGE_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=6, FN=0.
///
/// FP=6 root cause: multiline range literals such as `0..\n  10` were treated
/// as having spaces after the operator because the implementation counted the
/// indentation on the following line. RuboCop normalizes an immediate line break
/// after the operator before checking for interior spacing.
///
/// Fix: ignore a pure newline-plus-indentation gap immediately after the range
/// operator, but still collapse that gap during autocorrect when another real
/// spacing offense exists in the same range expression.
///
/// Rerun outcome: removed the target false positives from `markdownlint` (3),
/// `ruby-lsp` (2), and one `natalie` case. Aggregate corpus totals remain noisy:
/// `jruby` adds +33 offenses from RuboCop file-drop noise, and local reruns no
/// longer reproduce two baseline offenses in `peritor`, which suggests corpus
/// drift or repo-local execution differences rather than a confirmed new cop bug.
///
/// ## Corpus investigation (2026-03-28)
///
/// Corpus oracle reported FP=0, FN=1 in `jjyg__metasm__a70271c`
/// (`samples/lindebug.rb:626`): `when ?\ ..?~`.
///
/// Root cause: Prism reports the left operand location as `"?\\ "`, so the
/// trailing space character belongs to the operand token itself and there is no
/// byte gap between `left.end_offset()` and `operator_loc.start_offset()`.
/// RuboCop still flags this because it checks whitespace directly adjacent to
/// the operator in `node.source`, not only the inter-node gap.
///
/// Fix: keep the existing gap-based checks, but add a fallback that treats a
/// trailing horizontal-whitespace run immediately before the operator as an
/// interior-space offense even when that whitespace is part of the left token.
pub struct SpaceInsideRangeLiteral;

impl Cop for SpaceInsideRangeLiteral {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideRangeLiteral"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[RANGE_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Check both inclusive (..) and exclusive (...) ranges
        let (left, right, op_loc) = if let Some(range) = node.as_range_node() {
            (range.left(), range.right(), range.operator_loc())
        } else {
            return;
        };

        let bytes = source.as_bytes();
        let op_start = op_loc.start_offset();
        let op_end = op_loc.end_offset();

        let mut has_space = false;
        let mut space_before_range: Option<(usize, usize)> = None;
        let mut space_after_range: Option<(usize, usize)> = None;
        let mut multiline_gap_after_range: Option<(usize, usize)> = None;

        // Check space before operator
        if let Some(left_node) = left {
            let left_end = left_node.location().end_offset();
            if op_start > left_end {
                let between = &bytes[left_end..op_start];
                if between.iter().any(|&b| b == b' ' || b == b'\t') {
                    has_space = true;
                    space_before_range = Some((left_end, op_start));
                }
            } else if let Some(start) =
                horizontal_whitespace_run_before(bytes, node.location().start_offset(), op_start)
            {
                has_space = true;
                space_before_range = Some((start, op_start));
            }
        }

        // Check space after operator
        if let Some(right_node) = right {
            let right_start = right_node.location().start_offset();
            if right_start > op_end {
                let between = &bytes[op_end..right_start];
                if is_pure_multiline_gap(between) {
                    multiline_gap_after_range = Some((op_end, right_start));
                } else if between.iter().any(|&b| b == b' ' || b == b'\t') {
                    has_space = true;
                    space_after_range = Some((op_end, right_start));
                }
            }
        }

        if has_space {
            let (line, col) = source.offset_to_line_col(node.location().start_offset());
            let mut diag =
                self.diagnostic(source, line, col, "Space inside range literal.".to_string());
            if let Some(ref mut corr) = corrections {
                if let Some((start, end)) = space_before_range {
                    corr.push(crate::correction::Correction {
                        start,
                        end,
                        replacement: String::new(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                }
                if let Some((start, end)) = space_after_range {
                    corr.push(crate::correction::Correction {
                        start,
                        end,
                        replacement: String::new(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                }
                if let Some((start, end)) = multiline_gap_after_range
                    .filter(|_| space_before_range.is_some() || space_after_range.is_some())
                {
                    corr.push(crate::correction::Correction {
                        start,
                        end,
                        replacement: String::new(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                }
                diag.corrected = true;
            }
            diagnostics.push(diag);
        }
    }
}

fn is_pure_multiline_gap(bytes: &[u8]) -> bool {
    if let Some(rest) = bytes.strip_prefix(b"\r\n") {
        return rest.iter().all(|&b| b == b' ' || b == b'\t');
    }

    if let Some(rest) = bytes.strip_prefix(b"\n") {
        return rest.iter().all(|&b| b == b' ' || b == b'\t');
    }

    false
}

fn horizontal_whitespace_run_before(
    bytes: &[u8],
    lower_bound: usize,
    upper_bound: usize,
) -> Option<usize> {
    if upper_bound == 0 || upper_bound <= lower_bound {
        return None;
    }

    let mut start = upper_bound;
    while start > lower_bound && matches!(bytes[start - 1], b' ' | b'\t') {
        start -= 1;
    }

    (start < upper_bound).then_some(start)
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SpaceInsideRangeLiteral,
        "cops/layout/space_inside_range_literal"
    );
    crate::cop_autocorrect_fixture_tests!(
        SpaceInsideRangeLiteral,
        "cops/layout/space_inside_range_literal"
    );

    #[test]
    fn ignores_pure_multiline_gap_after_operator() {
        crate::testutil::assert_cop_no_offenses_full(&SpaceInsideRangeLiteral, b"x = 0..\n  10\n");
    }
}
