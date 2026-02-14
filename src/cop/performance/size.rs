use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct Size;

impl Cop for Size {
    fn name(&self) -> &'static str {
        "Performance/Size"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"count" {
            return Vec::new();
        }

        // Must have a receiver (bare `count` is not what we're looking for)
        if call.receiver().is_none() {
            return Vec::new();
        }

        // Must have no arguments and no block
        if call.arguments().is_some() || call.block().is_some() {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: "Use `size` instead of `count`.".to_string(),
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
            &Size,
            include_bytes!("../../../testdata/cops/performance/size/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &Size,
            include_bytes!("../../../testdata/cops/performance/size/no_offense.rb"),
        );
    }
}
