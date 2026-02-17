use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct LambdaCall;

impl Cop for LambdaCall {
    fn name(&self) -> &'static str {
        "Style/LambdaCall"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must have a receiver
        if call.receiver().is_none() {
            return Vec::new();
        }

        let enforced_style = config.get_str("EnforcedStyle", "call");

        match enforced_style {
            "call" => {
                // Detect lambda.() (implicit call — method name is "call" but no message_loc or
                // message_loc source is empty). In Prism, lambda.() is represented as CallNode
                // with name "call" but the method_name position is at the dot.
                let name = call.name();
                if name.as_slice() != b"call" {
                    return Vec::new();
                }

                // Check if this is an implicit call (lambda.() syntax)
                // In implicit call, there's no explicit "call" selector
                let msg_loc = match call.message_loc() {
                    Some(loc) => loc,
                    None => {
                        // No message_loc means implicit call like foo.()
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            "Prefer the use of `lambda.call(...)` over `lambda.(...)`.".to_string(),
                        )];
                    }
                };

                // If the message_loc source IS "call", this is already explicit style
                if msg_loc.as_slice() == b"call" {
                    return Vec::new();
                }

                // Otherwise it's implicit
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer the use of `lambda.call(...)` over `lambda.(...)`.".to_string(),
                )]
            }
            "braces" => {
                // Detect lambda.call() (explicit call)
                let name = call.name();
                if name.as_slice() != b"call" {
                    return Vec::new();
                }

                // Check if this is an explicit call
                let msg_loc = match call.message_loc() {
                    Some(loc) => loc,
                    None => return Vec::new(), // Already implicit
                };

                if msg_loc.as_slice() != b"call" {
                    return Vec::new();
                }

                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer the use of `lambda.(...)` over `lambda.call(...)`.".to_string(),
                )]
            }
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LambdaCall, "cops/style/lambda_call");
}
