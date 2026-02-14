use ruby_prism::Visit;

use crate::cop::util::{is_rspec_example, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct NoExpectationExample;

impl Cop for NoExpectationExample {
    fn name(&self) -> &'static str {
        "RSpec/NoExpectationExample"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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

        let method_name = call.name().as_slice();
        if !is_rspec_example(method_name) {
            return Vec::new();
        }

        // Skip `pending` and `skip` examples -- they intentionally have no expectations
        if method_name == b"pending" || method_name == b"skip" {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        // Check if the block body contains any expectation
        let mut finder = ExpectationFinder { found: false };
        if let Some(body) = block.body() {
            finder.visit(&body);
        }

        if !finder.found {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            vec![self.diagnostic(
                source,
                line,
                column,
                "No expectation found in this example.".to_string(),
            )]
        } else {
            Vec::new()
        }
    }
}

struct ExpectationFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for ExpectationFinder {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.found {
            return;
        }
        let name = node.name().as_slice();
        // Check for: expect, is_expected, assert*, should, should_not, pending, skip
        if node.receiver().is_none()
            && (name == b"expect"
                || name == b"expect_any_instance_of"
                || name == b"is_expected"
                || name.starts_with(b"assert")
                || name == b"pending"
                || name == b"skip")
        {
            self.found = true;
            return;
        }
        // Check for `should` and `should_not` (with any receiver)
        if name == b"should" || name == b"should_not" {
            self.found = true;
            return;
        }
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NoExpectationExample, "cops/rspec/no_expectation_example");
}
