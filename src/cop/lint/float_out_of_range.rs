use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct FloatOutOfRange;

impl Cop for FloatOutOfRange {
    fn name(&self) -> &'static str {
        "Lint/FloatOutOfRange"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let float_node = match node.as_float_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let loc = float_node.location();
        let src = loc.as_slice();

        // Remove underscores and parse as f64
        let cleaned: Vec<u8> = src.iter().copied().filter(|&b| b != b'_').collect();
        let text = match std::str::from_utf8(&cleaned) {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        match text.parse::<f64>() {
            Ok(val) if val.is_infinite() => {
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: self.default_severity(),
                    cop_name: self.name().to_string(),
                    message: "Float out of range.".to_string(),
                }]
            }
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &FloatOutOfRange,
            include_bytes!("../../../testdata/cops/lint/float_out_of_range/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &FloatOutOfRange,
            include_bytes!("../../../testdata/cops/lint/float_out_of_range/no_offense.rb"),
        );
    }
}
