use crate::cop::rspec_rails::RSPEC_RAILS_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct HaveHttpStatus;

/// Default response method names.
const DEFAULT_RESPONSE_METHODS: &[&str] = &["response", "last_response"];

/// Equality matchers that can be replaced by have_http_status.
const EQUALITY_MATCHERS: &[&[u8]] = &[b"be", b"eq", b"eql", b"equal"];

/// Runner methods: to, not_to, to_not.
const RUNNERS: &[&[u8]] = &[b"to", b"not_to", b"to_not"];

impl Cop for HaveHttpStatus {
    fn name(&self) -> &'static str {
        "RSpecRails/HaveHttpStatus"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_RAILS_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Pattern: expect(response.status).to be(200)
        // AST: CallNode(receiver=CallNode(expect(CallNode(response.status))), name=to,
        //   args=[CallNode(be, args=[IntegerNode(200)])])
        //
        // We look for the runner call (to/not_to/to_not).
        let runner_call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let runner_name = runner_call.name().as_slice();
        if !RUNNERS.iter().any(|r| *r == runner_name) {
            return Vec::new();
        }

        // The receiver must be an expect(...) call
        let expect_call = match runner_call.receiver() {
            Some(r) => match r.as_call_node() {
                Some(c) => c,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        if expect_call.name().as_slice() != b"expect" || expect_call.receiver().is_some() {
            return Vec::new();
        }

        // The argument to expect must be response.status or response.code
        let expect_args = match expect_call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let expect_arg_list: Vec<_> = expect_args.arguments().iter().collect();
        if expect_arg_list.len() != 1 {
            return Vec::new();
        }

        let response_status_call = match expect_arg_list[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let accessor = response_status_call.name().as_slice();
        if accessor != b"status" && accessor != b"code" {
            return Vec::new();
        }

        // The receiver of .status/.code must be a response method
        let response_recv = match response_status_call.receiver() {
            Some(r) => match r.as_call_node() {
                Some(c) => c,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        if response_recv.receiver().is_some() {
            return Vec::new();
        }

        let response_method = std::str::from_utf8(response_recv.name().as_slice()).unwrap_or("");
        let response_methods = config
            .get_string_array("ResponseMethods")
            .unwrap_or_else(|| DEFAULT_RESPONSE_METHODS.iter().map(|s| s.to_string()).collect());
        if !response_methods.iter().any(|m| m == response_method) {
            return Vec::new();
        }

        // The runner argument must be a matcher call: be/eq/eql/equal with a numeric argument
        let runner_args = match runner_call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let runner_arg_list: Vec<_> = runner_args.arguments().iter().collect();
        if runner_arg_list.len() != 1 {
            return Vec::new();
        }

        let matcher_call = match runner_arg_list[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let matcher_name = matcher_call.name().as_slice();
        if !EQUALITY_MATCHERS.iter().any(|m| *m == matcher_name) {
            return Vec::new();
        }

        if matcher_call.receiver().is_some() {
            return Vec::new();
        }

        // Get the status argument
        let matcher_args = match matcher_call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let matcher_arg_list: Vec<_> = matcher_args.arguments().iter().collect();
        if matcher_arg_list.len() != 1 {
            return Vec::new();
        }

        let status_arg = &matcher_arg_list[0];

        // Extract numeric status value
        let status_str = if status_arg.as_integer_node().is_some() {
            let int_loc = status_arg.location();
            let text = std::str::from_utf8(int_loc.as_slice()).unwrap_or("");
            text.to_string()
        } else if let Some(str_node) = status_arg.as_string_node() {
            let content = str_node.unescaped();
            let s = std::str::from_utf8(content.as_ref()).unwrap_or("");
            // Must be purely numeric
            if !s.bytes().all(|b| b.is_ascii_digit()) || s.is_empty() {
                return Vec::new();
            }
            s.to_string()
        } else {
            return Vec::new();
        };

        let runner_str = std::str::from_utf8(runner_name).unwrap_or("to");
        let loc = expect_call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        let bad_code_start = loc.start_offset();
        let bad_code_end = runner_call.location().start_offset() + runner_call.location().as_slice().len();
        let bad_code = std::str::from_utf8(
            &source.as_bytes()[bad_code_start..bad_code_end],
        )
        .unwrap_or("...");

        vec![self.diagnostic(
            source,
            line,
            column,
            format!(
                "Prefer `expect({response_method}).{runner_str} have_http_status({status_str})` over `{bad_code}`."
            ),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HaveHttpStatus, "cops/rspecrails/have_http_status");
}
