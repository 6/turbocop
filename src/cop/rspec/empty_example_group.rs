use ruby_prism::Visit;

use crate::cop::util::{is_rspec_example, is_rspec_example_group, is_rspec_hook, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyExampleGroup;

impl Cop for EmptyExampleGroup {
    fn name(&self) -> &'static str {
        "RSpec/EmptyExampleGroup"
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

        // Check for example group calls (including RSpec.describe)
        let is_example_group = if let Some(recv) = call.receiver() {
            if let Some(rc) = recv.as_constant_read_node() {
                rc.name().as_slice() == b"RSpec" && method_name == b"describe"
            } else {
                false
            }
        } else {
            is_rspec_example_group(method_name)
        };

        if !is_example_group {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        // Check if the block body contains any examples
        let has_examples = if let Some(body) = block.body() {
            let mut finder = ExampleFinder { found: false };
            finder.visit(&body);
            finder.found
        } else {
            false
        };

        if !has_examples {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            vec![self.diagnostic(
                source,
                line,
                column,
                "Empty example group detected.".to_string(),
            )]
        } else {
            Vec::new()
        }
    }
}

struct ExampleFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for ExampleFinder {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.found {
            return;
        }
        let name = node.name().as_slice();

        // Check for example methods (it, specify, etc.)
        if node.receiver().is_none() && is_rspec_example(name) {
            self.found = true;
            return;
        }

        // Check for include_examples, it_behaves_like, etc.
        if node.receiver().is_none()
            && (name == b"include_examples"
                || name == b"it_behaves_like"
                || name == b"it_should_behave_like"
                || name == b"include_context")
        {
            self.found = true;
            return;
        }

        // Nested example groups count as "content" (they'll be checked individually)
        if node.receiver().is_none() && is_rspec_example_group(name) {
            if node.block().is_some() {
                self.found = true;
            }
            return;
        }

        // Don't descend into hooks (before/after/around) - examples inside hooks don't count
        if node.receiver().is_none() && is_rspec_hook(name) {
            return;
        }

        ruby_prism::visit_call_node(self, node);
    }

    // Also check inside if/else and case/when branches
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        if self.found {
            return;
        }
        ruby_prism::visit_if_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        EmptyExampleGroup, "cops/rspec/empty_example_group",
        scenario_empty_context = "empty_context.rb",
        scenario_empty_describe = "empty_describe.rb",
        scenario_hooks_only = "hooks_only.rb",
    );
}
