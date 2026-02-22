use crate::cop::node_type::CALL_NODE;
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "flexible");
        let allow_to_time = config.get_bool("AllowToTime", true);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = call.name().as_slice();

        // In strict mode, also flag `to_time`
        if method == b"to_time" && !allow_to_time && style == "strict" {
            let msg_loc = match call.message_loc() {
                Some(loc) => loc,
                None => return,
            };
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Do not use `to_time` in strict mode.".to_string(),
            ));
        }

        if method != b"today" {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        // Handle both ConstantReadNode (Date) and ConstantPathNode (::Date)
        if util::constant_name(&recv) != Some(b"Date") {
            return;
        }

        let msg_loc = match call.message_loc() {
            Some(loc) => loc,
            None => return,
        };
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `Date.current` instead of `Date.today`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Date, "cops/rails/date");
}
