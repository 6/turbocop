use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
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

        let def_kw_loc = def_node.def_keyword_loc();
        let (_, def_col) = source.offset_to_line_col(def_kw_loc.start_offset());
        let (end_line, end_col) = source.offset_to_line_col(end_kw_loc.start_offset());

        if end_col != def_col {
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location {
                    line: end_line,
                    column: end_col,
                },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: "Align `end` with `def`.".to_string(),
            }];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full, run_cop_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &DefEndAlignment,
            include_bytes!("../../../testdata/cops/layout/def_end_alignment/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &DefEndAlignment,
            include_bytes!("../../../testdata/cops/layout/def_end_alignment/no_offense.rb"),
        );
    }

    #[test]
    fn endless_method_no_offense() {
        let source = b"def foo = 42\n";
        let diags = run_cop_full(&DefEndAlignment, source);
        assert!(diags.is_empty());
    }
}
