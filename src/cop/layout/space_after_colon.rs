use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct SpaceAfterColon;

impl Cop for SpaceAfterColon {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterColon"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let assoc = match node.as_assoc_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // For shorthand hash syntax (key: value), the `:` is the closing
        // delimiter of the SymbolNode key, and operator_loc is None.
        // For rocket syntax (key => value), operator_loc is `=>`.
        // We only care about the shorthand `:` form.

        let key = assoc.key();
        let sym = match key.as_symbol_node() {
            Some(s) => s,
            None => return Vec::new(), // key is not a symbol (e.g., string key with =>)
        };

        let colon_loc = match sym.closing_loc() {
            Some(loc) if loc.as_slice() == b":" => loc,
            _ => return Vec::new(), // Not shorthand syntax
        };

        let bytes = source.as_bytes();
        let after_colon = colon_loc.end_offset();
        if bytes.get(after_colon) != Some(&b' ') {
            let (line, column) = source.offset_to_line_col(colon_loc.start_offset());
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line, column },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: "Space missing after colon.".to_string(),
            }];
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &SpaceAfterColon,
            include_bytes!("../../../testdata/cops/layout/space_after_colon/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &SpaceAfterColon,
            include_bytes!("../../../testdata/cops/layout/space_after_colon/no_offense.rb"),
        );
    }
}
