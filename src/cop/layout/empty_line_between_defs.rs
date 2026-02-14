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
    trimmed.starts_with(b"class ")
        || trimmed.starts_with(b"module ")
        || trimmed.starts_with(b"def ")
        || trimmed.starts_with(b"do")
        || trimmed.starts_with(b"begin")
        || trimmed.starts_with(b"{")
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
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let def_loc = def_node.def_keyword_loc();
        let (def_line, def_col) = source.offset_to_line_col(def_loc.start_offset());

        // Skip if def is on the first line
        if def_line <= 1 {
            return Vec::new();
        }

        let prev_line = match line_at(source, def_line - 1) {
            Some(l) => l,
            None => return Vec::new(),
        };

        // No offense if the previous line is blank
        if is_blank(prev_line) {
            return Vec::new();
        }

        // No offense if previous line is an opening keyword (class, module, def, do, begin, {)
        if is_opening_line(prev_line) {
            return Vec::new();
        }

        // No offense if previous line is a comment (typically a doc comment for this method)
        if is_comment_line(prev_line) {
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
