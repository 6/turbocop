use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct IdenticalEqualityAssertion;

impl Cop for IdenticalEqualityAssertion {
    fn name(&self) -> &'static str {
        "RSpec/IdenticalEqualityAssertion"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Look for expect(X).to eq(X) / eql(X) / be(X)
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return Vec::new();
        }

        // Only flag `.to` (not `.not_to`)
        if method_name != b"to" {
            return Vec::new();
        }

        // Receiver must be expect(X)
        let expect_call = match call.receiver() {
            Some(recv) => match recv.as_call_node() {
                Some(c) => c,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        if expect_call.name().as_slice() != b"expect" {
            return Vec::new();
        }

        // Get the expect argument
        let expect_args = match expect_call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let expect_arg_list: Vec<_> = expect_args.arguments().iter().collect();
        if expect_arg_list.len() != 1 {
            return Vec::new();
        }

        let expect_arg = &expect_arg_list[0];

        // Get the matcher call (eq/eql/be)
        let matcher_args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let matcher_arg_list: Vec<_> = matcher_args.arguments().iter().collect();
        if matcher_arg_list.is_empty() {
            return Vec::new();
        }

        let matcher_node = &matcher_arg_list[0];
        let matcher_call = match matcher_node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let matcher_name = matcher_call.name().as_slice();
        if matcher_name != b"eq" && matcher_name != b"eql" && matcher_name != b"be" {
            return Vec::new();
        }

        if matcher_call.receiver().is_some() {
            return Vec::new();
        }

        let matcher_inner_args = match matcher_call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let inner_arg_list: Vec<_> = matcher_inner_args.arguments().iter().collect();
        if inner_arg_list.len() != 1 {
            return Vec::new();
        }

        let matcher_arg = &inner_arg_list[0];

        // Compare source text of both expressions
        let expect_loc = expect_arg.location();
        let matcher_loc = matcher_arg.location();

        let expect_text = &source.as_bytes()[expect_loc.start_offset()..expect_loc.end_offset()];
        let matcher_text = &source.as_bytes()[matcher_loc.start_offset()..matcher_loc.end_offset()];

        if expect_text == matcher_text {
            let loc = expect_call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Identical expressions on both sides of the equality may indicate a flawed test.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IdenticalEqualityAssertion, "cops/rspec/identical_equality_assertion");
}
