use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DefEndAlignment;

impl Cop for DefEndAlignment {
    fn name(&self) -> &'static str {
        "Layout/DefEndAlignment"
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

        // Skip endless methods (no end keyword)
        let end_kw_loc = match def_node.end_keyword_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        // Skip single-line defs (e.g., `def foo; 42; end`)
        let (def_line, _) = source.offset_to_line_col(def_node.def_keyword_loc().start_offset());
        let (end_line, _) = source.offset_to_line_col(end_kw_loc.start_offset());
        if def_line == end_line {
            return Vec::new();
        }

        util::check_keyword_end_alignment(
            self.name(),
            source,
            "def",
            def_node.def_keyword_loc().start_offset(),
            end_kw_loc.start_offset(),
        )
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
}
