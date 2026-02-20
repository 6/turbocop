use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, STRING_NODE};

pub struct ArelStar;

impl Cop for ArelStar {
    fn name(&self) -> &'static str {
        "Rails/ArelStar"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be `[]` method
        if call.name().as_slice() != b"[]" {
            return;
        }

        // Receiver must exist (arel_table call or constant)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Check if receiver is an arel_table call or a constant
        let is_arel_table = if let Some(recv_call) = receiver.as_call_node() {
            recv_call.name().as_slice() == b"arel_table"
        } else {
            receiver.as_constant_read_node().is_some() || receiver.as_constant_path_node().is_some()
        };

        if !is_arel_table {
            return;
        }

        // Argument must be a string "*"
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }
        let str_node = match arg_list[0].as_string_node() {
            Some(s) => s,
            None => return,
        };
        if str_node.unescaped() != b"*" {
            return;
        }

        let loc = str_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `Arel.star` instead of `\"*\"` for expanded column lists.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ArelStar, "cops/rails/arel_star");
}
