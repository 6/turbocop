use ruby_prism::Visit;

use crate::cop::util::{
    self, RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group, is_rspec_hook,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/EmptyExampleGroup: Checks if an example group does not include any tests.
///
/// FP investigation (2026-03):
/// - Example groups defined inside method definitions (`def`/`def self.`) are skipped,
///   matching RuboCop's `return if node.each_ancestor(:any_def).any?`. These commonly
///   use `instance_eval(&block)`, `class_eval(&block)`, `module_exec(&block)`, or
///   `yield` to inject content dynamically from the caller.
/// - Example groups nested inside example blocks (`it`, `specify`, etc.) are skipped,
///   matching RuboCop's `return if node.each_ancestor(:block).any? { example?(block) }`.
///   These are meta-spec patterns like `RSpec.describe { } .run` inside an `it` block.
///
/// ## Corpus investigation (2026-03-19)
///
/// FP=0, FN=11.
///
/// FN=11: All FNs were example groups with `def self.method_name` that internally
/// calls `it`/`example`/etc. The `ExampleFinder` was descending into def bodies
/// and finding these dynamically-created examples, treating the group as non-empty.
/// RuboCop's `examples?` matcher only checks immediate children and blocks, not
/// def bodies. Fix: override `visit_def_node` in ExampleFinder to skip def bodies.
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

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = EmptyGroupVisitor {
            source,
            cop: self,
            diagnostics,
            def_depth: 0,
            example_depth: 0,
        };
        visitor.visit(&parse_result.node());
    }
}

struct EmptyGroupVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a EmptyExampleGroup,
    diagnostics: &'a mut Vec<Diagnostic>,
    /// Depth inside method definitions (def/defs). When > 0, skip flagging.
    def_depth: u32,
    /// Depth inside example blocks (it/specify/etc). When > 0, skip flagging.
    example_depth: u32,
}

impl<'a> EmptyGroupVisitor<'a> {
    fn is_example_group_call(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        let method_name = call.name().as_slice();
        if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec") && method_name == b"describe"
        } else {
            is_rspec_example_group(method_name)
                && method_name != b"shared_examples"
                && method_name != b"shared_examples_for"
                && method_name != b"shared_context"
        }
    }

    fn is_example_call(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        call.receiver().is_none() && is_rspec_example(call.name().as_slice())
    }
}

impl<'a, 'pr> Visit<'pr> for EmptyGroupVisitor<'a> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.def_depth += 1;
        ruby_prism::visit_def_node(self, node);
        self.def_depth -= 1;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // If inside a def, just continue visiting (don't flag anything)
        if self.def_depth > 0 || self.example_depth > 0 {
            ruby_prism::visit_call_node(self, node);
            return;
        }

        // Check if this is an example group call with a block
        if self.is_example_group_call(node) {
            if let Some(block_arg) = node.block() {
                if let Some(block) = block_arg.as_block_node() {
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
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Empty example group detected.".to_string(),
                        ));
                    }

                    // Visit inside the block to find nested example groups,
                    // but DON'T increment example_depth since this is an example group, not an example
                    if let Some(body) = block.body() {
                        self.visit(&body);
                    }
                    return;
                }
            }
        }

        // If this is an example call (it/specify/etc) with a block, track depth
        if self.is_example_call(node) {
            if let Some(block_arg) = node.block() {
                if let Some(block) = block_arg.as_block_node() {
                    self.example_depth += 1;
                    if let Some(body) = block.body() {
                        self.visit(&body);
                    }
                    self.example_depth -= 1;
                    return;
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
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

    // Don't descend into method definitions — examples inside `def self.method`
    // are dynamically defined and don't count as direct examples for the group.
    // RuboCop's `examples?` uses `def_node_matcher` which only checks immediate
    // children and blocks, not def bodies.
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {
        // Skip — examples inside defs don't make the group non-empty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        EmptyExampleGroup,
        "cops/rspec/empty_example_group",
        scenario_empty_context = "empty_context.rb",
        scenario_empty_describe = "empty_describe.rb",
        scenario_hooks_only = "hooks_only.rb",
        scenario_qualified_rspec = "qualified_rspec.rb",
        scenario_def_self_example_factory = "def_self_example_factory.rb",
    );
}
