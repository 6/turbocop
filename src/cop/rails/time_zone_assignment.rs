use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct TimeZoneAssignment;

impl Cop for TimeZoneAssignment {
    fn name(&self) -> &'static str {
        "Rails/TimeZoneAssignment"
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

        if call.name().as_slice() != b"zone=" {
            return Vec::new();
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        // Handle both ConstantReadNode (Time) and ConstantPathNode (::Time)
        if util::constant_name(&recv) != Some(b"Time") {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Do not set `Time.zone` directly. Use `Time.use_zone` instead.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TimeZoneAssignment, "cops/rails/time_zone_assignment");
}
