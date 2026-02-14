use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ResponseParsedBody;

impl Cop for ResponseParsedBody {
    fn name(&self) -> &'static str {
        "Rails/ResponseParsedBody"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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

        if call.name().as_slice() != b"parse" {
            return Vec::new();
        }

        // Receiver must be constant `JSON`
        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let const_read = match recv.as_constant_read_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if const_read.name().as_slice() != b"JSON" {
            return Vec::new();
        }

        // First argument should be `response.body`
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let arg_call = match arg_list[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if arg_call.name().as_slice() != b"body" {
            return Vec::new();
        }

        // The receiver of .body should be `response`
        let body_recv = match arg_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let body_recv_call = match body_recv.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if body_recv_call.name().as_slice() != b"response" {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `response.parsed_body` instead of `JSON.parse(response.body)`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ResponseParsedBody, "cops/rails/response_parsed_body");
}
