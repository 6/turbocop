use crate::cop::util::preceding_comment_line;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct Documentation;

impl Cop for Documentation {
    fn name(&self) -> &'static str {
        "Style/Documentation"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        if let Some(class_node) = node.as_class_node() {
            let kw_loc = class_node.class_keyword_loc();
            let start = kw_loc.start_offset();
            if !preceding_comment_line(source, start) {
                let (line, column) = source.offset_to_line_col(start);
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Missing top-level documentation comment for `class`.".to_string(),
                }];
            }
        } else if let Some(module_node) = node.as_module_node() {
            let kw_loc = module_node.module_keyword_loc();
            let start = kw_loc.start_offset();
            if !preceding_comment_line(source, start) {
                let (line, column) = source.offset_to_line_col(start);
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Missing top-level documentation comment for `module`.".to_string(),
                }];
            }
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
            &Documentation,
            include_bytes!("../../../testdata/cops/style/documentation/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &Documentation,
            include_bytes!("../../../testdata/cops/style/documentation/no_offense.rb"),
        );
    }

    #[test]
    fn first_line_class_has_no_preceding_comment() {
        let source = b"class Foo\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("class"));
    }

    #[test]
    fn module_without_comment() {
        let source = b"module Bar\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("module"));
    }
}
