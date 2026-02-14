use ruby_prism::Visit;

use crate::cop::util::{is_rspec_example, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MultipleExpectations;

impl Cop for MultipleExpectations {
    fn name(&self) -> &'static str {
        "RSpec/MultipleExpectations"
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
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        if !is_rspec_example(method_name) {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        let max = config.get_usize("Max", 1);

        // Count expect/is_expected calls in the block body
        let mut counter = ExpectCounter { count: 0 };
        if let Some(body) = block.body() {
            counter.visit(&body);
        }

        if counter.count > max {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            vec![self.diagnostic(
                source,
                line,
                column,
                format!(
                    "Example has too many expectations [{}/{}].",
                    counter.count, max
                ),
            )]
        } else {
            Vec::new()
        }
    }
}

struct ExpectCounter {
    count: usize,
}

impl<'pr> Visit<'pr> for ExpectCounter {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();
        if node.receiver().is_none()
            && (name == b"expect"
                || name == b"expect_any_instance_of"
                || name == b"is_expected")
        {
            self.count += 1;
        }
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultipleExpectations, "cops/rspec/multiple_expectations");
}
