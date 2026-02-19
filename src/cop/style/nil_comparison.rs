use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, NIL_NODE};

pub struct NilComparison;

impl Cop for NilComparison {
    fn name(&self) -> &'static str {
        "Style/NilComparison"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, NIL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "predicate");

        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call_node.name();
        let method_bytes = method_name.as_slice();

        if call_node.receiver().is_none() {
            return Vec::new();
        }

        if enforced_style == "predicate" {
            // Flag `x == nil` and `x === nil`
            if method_bytes != b"==" && method_bytes != b"===" {
                return Vec::new();
            }

            // Check if the argument is nil
            let args = match call_node.arguments() {
                Some(a) => a,
                None => return Vec::new(),
            };
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 1 {
                return Vec::new();
            }
            if arg_list[0].as_nil_node().is_none() {
                return Vec::new();
            }

            let msg_loc = call_node.message_loc().unwrap_or_else(|| call_node.location());
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            vec![self.diagnostic(
                source,
                line,
                column,
                "Prefer the use of the `nil?` predicate.".to_string(),
            )]
        } else {
            // comparison style: flag `x.nil?`
            if method_bytes != b"nil?" {
                return Vec::new();
            }

            let msg_loc = call_node.message_loc().unwrap_or_else(|| call_node.location());
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            vec![self.diagnostic(
                source,
                line,
                column,
                "Prefer the use of the `==` comparison.".to_string(),
            )]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NilComparison, "cops/style/nil_comparison");
}
