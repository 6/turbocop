use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantBlockCall;

impl Cop for RedundantBlockCall {
    fn name(&self) -> &'static str {
        "Performance/RedundantBlockCall"
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

        if call.name().as_slice() != b"call" {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Check if receiver is a local variable named `block`
        let is_block = if let Some(local_var) = receiver.as_local_variable_read_node() {
            local_var.name().as_slice() == b"block"
        } else if let Some(recv_call) = receiver.as_call_node() {
            // In a bare context, `block` may be parsed as a method call
            recv_call.name().as_slice() == b"block"
                && recv_call.receiver().is_none()
                && recv_call.arguments().is_none()
        } else {
            false
        };

        if !is_block {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: "Use `yield` instead of `block.call`.".to_string(),
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
            &RedundantBlockCall,
            include_bytes!(
                "../../../testdata/cops/performance/redundant_block_call/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &RedundantBlockCall,
            include_bytes!(
                "../../../testdata/cops/performance/redundant_block_call/no_offense.rb"
            ),
        );
    }
}
