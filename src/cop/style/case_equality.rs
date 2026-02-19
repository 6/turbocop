use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_REGULAR_EXPRESSION_NODE, REGULAR_EXPRESSION_NODE, SELF_NODE};

pub struct CaseEquality;

impl Cop for CaseEquality {
    fn name(&self) -> &'static str {
        "Style/CaseEquality"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_REGULAR_EXPRESSION_NODE, REGULAR_EXPRESSION_NODE, SELF_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allow_on_constant = config.get_bool("AllowOnConstant", false);
        let allow_on_self_class = config.get_bool("AllowOnSelfClass", false);

        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call_node.name().as_slice() != b"===" {
            return Vec::new();
        }

        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Skip regexp receivers (Performance/RegexpMatch handles those)
        if receiver.as_regular_expression_node().is_some()
            || receiver.as_interpolated_regular_expression_node().is_some()
        {
            return Vec::new();
        }

        // AllowOnConstant
        if allow_on_constant
            && (receiver.as_constant_read_node().is_some()
                || receiver.as_constant_path_node().is_some())
        {
            return Vec::new();
        }

        // AllowOnSelfClass: self.class === something
        if allow_on_self_class {
            if let Some(recv_call) = receiver.as_call_node() {
                if recv_call.name().as_slice() == b"class" {
                    if let Some(inner_recv) = recv_call.receiver() {
                        if inner_recv.as_self_node().is_some() {
                            return Vec::new();
                        }
                    }
                }
            }
        }

        let msg_loc = call_node.message_loc().unwrap_or_else(|| call_node.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Avoid the use of the case equality operator `===`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CaseEquality, "cops/style/case_equality");
}
