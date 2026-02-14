use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct SymbolArray;

impl Cop for SymbolArray {
    fn name(&self) -> &'static str {
        "Style/SymbolArray"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let array_node = match node.as_array_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Must have `[` opening (not %i or %I)
        let opening = match array_node.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        if opening.as_slice() != b"[" {
            return Vec::new();
        }

        let elements = array_node.elements();
        let min_size: usize = config
            .options
            .get("MinSize")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(2);

        if elements.len() < min_size {
            return Vec::new();
        }

        // All elements must be symbol nodes
        for elem in elements.iter() {
            if elem.as_symbol_node().is_none() {
                return Vec::new();
            }
        }

        let (line, column) = source.offset_to_line_col(opening.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: Severity::Convention,
            cop_name: self.name().to_string(),
            message: "Use `%i` or `%I` for an array of symbols.".to_string(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &SymbolArray,
            include_bytes!("../../../testdata/cops/style/symbol_array/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &SymbolArray,
            include_bytes!("../../../testdata/cops/style/symbol_array/no_offense.rb"),
        );
    }
}
