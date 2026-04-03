use crate::cop::shared::node_type::DEF_NODE;
use crate::cop::shared::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// FP investigation (2026-03-11): 574 FPs caused by `check_keyword_end_alignment`
/// counting only ASCII space bytes (0x20) for line indent, while `offset_to_line_col`
/// counts UTF-8 characters for `end` column. Tab-indented files had `line_indent=0`
/// but `end_col>0`, causing every multi-line def to fire. Fixed by using
/// `offset_to_line_col` on the first non-whitespace byte position (skipping both
/// spaces and tabs) instead of raw space counting. Also handles BOM correctly.
///
/// FP/FN investigation (2026-03-14): 1 FP and 3 FN caused by `start_of_line` mode
/// always aligning `end` with the first non-ws char on the `def` line. RuboCop's
/// `on_def` handler aligns `end` with the `def` keyword in BOTH modes; the
/// `start_of_line` vs `def` distinction only applies in `on_send` for modifier
/// methods (e.g., `private def foo`). Fixed by detecting whether the `def` is
/// preceded by a modifier-like prefix (identifiers + whitespace only) vs non-modifier
/// code (semicolons, operators, parens). Non-modifier mid-line defs now align `end`
/// with the `def` keyword. Cases: minified `class H<Hash;def ...` (FP), parenthesized
/// `protected (def bar ...` (FN), `false && def ...` (FN), `module X;def ...` (FN).
///
/// FP investigation (2026-03-25): 2 FPs from `foodcoops__foodsoft` caused by line
/// continuations before `def` (e.g., `helper_method \\\n  def foo`). The `def` appears
/// on its own line with only whitespace before it, so `is_modifier_prefix` returned
/// true and `check_keyword_end_alignment` aligned `end` with `def`'s line. But since
/// the previous line ends with `\`, the `def` is an argument to the method call on
/// the continuation line. RuboCop handles this in `on_send` via `def_modifier?`, which
/// recognizes the send node wrapping the def. Fixed by detecting line continuations
/// (`\` at end of previous line) and tracing back to the real statement start line,
/// then aligning `end` with that line's indentation.
pub struct DefEndAlignment;

/// Check if the text before `def` on the same line looks like a modifier method chain.
/// Modifier patterns: `private def`, `foo bar def`, `private_class_method def self.helper`.
/// Non-modifier patterns: `class H<Hash;def`, `false && def`, `protected (def`.
/// Returns true if the prefix contains only word characters (a-z, A-Z, 0-9, _) and whitespace.
fn is_modifier_prefix(source: &SourceFile, def_kw_offset: usize) -> bool {
    let bytes = source.as_bytes();
    // Find start of the line containing `def`
    let mut line_start = def_kw_offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    // Skip UTF-8 BOM if present at the very start of the file
    if line_start == 0 && bytes.len() >= 3 && bytes[0..3] == [0xEF, 0xBB, 0xBF] {
        line_start = 3;
    }
    // Check all bytes between line start and def keyword
    let prefix = &bytes[line_start..def_kw_offset];
    // If prefix is only whitespace, def starts the line — treat as modifier-compatible
    // (alignment with line start is correct since def IS at line start)
    if prefix.iter().all(|b| *b == b' ' || *b == b'\t') {
        return true;
    }
    // Check if prefix is only word chars + whitespace (modifier pattern)
    prefix
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b' ' || *b == b'\t')
}

/// Find the offset of the real statement start when line continuations (`\`) are involved.
/// If the line before `def` ends with `\`, the `def` is part of a larger expression that
/// started on an earlier line (e.g., `helper_method \\\n  def foo`). This traces back
/// through any chain of continuation lines and returns the offset of the first line's
/// first non-whitespace character. Returns `None` if there is no line continuation.
fn find_continuation_start(source: &SourceFile, def_kw_offset: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    // Find start of the line containing `def`
    let mut line_start = def_kw_offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    // If def is on the first line, no previous line to check
    if line_start == 0 {
        return None;
    }
    // Check if the previous line ends with `\` (before the newline at line_start - 1)
    // line_start - 1 is the `\n` of the previous line
    let prev_newline = line_start - 1;
    if prev_newline == 0 || bytes[prev_newline - 1] != b'\\' {
        return None;
    }
    // Found a continuation. Trace back through any further continuation lines.
    let mut cur_line_start = line_start;
    loop {
        // Find start of the previous line
        let prev_nl = cur_line_start - 1; // the \n
        let mut prev_line_start = if prev_nl > 0 { prev_nl } else { 0 };
        while prev_line_start > 0 && bytes[prev_line_start - 1] != b'\n' {
            prev_line_start -= 1;
        }
        cur_line_start = prev_line_start;
        // Check if this line's predecessor also ends with `\`
        if prev_line_start == 0 {
            break; // first line in file, done
        }
        let prev_prev_nl = prev_line_start - 1;
        if prev_prev_nl == 0 || bytes[prev_prev_nl - 1] != b'\\' {
            break; // no more continuations
        }
    }
    // cur_line_start is the start of the first line in the continuation chain
    // Find first non-whitespace on that line
    let mut pos = cur_line_start;
    while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
        pos += 1;
    }
    Some(pos)
}

impl Cop for DefEndAlignment {
    fn name(&self) -> &'static str {
        "Layout/DefEndAlignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
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
        let style = config.get_str("EnforcedStyleAlignWith", "start_of_line");
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Skip endless methods (no end keyword)
        let end_kw_loc = match def_node.end_keyword_loc() {
            Some(loc) => loc,
            None => return,
        };

        // Skip single-line defs (e.g., `def foo; 42; end`)
        let def_kw_offset = def_node.def_keyword_loc().start_offset();
        let (def_line, _) = source.offset_to_line_col(def_kw_offset);
        let (end_line, end_col) = source.offset_to_line_col(end_kw_loc.start_offset());
        if def_line == end_line {
            return;
        }

        match style {
            "def" => {
                // Align `end` with `def` keyword
                let (_, def_col) = source.offset_to_line_col(def_kw_offset);
                if end_col != def_col {
                    diagnostics.push(self.diagnostic(
                        source,
                        end_line,
                        end_col,
                        "Align `end` with `def`.".to_string(),
                    ));
                }
            }
            _ => {
                // "start_of_line" (default): RuboCop's on_def always aligns end with
                // the def keyword. The start_of_line vs def distinction only applies
                // in on_send for modifier methods. Detect modifier prefixes (e.g.,
                // `private def`) and align with line start; for non-modifier mid-line
                // defs (e.g., `class X;def` or `false && def`), align with def keyword.
                //
                // Line continuations: `helper_method \\\n  def foo` means `def` is part
                // of a method call expression. Align `end` with the statement start
                // (the first line of the continuation chain), not the `def` line.
                if let Some(stmt_start_offset) = find_continuation_start(source, def_kw_offset) {
                    diagnostics.extend(util::check_keyword_end_alignment(
                        self.name(),
                        source,
                        "def",
                        stmt_start_offset,
                        end_kw_loc.start_offset(),
                    ));
                } else if is_modifier_prefix(source, def_kw_offset) {
                    diagnostics.extend(util::check_keyword_end_alignment(
                        self.name(),
                        source,
                        "def",
                        def_kw_offset,
                        end_kw_loc.start_offset(),
                    ));
                } else {
                    // Non-modifier mid-line def: align end with def keyword
                    let (_, def_col) = source.offset_to_line_col(def_kw_offset);
                    if end_col != def_col {
                        diagnostics.push(self.diagnostic(
                            source,
                            end_line,
                            end_col,
                            "Align `end` with `def`.".to_string(),
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(DefEndAlignment, "cops/layout/def_end_alignment");

    #[test]
    fn endless_method_no_offense() {
        let source = b"def foo = 42\n";
        let diags = run_cop_full(&DefEndAlignment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn tab_indented_def_no_offense() {
        // Tab-indented def: end aligned with def via tabs
        let source = b"\tdef foo\n\t\t42\n\tend\n";
        let diags = run_cop_full(&DefEndAlignment, source);
        assert!(
            diags.is_empty(),
            "tab-indented def should not fire: {:?}",
            diags
        );
    }

    #[test]
    fn tab_indented_modifier_def_no_offense() {
        // Tab-indented modifier def: end aligned with private via tabs
        let source = b"\tprivate def foo\n\t\t42\n\tend\n";
        let diags = run_cop_full(&DefEndAlignment, source);
        assert!(
            diags.is_empty(),
            "tab-indented modifier def should not fire: {:?}",
            diags
        );
    }

    #[test]
    fn bom_prefix_no_offense() {
        // UTF-8 BOM before def: end at column 0 should not fire
        let source = b"\xef\xbb\xbfdef foo\n  42\nend\n";
        let diags = run_cop_full(&DefEndAlignment, source);
        assert!(
            diags.is_empty(),
            "BOM-prefixed def should not fire: {:?}",
            diags
        );
    }

    #[test]
    fn def_style_aligns_with_def_keyword() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyleAlignWith".into(),
                serde_yml::Value::String("def".into()),
            )]),
            ..CopConfig::default()
        };
        // `end` aligned with `def` (both at column 2)
        let src = b"  def foo\n    42\n  end\n";
        let diags = run_cop_full_with_config(&DefEndAlignment, src, config.clone());
        assert!(
            diags.is_empty(),
            "def style should accept end aligned with def"
        );

        // `end` at column 0, `def` at column 2 → mismatch
        let src2 = b"  def foo\n    42\nend\n";
        let diags2 = run_cop_full_with_config(&DefEndAlignment, src2, config);
        assert_eq!(
            diags2.len(),
            1,
            "def style should flag end not aligned with def"
        );
    }

    #[test]
    fn line_continuation_before_def_no_offense() {
        // `helper_method \` followed by `def` on next line: end aligns with helper_method
        let source = b"  helper_method \\\n    def foo\n    42\n  end\n";
        let diags = run_cop_full(&DefEndAlignment, source);
        assert!(
            diags.is_empty(),
            "line continuation def should not fire when end aligns with statement start: {:?}",
            diags
        );
    }

    #[test]
    fn line_continuation_before_def_misaligned_offense() {
        // `helper_method \` followed by `def`: end does NOT align with helper_method
        let source = b"  helper_method \\\n    def foo\n    42\n      end\n";
        let diags = run_cop_full(&DefEndAlignment, source);
        assert_eq!(
            diags.len(),
            1,
            "line continuation def should fire when end misaligned with statement start"
        );
    }
}
