use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantException;

impl RedundantException {
    fn is_runtime_error(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(cr) = node.as_constant_read_node() {
            return cr.name().as_slice() == b"RuntimeError";
        }
        if let Some(cp) = node.as_constant_path_node() {
            // ::RuntimeError
            if cp.parent().is_none() {
                if let Some(name) = cp.name() {
                    return name.as_slice() == b"RuntimeError";
                }
            }
        }
        false
    }
}

impl Cop for RedundantException {
    fn name(&self) -> &'static str {
        "Style/RedundantException"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be `raise` or `fail` without a receiver
        let method = call_node.name();
        let method_name = method.as_slice();
        if !matches!(method_name, b"raise" | b"fail") {
            return;
        }
        if call_node.receiver().is_some() {
            return;
        }

        let args = match call_node.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().into_iter().collect();

        // Pattern 1: raise RuntimeError, "message" (exactly 2 args)
        if arg_list.len() == 2 && Self::is_runtime_error(&arg_list[0]) {
            let loc = call_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Redundant `RuntimeError` argument can be removed.".to_string(),
            ));
        }

        // Pattern 2: raise RuntimeError.new("message") (1 arg that's a call to .new on RuntimeError)
        if arg_list.len() == 1 {
            if let Some(new_call) = arg_list[0].as_call_node() {
                if new_call.name().as_slice() == b"new" {
                    if let Some(receiver) = new_call.receiver() {
                        if Self::is_runtime_error(&receiver) {
                            let loc = call_node.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                "Redundant `RuntimeError.new` call can be replaced with just the message.".to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantException, "cops/style/redundant_exception");
}
