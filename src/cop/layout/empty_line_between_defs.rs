use crate::cop::util::line_at;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLineBetweenDefs;

fn is_blank(line: &[u8]) -> bool {
    line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r')
}

/// Check if a line is a scope-opening keyword line (class, module, def, do, begin, or `{`).
fn is_opening_line(line: &[u8]) -> bool {
    let trimmed: Vec<u8> = line
        .iter()
        .copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect();
    // Strip trailing whitespace for end-of-line checks
    let end_trimmed = trimmed
        .iter()
        .rposition(|&b| b != b' ' && b != b'\t' && b != b'\n' && b != b'\r')
        .map_or(&[] as &[u8], |i| &trimmed[..=i]);

    trimmed.starts_with(b"class ")
        || trimmed.starts_with(b"module ")
        || trimmed.starts_with(b"def ")
        || trimmed.starts_with(b"do")
        || trimmed.starts_with(b"begin")
        || trimmed.starts_with(b"{")
        // Lines ending with `do` or `do |...|` (block opening)
        || end_trimmed.ends_with(b" do")
        || end_trimmed.ends_with(b"|")
            && end_trimmed.windows(4).any(|w| w == b" do ")
}

/// Check if a line is a comment line.
fn is_comment_line(line: &[u8]) -> bool {
    let trimmed: Vec<u8> = line
        .iter()
        .copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect();
    trimmed.starts_with(b"#")
}

impl Cop for EmptyLineBetweenDefs {
    fn name(&self) -> &'static str {
        "Layout/EmptyLineBetweenDefs"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let def_loc = def_node.def_keyword_loc();
        let (def_line, def_col) = source.offset_to_line_col(def_loc.start_offset());

        // AllowAdjacentOneLineDefs: skip if this def is a single-line def
        // and the previous def-ending line is also from a single-line def
        let allow_adjacent = config.get_bool("AllowAdjacentOneLineDefs", true);
        if allow_adjacent {
            let end_line = def_node
                .end_keyword_loc()
                .map(|loc| source.offset_to_line_col(loc.start_offset()).0)
                .unwrap_or(def_line);
            if end_line == def_line {
                // This is a single-line def — skip it
                return Vec::new();
            }
        }

        // Skip if def is on the first line
        if def_line <= 1 {
            return Vec::new();
        }

        // Scan backwards from the def line to find the first non-blank, non-comment line.
        // Only flag if it's `end` (indicating consecutive method definitions).
        let mut check_line = def_line - 1; // 1-indexed
        loop {
            if check_line < 1 {
                return Vec::new();
            }
            let line = match line_at(source, check_line) {
                Some(l) => l,
                None => return Vec::new(),
            };
            if is_blank(line) {
                // Found blank line before reaching `end` — no offense
                return Vec::new();
            }
            if is_comment_line(line) {
                // Skip comment lines
                check_line -= 1;
                continue;
            }
            // Check if this is an opening line (class, module, def, etc.)
            if is_opening_line(line) {
                return Vec::new();
            }
            // Check if this line is `end` (with optional leading whitespace)
            let trimmed: Vec<u8> = line
                .iter()
                .copied()
                .skip_while(|&b| b == b' ' || b == b'\t')
                .collect();
            if trimmed == b"end" || trimmed.starts_with(b"end ") || trimmed.starts_with(b"end\t") {
                // Previous method ended here — flag the missing empty line
                break;
            }
            // Something else (e.g., LONG_DESC, attr_accessor, etc.) — don't flag
            return Vec::new();
        }

        vec![self.diagnostic(
            source,
            def_line,
            def_col,
            "Use empty lines between method definitions.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(EmptyLineBetweenDefs, "cops/layout/empty_line_between_defs");

    #[test]
    fn single_def_no_offense() {
        let src = b"class Foo\n  def bar\n    1\n  end\nend\n";
        let diags = run_cop_full(&EmptyLineBetweenDefs, src);
        assert!(diags.is_empty(), "Single def should not trigger offense");
    }

    #[test]
    fn def_after_end_without_blank_line() {
        let src = b"class Foo\n  def bar\n    1\n  end\n  def baz\n    2\n  end\nend\n";
        let diags = run_cop_full(&EmptyLineBetweenDefs, src);
        assert_eq!(diags.len(), 1, "Missing blank line between defs should trigger");
        assert_eq!(diags[0].location.line, 5);
    }
}
