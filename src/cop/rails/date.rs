use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Date;

impl Cop for Date {
    fn name(&self) -> &'static str {
        "Rails/Date"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "flexible");
        let allow_to_time = config.get_bool("AllowToTime", true);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method = call.name().as_slice();

        // In strict mode, also flag `to_time`
        if method == b"to_time" && !allow_to_time && style == "strict" {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Do not use `to_time` in strict mode.".to_string(),
            )];
        }

        if method != b"today" {
            return Vec::new();
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        // Handle both ConstantReadNode (Date) and ConstantPathNode (::Date)
        if util::constant_name(&recv) != Some(b"Date") {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `Date.current` instead of `Date.today`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Date, "cops/rails/date");
}
