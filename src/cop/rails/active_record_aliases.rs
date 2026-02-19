use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct ActiveRecordAliases;

impl Cop for ActiveRecordAliases {
    fn name(&self) -> &'static str {
        "Rails/ActiveRecordAliases"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();

        // Must have arguments (update_attributes with no args is a different method)
        if call.arguments().is_none() {
            return;
        }

        let (current, prefer) = if name == b"update_attributes" {
            ("update_attributes", "update")
        } else if name == b"update_attributes!" {
            ("update_attributes!", "update!")
        } else {
            return;
        };

        // Must have a receiver (obj.update_attributes)
        if call.receiver().is_none() {
            return;
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{prefer}` instead of `{current}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ActiveRecordAliases, "cops/rails/active_record_aliases");
}
