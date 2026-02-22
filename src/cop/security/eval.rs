use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Eval;

impl Cop for Eval {
    fn name(&self) -> &'static str {
        "Security/Eval"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"eval" {
            return;
        }

        // Allow: no receiver (bare eval) or receiver is Kernel
        let allowed = match call.receiver() {
            None => true,
            Some(recv) => {
                recv.as_constant_read_node()
                    .map(|c| c.name().as_slice() == b"Kernel")
                    .unwrap_or(false)
                    || recv
                        .as_constant_path_node()
                        .map(|cp| {
                            let loc = cp.location();
                            &source.as_bytes()[loc.start_offset()..loc.end_offset()] == b"Kernel"
                                || source.as_bytes()[loc.start_offset()..loc.end_offset()]
                                    .ends_with(b"::Kernel")
                        })
                        .unwrap_or(false)
            }
        };

        if !allowed {
            return;
        }

        let msg_loc = call.message_loc().unwrap();
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "The use of `eval` is a serious security risk.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Eval, "cops/security/eval");
}
