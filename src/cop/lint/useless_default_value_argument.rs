use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for `fetch` or `Array.new` with both a default value argument and a block.
/// The block always supersedes the default value argument.
pub struct UselessDefaultValueArgument;

impl Cop for UselessDefaultValueArgument {
    fn name(&self) -> &'static str {
        "Lint/UselessDefaultValueArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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

        // Must have a block
        if call.block().is_none() {
            return Vec::new();
        }

        let method_name = call.name().as_slice();

        if method_name == b"fetch" {
            // Must have a receiver (not a bare fetch call)
            let receiver = match call.receiver() {
                Some(r) => r,
                None => return Vec::new(),
            };

            // Skip if receiver is in AllowedReceivers
            let allowed_receivers = config.get_string_array("AllowedReceivers").unwrap_or_default();
            if !allowed_receivers.is_empty() {
                let recv_bytes = receiver.location().as_slice();
                let recv_str = std::str::from_utf8(recv_bytes).unwrap_or("");
                if allowed_receivers.iter().any(|r| r == recv_str) {
                    return Vec::new();
                }
            }

            // Must have 2 arguments (key and default_value)
            let args = match call.arguments() {
                Some(a) => a,
                None => return Vec::new(),
            };
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 2 {
                return Vec::new();
            }

            // Skip if second argument is a keyword hash (keyword arguments)
            if arg_list[1].as_keyword_hash_node().is_some() {
                return Vec::new();
            }

            let default_loc = arg_list[1].location();
            let (line, column) = source.offset_to_line_col(default_loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Block supersedes default value argument.".to_string(),
            )];
        } else if method_name == b"new" {
            // Check for Array.new(size, default) { block }
            let receiver = match call.receiver() {
                Some(r) => r,
                None => return Vec::new(),
            };

            let recv_name = match constant_name(&receiver) {
                Some(n) => n,
                None => return Vec::new(),
            };

            if recv_name != b"Array" {
                return Vec::new();
            }

            let args = match call.arguments() {
                Some(a) => a,
                None => return Vec::new(),
            };
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 2 {
                return Vec::new();
            }

            let default_loc = arg_list[1].location();
            let (line, column) = source.offset_to_line_col(default_loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Block supersedes default value argument.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        UselessDefaultValueArgument,
        "cops/lint/useless_default_value_argument"
    );
}
