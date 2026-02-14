use ruby_prism::Visit;

use crate::cop::util::{is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct NestedGroups;

impl Cop for NestedGroups {
    fn name(&self) -> &'static str {
        "RSpec/NestedGroups"
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
        parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Only trigger on top-level RSpec.describe or top-level describe
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();

        // Check for RSpec.describe or bare describe at top level
        let is_top_level = if let Some(recv) = call.receiver() {
            if let Some(rc) = recv.as_constant_read_node() {
                rc.name().as_slice() == b"RSpec" && method_name == b"describe"
            } else {
                false
            }
        } else {
            is_rspec_example_group(method_name)
        };

        if !is_top_level {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        let max = config.get_usize("Max", 3);
        let mut diagnostics = Vec::new();

        // Walk the block body looking for nested groups
        if let Some(body) = block.body() {
            let mut visitor = NestingVisitor {
                source,
                max,
                depth: 1, // The top-level describe is depth 1
                diagnostics: &mut diagnostics,
                cop: self,
                parse_result,
            };
            visitor.visit(&body);
        }

        diagnostics
    }
}

struct NestingVisitor<'a, 'pr> {
    source: &'a SourceFile,
    max: usize,
    depth: usize,
    diagnostics: &'a mut Vec<Diagnostic>,
    cop: &'a NestedGroups,
    #[allow(dead_code)]
    parse_result: &'a ruby_prism::ParseResult<'pr>,
}

impl<'pr> Visit<'pr> for NestingVisitor<'_, 'pr> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        // Only count receiverless example group calls as nesting
        if node.receiver().is_none() && is_rspec_example_group(method_name) {
            let new_depth = self.depth + 1;

            if new_depth > self.max {
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!(
                        "Maximum example group nesting exceeded [{new_depth}/{}].",
                        self.max
                    ),
                ));
            }

            // Recurse into the block body with incremented depth
            if let Some(block) = node.block() {
                if let Some(bn) = block.as_block_node() {
                    if let Some(body) = bn.body() {
                        let old_depth = self.depth;
                        self.depth = new_depth;
                        self.visit(&body);
                        self.depth = old_depth;
                    }
                }
            }
            // Don't call default visit since we handled recursion
            return;
        }

        // For non-example-group calls, recurse into their blocks
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NestedGroups, "cops/rspec/nested_groups");
}
