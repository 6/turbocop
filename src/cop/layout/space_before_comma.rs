use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// ## Corpus investigation (2026-03-28)
///
/// CI reported FP=2, FN=1 for this cop.
///
/// Reproduced locally:
/// - FP: a comma that starts a continued argument line (`\n        , env: env`)
///   was incorrectly flagged because the raw byte scan treated indentation as
///   whitespace before the comma. Fixed by skipping whitespace runs that reach
///   the start of the line, matching RuboCop's behavior for leading commas.
/// - FP: RuboCop accepts space before a comma after the closing quote of a
///   line-continued string literal that spans 3+ physical lines, such as the
///   `ruby-gnome` sample. Nitrocop flagged the raw `" ,` sequence. Fixed by
///   collecting closing-quote offsets for those multiline non-heredoc strings
///   and skipping only that narrow context. A 2-line continuation still
///   registers an offense in RuboCop, so the skip stays limited to 3+ lines.
/// - FN: a comma inside `#{}` within heredoc content
///   (`#{response.body[[0, n - 200].max , 400]}`) was missed because the cop
///   only considered `code_map.is_code(i)`. Fixed by also scanning comma bytes
///   inside heredoc interpolation, excluding nested non-code literals there.
/// ## Corpus investigation (2026-03-10)
///
/// CI baseline reported FP=0, FN=7.
///
/// Fixed the remaining syntax gap in command-form calls and autocorrect:
/// RuboCop removes the entire whitespace span before `,`, but nitrocop only
/// removed one ASCII space. The accepted fix now trims the full contiguous
/// space/tab run before the comma, which covers cases like `break  1  , 2`
/// as well as the sampled `break`/`next`/`yield` forms from `rufo`.
///
/// Acceptance gate after this patch (`scripts/check-cop.py --verbose --rerun`):
/// expected=3,134, actual=3,162, CI baseline=3,127, raw excess=28,
/// missing=0, file-drop noise=103. The rerun passes against the CI baseline
/// once that existing parser-crash noise is applied.
///
/// ## FP fix: character literal `?\ ,` (2026-03-25)
///
/// All 5 FPs were from Ruby character literals using escaped space (`?\ `).
/// In `?\ ,`, the space is part of the character literal, not whitespace
/// before the comma. Fixed by checking for `?\` immediately before the
/// whitespace run and skipping those offenses.
pub struct SpaceBeforeComma;

fn whitespace_before_comma_start(bytes: &[u8], comma_offset: usize) -> Option<usize> {
    let mut start = comma_offset;
    while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
        start -= 1;
    }

    (start < comma_offset).then_some(start)
}

fn comma_is_code(code_map: &CodeMap, offset: usize) -> bool {
    code_map.is_code(offset)
        || (code_map.is_heredoc_interpolation(offset)
            && !code_map.is_non_code_in_heredoc_interpolation(offset))
}

struct MultilineStringClosingCollector<'a> {
    source: &'a SourceFile,
    offsets: Vec<usize>,
}

impl<'pr> Visit<'pr> for MultilineStringClosingCollector<'_> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.collect(&node);
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.collect(&node);
    }
}

impl MultilineStringClosingCollector<'_> {
    fn collect(&mut self, node: &ruby_prism::Node<'_>) {
        match node {
            ruby_prism::Node::StringNode { .. } => {
                let string = node.as_string_node().unwrap();
                self.collect_string(node.location(), string.opening_loc(), string.closing_loc());
            }
            ruby_prism::Node::InterpolatedStringNode { .. } => {
                let string = node.as_interpolated_string_node().unwrap();
                self.collect_string(node.location(), string.opening_loc(), string.closing_loc());
            }
            _ => {}
        }
    }

    fn collect_string(
        &mut self,
        loc: ruby_prism::Location<'_>,
        opening: Option<ruby_prism::Location<'_>>,
        closing: Option<ruby_prism::Location<'_>>,
    ) {
        let (Some(open), Some(close)) = (opening, closing) else {
            return;
        };

        if open.as_slice().starts_with(b"<<") {
            return;
        }

        let start_line = self.source.offset_to_line_col(loc.start_offset()).0;
        let end_line = self
            .source
            .offset_to_line_col(loc.end_offset().saturating_sub(1))
            .0;

        if end_line >= start_line + 2 {
            self.offsets.push(close.start_offset());
        }
    }
}

fn multiline_string_closing_offsets(
    source: &SourceFile,
    parse_result: &ruby_prism::ParseResult<'_>,
) -> Vec<usize> {
    let mut collector = MultilineStringClosingCollector {
        source,
        offsets: Vec::new(),
    };
    collector.visit(&parse_result.node());
    collector.offsets.sort_unstable();
    collector.offsets.dedup();
    collector.offsets
}

impl Cop for SpaceBeforeComma {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeComma"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let bytes = source.as_bytes();
        let multiline_string_closing_offsets =
            multiline_string_closing_offsets(source, parse_result);
        for (i, &byte) in bytes.iter().enumerate() {
            if byte != b',' || !comma_is_code(code_map, i) {
                continue;
            }

            let Some(start) = whitespace_before_comma_start(bytes, i) else {
                continue;
            };

            let line_start = bytes[..i]
                .iter()
                .rposition(|&b| b == b'\n')
                .map_or(0, |idx| idx + 1);

            // A comma that starts the continued line should not inherit this
            // line's indentation as "space before comma".
            if start == line_start {
                continue;
            }

            if start > 0
                && multiline_string_closing_offsets
                    .binary_search(&(start - 1))
                    .is_ok()
            {
                continue;
            }

            // Skip Ruby character literal for escaped space: ?\ ,
            if start >= 2 && bytes[start - 1] == b'\\' && bytes[start - 2] == b'?' {
                continue;
            }

            let (line, column) = source.offset_to_line_col(start);
            let mut diag = self.diagnostic(
                source,
                line,
                column,
                "Space found before comma.".to_string(),
            );
            if let Some(ref mut corr) = corrections {
                corr.push(crate::correction::Correction {
                    start,
                    end: i,
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
            diagnostics.push(diag);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceBeforeComma, "cops/layout/space_before_comma");
    crate::cop_autocorrect_fixture_tests!(SpaceBeforeComma, "cops/layout/space_before_comma");

    #[test]
    fn autocorrect_remove_space() {
        let input = b"foo(1 , 2)\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&SpaceBeforeComma, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"foo(1, 2)\n");
    }

    #[test]
    fn autocorrect_multiple() {
        let input = b"foo(1 , 2 , 3)\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&SpaceBeforeComma, input);
        assert_eq!(corrections.len(), 2);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"foo(1, 2, 3)\n");
    }

    #[test]
    fn detects_tarantula_heredoc_interpolation_context() {
        let source = br#"def handle(response, regexp, result)
  if n = (response.body =~ /#{regexp}/)
    error_result = result.dup
    error_result.success = false
    error_result.description = "XSS error found, match was: #{h($1)}"
    error_result.data = <<-STR
  ########################################################################
  # Text around unescaped string: #{$1}
  ########################################################################
    #{response.body[[0, n - 200].max , 400]}





  ########################################################################
  # Attack information:
STR
  end
end
"#;

        let parse_result = crate::parse::parse_source(source);
        assert_eq!(parse_result.errors().count(), 0);
        let code_map = crate::parse::codemap::CodeMap::from_parse_result(source, &parse_result);
        let marker = b".max , 400]}";
        let comma = source
            .windows(marker.len())
            .position(|window| window == marker)
            .map(|start| start + 5)
            .unwrap();
        assert!(
            code_map.is_heredoc_interpolation(comma),
            "comma should be inside heredoc interpolation"
        );
        assert!(
            !code_map.is_non_code_in_heredoc_interpolation(comma),
            "comma should not be nested non-code inside heredoc interpolation"
        );

        let diags = crate::testutil::run_cop_full(&SpaceBeforeComma, source);
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert_eq!(diags[0].location.line, 10);
    }
}
