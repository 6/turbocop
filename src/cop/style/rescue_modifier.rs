use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct RescueModifier;

impl Cop for RescueModifier {
    fn name(&self) -> &'static str {
        "Style/RescueModifier"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let rescue_mod = match node.as_rescue_modifier_node() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let kw_loc = rescue_mod.keyword_loc();
        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: Severity::Convention,
            cop_name: self.name().to_string(),
            message: "Avoid rescuing without specifying an error class.".to_string(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full, run_cop_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &RescueModifier,
            include_bytes!("../../../testdata/cops/style/rescue_modifier/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &RescueModifier,
            include_bytes!("../../../testdata/cops/style/rescue_modifier/no_offense.rb"),
        );
    }

    #[test]
    fn inline_rescue_fires() {
        let source = b"x = foo rescue nil\n";
        let diags = run_cop_full(&RescueModifier, source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Avoid rescuing"));
    }
}
