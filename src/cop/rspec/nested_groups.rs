use ruby_prism::Visit;

use crate::cop::node_type::{BLOCK_NODE, CALL_NODE};
use crate::cop::util::{
    self, RSPEC_DEFAULT_INCLUDE, is_rspec_example_group, is_rspec_shared_group,
};
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Only trigger on top-level RSpec.describe or top-level describe
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Check for RSpec.describe / ::RSpec.describe or bare describe at top level
        let is_top_level = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec") && method_name == b"describe"
        } else {
            is_rspec_example_group(method_name)
        };

        if !is_top_level {
            return;
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        let max = config.get_usize("Max", 3);
        // Config: AllowedGroups — group method names exempt from nesting count
        let allowed_groups = config.get_string_array("AllowedGroups").unwrap_or_default();

        // Walk the block body looking for nested groups
        if let Some(body) = block.body() {
            let mut visitor = NestingVisitor {
                source,
                max,
                depth: 1, // The top-level describe is depth 1
                diagnostics,
                cop: self,
                parse_result,
                allowed_groups: &allowed_groups,
            };
            visitor.visit(&body);
        }
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
    allowed_groups: &'a [String],
}

impl<'pr> Visit<'pr> for NestingVisitor<'_, 'pr> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        // Shared group definitions (shared_examples, shared_context) do NOT
        // increment nesting depth — they define reusable groups with their own
        // independent scope.  Recurse into their block body at the SAME depth.
        if node.receiver().is_none() && is_rspec_shared_group(method_name) {
            if let Some(block) = node.block() {
                if let Some(bn) = block.as_block_node() {
                    if let Some(body) = bn.body() {
                        self.visit(&body);
                    }
                }
            }
            return;
        }

        // Only count receiverless example group calls as nesting
        let is_allowed = self
            .allowed_groups
            .iter()
            .any(|g| g.as_bytes() == method_name);
        if node.receiver().is_none() && is_rspec_example_group(method_name) && !is_allowed {
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

    #[test]
    fn allowed_groups_skips_matching() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                (
                    "Max".into(),
                    serde_yml::Value::Number(serde_yml::Number::from(1)),
                ),
                (
                    "AllowedGroups".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("context".into())]),
                ),
            ]),
            ..CopConfig::default()
        };
        // describe > context (allowed, not counted) — depth stays 1
        let source =
            b"describe Foo do\n  context 'bar' do\n    it 'works' do\n    end\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&NestedGroups, source, config);
        assert!(
            diags.is_empty(),
            "AllowedGroups should not count matching groups"
        );
    }
}
