use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Open;

impl Cop for Open {
    fn name(&self) -> &'static str {
        "Security/Open"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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

        if call.name().as_slice() != b"open" {
            return Vec::new();
        }

        // Allow: no receiver (bare open) or receiver is Kernel
        let allowed = match call.receiver() {
            None => true,
            Some(recv) => {
                if let Some(cr) = recv.as_constant_read_node() {
                    cr.name().as_slice() == b"Kernel"
                } else if let Some(cp) = recv.as_constant_path_node() {
                    cp.name().map(|n| n.as_slice() == b"Kernel").unwrap_or(false)
                        && cp.parent().is_none()
                } else {
                    false
                }
            }
        };

        if !allowed {
            return Vec::new();
        }

        let msg_loc = call.message_loc().unwrap();
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "The use of `Kernel#open` is a serious security risk.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Open, "cops/security/open");
}
