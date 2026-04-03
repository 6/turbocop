use crate::cop::shared::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, PROGRAM_NODE,
};
use crate::cop::shared::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/MultipleDescribes: Flag multiple top-level example groups in a single file.
///
/// ## Corpus investigation (2026-03-14)
///
/// FP=1: asciidoctor-pdf `describe '...', if: cond, &(proc do...end)` style.
/// `&(proc do end)` stores a BlockArgumentNode in call.block(), not a BlockNode.
/// RuboCop's on_block only fires for BlockNode. Fixed by requiring BlockNode.
///
/// ## Corpus investigation (2026-03-18)
///
/// FN=4: Files where `describe` blocks are inside `module`/`class` wrappers
/// (prontolabs/pronto, louismullie/treat, rspec/rspec-rails). The cop only
/// checked direct children of ProgramNode. RuboCop's TopLevelGroup mixin
/// recursively unwraps module/class when there's a single top-level statement.
/// Fixed by implementing the same unwrapping in `collect_example_groups`.
pub struct MultipleDescribes;

impl Cop for MultipleDescribes {
    fn name(&self) -> &'static str {
        "RSpec/MultipleDescribes"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            PROGRAM_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Only check ProgramNode (root)
        let program = match node.as_program_node() {
            Some(p) => p,
            None => return,
        };

        let stmts = program.statements();

        // Collect top-level example groups, unwrapping single module/class wrappers
        // to match RuboCop's TopLevelGroup mixin.
        let mut example_groups: Vec<(usize, usize)> = Vec::new();
        collect_example_groups(source, &stmts.body(), &mut example_groups);

        if example_groups.len() <= 1 {
            return;
        }

        // RuboCop fires only once per file, on the FIRST top-level example group
        let (line, col) = example_groups[0];
        diagnostics.push(self.diagnostic(
            source,
            line,
            col,
            "Do not use multiple top-level example groups - try to nest them.".to_string(),
        ));
    }
}

/// Mirrors RuboCop's TopLevelGroup#top_level_nodes: if the body has exactly one
/// statement and it's a module or class, recurse into its body. Otherwise scan
/// directly for example groups. This handles files like `module Foo; describe ...; end`.
fn collect_example_groups(
    source: &SourceFile,
    body: &ruby_prism::NodeList<'_>,
    example_groups: &mut Vec<(usize, usize)>,
) {
    let nodes: Vec<_> = body.iter().collect();
    // If there's exactly one node and it's a module/class, unwrap into its body
    if nodes.len() == 1 {
        if let Some(module_node) = nodes[0].as_module_node() {
            if let Some(inner_body) = module_node.body() {
                if let Some(stmts) = inner_body.as_statements_node() {
                    collect_example_groups(source, &stmts.body(), example_groups);
                }
            }
            return;
        }
        if let Some(class_node) = nodes[0].as_class_node() {
            if let Some(inner_body) = class_node.body() {
                if let Some(stmts) = inner_body.as_statements_node() {
                    collect_example_groups(source, &stmts.body(), example_groups);
                }
            }
            return;
        }
    }

    for node in &nodes {
        if let Some(call) = node.as_call_node() {
            let name = call.name().as_slice();
            // Must have a real BlockNode (do...end or { }). BlockArgumentNode (&proc)
            // is not counted — RuboCop's on_block only fires for BlockNode.
            let has_block_node = call.block().is_some_and(|b| b.as_block_node().is_some());
            if has_block_node && is_top_level_example_group(call.receiver().as_ref(), name) {
                let loc = call.location();
                let (line, col) = source.offset_to_line_col(loc.start_offset());
                example_groups.push((line, col));
            }
        }
    }
}

fn is_top_level_example_group(receiver: Option<&ruby_prism::Node<'_>>, name: &[u8]) -> bool {
    // Shared examples/contexts are excluded
    if name == b"shared_examples" || name == b"shared_examples_for" || name == b"shared_context" {
        return false;
    }

    if is_rspec_example_group(name) {
        // Must be receiverless or RSpec.describe / ::RSpec.describe
        match receiver {
            None => return true,
            Some(recv) => {
                if let Some(cr) = recv.as_constant_read_node() {
                    if cr.name().as_slice() == b"RSpec" {
                        return true;
                    }
                }
                if let Some(cp) = recv.as_constant_path_node() {
                    if let Some(n) = cp.name() {
                        if n.as_slice() == b"RSpec" && cp.parent().is_none() {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_scenario_fixture_tests!(
        MultipleDescribes,
        "cops/rspec/multiple_describes",
        scenario_class_and_method = "class_and_method.rb",
        scenario_class_only = "class_only.rb",
        scenario_string_args = "string_args.rb",
        scenario_module_wrapped = "module_wrapped.rb",
        scenario_class_wrapped = "class_wrapped.rb",
    );
}
