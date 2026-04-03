use crate::cop::shared::node_type::DEF_NODE;
use crate::cop::shared::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Layout/EmptyLinesAroundMethodBody
///
/// Flags extra empty lines at the beginning/end of a method body.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=0, FN=1.
///
/// FP=0: no corpus false positives are currently known.
///
/// FN=1: multiline method signatures without parentheses still anchored the
/// beginning check at the `def` line, so a blank line after a continued
/// signature like `def fetch uri,\n        method = :get` was missed. This cop
/// now anchors regular methods at the multiline `)` when present, otherwise at
/// the last parameter line when the signature spans lines without explicit
/// parentheses. Earlier fixes in this cop already covered whitespace-only lines
/// (not offenses) and multiline endless methods by using the `=` line as the
/// body-start anchor.
///
/// ## Corpus investigation (2026-03-30)
///
/// FN=5: empty multiline method definitions like
/// `def self.foo(a,\n             b) end` were still missed when followed by a
/// blank line. The shared body helper exits when the computed body-start line
/// equals the `end` line, which is correct for single-line defs but wrong for
/// multiline empty defs whose signature and `end` share the last line. This
/// cop now special-cases that narrow shape and treats the blank line
/// immediately after the definition as a method-body-beginning offense,
/// matching RuboCop.
pub struct EmptyLinesAroundMethodBody;

impl Cop for EmptyLinesAroundMethodBody {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundMethodBody"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
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
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        if let Some(end_loc) = def_node.end_keyword_loc() {
            // Regular method (has `end` keyword)
            let keyword_offset = regular_method_body_start_offset(source, &def_node);

            diagnostics.extend(util::check_empty_lines_around_body_with_corrections(
                self.name(),
                source,
                keyword_offset,
                end_loc.start_offset(),
                "method",
                corrections.as_deref_mut(),
            ));

            diagnostics.extend(check_empty_line_after_empty_multiline_method_definition(
                self.name(),
                source,
                &def_node,
                keyword_offset,
                end_loc.start_offset(),
                corrections.as_deref_mut(),
            ));
        } else if let Some(equal_loc) = def_node.equal_loc() {
            // Endless method (`def foo = expr`)
            // Only check for blank line after `=` (no `end` keyword, so no end check)
            let equal_offset = equal_loc.start_offset();
            let (equal_line, _) = source.offset_to_line_col(equal_offset);

            // Find the last line of the body to use as end boundary
            let body_end = def_node.location().end_offset();
            let (body_end_line, _) = source.offset_to_line_col(body_end);

            // Skip if everything is on the same line as `=`
            if body_end_line <= equal_line {
                return;
            }

            // Check for blank line after `=`
            let after_equal = equal_line + 1;
            if let Some(line) = util::line_at(source, after_equal) {
                if util::is_blank_line(line) && after_equal < body_end_line {
                    let mut diag = Diagnostic {
                        path: source.path_str().to_string(),
                        location: crate::diagnostic::Location {
                            line: after_equal,
                            column: 0,
                        },
                        severity: crate::diagnostic::Severity::Convention,
                        cop_name: self.name().to_string(),
                        message: "Extra empty line detected at method body beginning.".to_string(),
                        corrected: false,
                    };
                    if let Some(ref mut corr) = corrections {
                        if let (Some(start), Some(end)) = (
                            source.line_col_to_offset(after_equal, 0),
                            source.line_col_to_offset(after_equal + 1, 0),
                        ) {
                            corr.push(crate::correction::Correction {
                                start,
                                end,
                                replacement: String::new(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                    }
                    diagnostics.push(diag);
                }
            }
        }
    }
}

fn check_empty_line_after_empty_multiline_method_definition(
    cop_name: &'static str,
    source: &SourceFile,
    def_node: &ruby_prism::DefNode<'_>,
    body_start_offset: usize,
    end_offset: usize,
    mut corrections: Option<&mut Vec<crate::correction::Correction>>,
) -> Vec<Diagnostic> {
    if def_node.body().is_some() {
        return Vec::new();
    }

    let def_offset = def_node.def_keyword_loc().start_offset();
    let (def_line, _) = source.offset_to_line_col(def_offset);
    let (body_start_line, _) = source.offset_to_line_col(body_start_offset);
    let (end_line, _) = source.offset_to_line_col(end_offset);

    // RuboCop still treats a blank line after `def foo(\n  bar) end` as being at
    // the method body beginning, but only when the multiline signature and the
    // `end` share the same final line.
    if body_start_line != end_line || body_start_line == def_line {
        return Vec::new();
    }

    let blank_line = end_line + 1;
    let Some(line) = util::line_at(source, blank_line) else {
        return Vec::new();
    };
    if !util::is_blank_line(line) {
        return Vec::new();
    }

    let mut diag = Diagnostic {
        path: source.path_str().to_string(),
        location: crate::diagnostic::Location {
            line: blank_line,
            column: 0,
        },
        severity: crate::diagnostic::Severity::Convention,
        cop_name: cop_name.to_string(),
        message: "Extra empty line detected at method body beginning.".to_string(),
        corrected: false,
    };
    if let Some(ref mut corr) = corrections {
        if let Some(start) = source.line_col_to_offset(blank_line, 0) {
            let end = source
                .line_col_to_offset(blank_line + 1, 0)
                .unwrap_or(source.content.len());
            corr.push(crate::correction::Correction {
                start,
                end,
                replacement: String::new(),
                cop_name,
                cop_index: 0,
            });
            diag.corrected = true;
        }
    }

    vec![diag]
}

fn regular_method_body_start_offset(
    source: &SourceFile,
    def_node: &ruby_prism::DefNode<'_>,
) -> usize {
    let def_offset = def_node.def_keyword_loc().start_offset();
    let (def_line, _) = source.offset_to_line_col(def_offset);

    if let Some(rparen) = def_node.rparen_loc() {
        let (rparen_line, _) = source.offset_to_line_col(rparen.start_offset());
        if rparen_line > def_line {
            return rparen.start_offset();
        }
    }

    let Some(params) = def_node.parameters() else {
        return def_offset;
    };

    let params_end_offset = params.location().end_offset().saturating_sub(1);
    let (params_end_line, _) = source.offset_to_line_col(params_end_offset);
    if params_end_line > def_line {
        params_end_offset
    } else {
        def_offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        EmptyLinesAroundMethodBody,
        "cops/layout/empty_lines_around_method_body"
    );
    crate::cop_autocorrect_fixture_tests!(
        EmptyLinesAroundMethodBody,
        "cops/layout/empty_lines_around_method_body"
    );

    #[test]
    fn single_line_def_no_offense() {
        let src = b"def foo; 42; end\n";
        let diags = run_cop_full(&EmptyLinesAroundMethodBody, src);
        assert!(diags.is_empty(), "Single-line def should not trigger");
    }

    #[test]
    fn endless_method_no_offense() {
        let src = b"def foo = 42\n";
        let diags = run_cop_full(&EmptyLinesAroundMethodBody, src);
        assert!(diags.is_empty(), "Endless method should not trigger");
    }

    #[test]
    fn whitespace_only_line_no_offense() {
        // RuboCop does NOT flag lines with trailing whitespace — only truly empty lines.
        // Lines with spaces/tabs are handled by Layout/TrailingWhitespace.
        let src = b"def some_method\n  \n  do_something\nend\n";
        let diags = run_cop_full(&EmptyLinesAroundMethodBody, src);
        assert!(
            diags.is_empty(),
            "Whitespace-only line should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn whitespace_only_line_at_end_no_offense() {
        let src = b"def some_method\n  do_something\n  \nend\n";
        let diags = run_cop_full(&EmptyLinesAroundMethodBody, src);
        assert!(
            diags.is_empty(),
            "Whitespace-only line at end should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn multiline_arg_def_blank_after_rparen() {
        let src = b"def some_method(\n  arg\n)\n\n  do_something\nend\n";
        let diags = run_cop_full(&EmptyLinesAroundMethodBody, src);
        assert_eq!(diags.len(), 1, "Should flag blank line after ): {diags:?}");
        assert!(diags[0].message.contains("beginning"));
    }

    #[test]
    fn multiline_arg_def_without_parens_blank_after_last_arg() {
        let src = b"def fetch uri,\n          method = :get\n\n  build_request(uri, method)\nend\n";
        let diags = run_cop_full(&EmptyLinesAroundMethodBody, src);
        assert_eq!(
            diags.len(),
            1,
            "Should flag blank line after continued def signature: {diags:?}"
        );
        assert!(diags[0].message.contains("beginning"));
    }

    #[test]
    fn multiline_empty_def_blank_after_end() {
        let src = b"def self.foo(a,\n             b) end\n\n# comment\n";
        let diags = run_cop_full(&EmptyLinesAroundMethodBody, src);
        assert_eq!(
            diags.len(),
            1,
            "Should flag blank line after empty multiline def: {diags:?}"
        );
        assert!(diags[0].message.contains("beginning"));
    }

    #[test]
    fn multiline_empty_def_no_blank_no_offense() {
        let src = b"def self.foo(a,\n             b) end\n# comment\n";
        let diags = run_cop_full(&EmptyLinesAroundMethodBody, src);
        assert!(
            diags.is_empty(),
            "Empty multiline def without blank should not trigger: {diags:?}"
        );
    }

    #[test]
    fn endless_method_multiline_blank_after_equal() {
        let src = b"def compute(value,\n  factor) =\n\n  value * factor\n";
        let diags = run_cop_full(&EmptyLinesAroundMethodBody, src);
        assert_eq!(
            diags.len(),
            1,
            "Should flag blank line after = in endless method: {diags:?}"
        );
        assert!(diags[0].message.contains("beginning"));
    }

    #[test]
    fn endless_method_no_blank_no_offense() {
        let src = b"def compute(value,\n  factor) =\n  value * factor\n";
        let diags = run_cop_full(&EmptyLinesAroundMethodBody, src);
        assert!(
            diags.is_empty(),
            "Endless method without blank should not trigger: {diags:?}"
        );
    }
}
