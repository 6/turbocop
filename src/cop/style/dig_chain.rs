use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DigChain;

impl Cop for DigChain {
    fn name(&self) -> &'static str {
        "Style/DigChain"
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

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if method_name != "dig" {
            return Vec::new();
        }

        // Must have arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // Check for hash/keyword hash args (not supported)
        for arg in &arg_list {
            if arg.as_hash_node().is_some() || arg.as_keyword_hash_node().is_some() {
                return Vec::new();
            }
        }

        // Check if the receiver is also a dig call
        let receiver = match call.receiver() {
            Some(r) => r,
            None => {
                // No receiver - check if receiver-less dig is chained
                return Vec::new();
            }
        };

        if let Some(recv_call) = receiver.as_call_node() {
            let recv_method = std::str::from_utf8(recv_call.name().as_slice()).unwrap_or("");
            if recv_method == "dig" {
                // Check that inner dig also has arguments
                if let Some(inner_args) = recv_call.arguments() {
                    let inner_list: Vec<_> = inner_args.arguments().iter().collect();
                    if inner_list.is_empty() {
                        return Vec::new();
                    }
                    // Check for hash/keyword hash args in inner call
                    for arg in &inner_list {
                        if arg.as_hash_node().is_some() || arg.as_keyword_hash_node().is_some() {
                            return Vec::new();
                        }
                    }
                } else {
                    return Vec::new();
                }

                // This is a chained dig - only report on the outermost call
                // First check if *this* call is also a receiver of another dig
                // (we only want to report the topmost in a chain)
                // We can't check parent, so we just report and the dedup will handle it.

                let loc = recv_call.message_loc().unwrap_or(recv_call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `dig` with multiple parameters instead of chaining.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DigChain, "cops/style/dig_chain");
}
