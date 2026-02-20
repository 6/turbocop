use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, FLOAT_NODE, INTEGER_NODE};

pub struct RandOne;

impl Cop for RandOne {
    fn name(&self) -> &'static str {
        "Lint/RandOne"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, FLOAT_NODE, INTEGER_NODE]
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

        if call.name().as_slice() != b"rand" {
            return;
        }

        // Must be receiverless or Kernel.rand
        if let Some(recv) = call.receiver() {
            match constant_name(&recv) {
                Some(name) if name == b"Kernel" => {}
                _ => return,
            }
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        if args.len() != 1 {
            return;
        }

        let first_arg = args.iter().next().unwrap();
        let is_one = is_one_value(&first_arg, source);
        if !is_one {
            return;
        }

        let loc = call.location();
        let call_src = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
        let call_str = std::str::from_utf8(call_src).unwrap_or("rand(1)");
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("`{call_str}` always returns `0`. Perhaps you meant `rand(2)` or `rand`?"),
        ));
    }
}

fn is_one_value(node: &ruby_prism::Node<'_>, source: &SourceFile) -> bool {
    // Check for integer 1 or -1
    if let Some(int_node) = node.as_integer_node() {
        let src = &source.as_bytes()[int_node.location().start_offset()..int_node.location().end_offset()];
        return src == b"1" || src == b"-1";
    }
    // Check for float 1.0 or -1.0
    if let Some(float_node) = node.as_float_node() {
        let src = &source.as_bytes()[float_node.location().start_offset()..float_node.location().end_offset()];
        return src == b"1.0" || src == b"-1.0";
    }
    // Check for unary minus: -1 as a CallNode wrapping IntegerNode
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"-@" {
            if let Some(recv) = call.receiver() {
                if let Some(int_node) = recv.as_integer_node() {
                    let src = &source.as_bytes()[int_node.location().start_offset()..int_node.location().end_offset()];
                    return src == b"1";
                }
                if let Some(float_node) = recv.as_float_node() {
                    let src = &source.as_bytes()[float_node.location().start_offset()..float_node.location().end_offset()];
                    return src == b"1.0";
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RandOne, "cops/lint/rand_one");
}
