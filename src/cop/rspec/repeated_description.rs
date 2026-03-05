use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use std::collections::HashMap;

/// RSpec/RepeatedDescription: Don't repeat descriptions within an example group.
///
/// ## Corpus investigation findings
///
/// **FP root cause (408 FPs):** `its` calls were grouped together with `it`/`specify` using
/// only their arguments as signature. RuboCop treats `its` separately (`repeated_its` vs
/// `repeated_descriptions`) and includes the full block body in the `its` signature. So
/// `its(:x) { be_present }` and `its(:x) { be_blank }` are distinct in RuboCop but were
/// colliding in nitrocop.
///
/// **Fix:** For `its` calls, include the block body source text in the signature, and group
/// `its` calls separately from `it`/`specify`/etc. calls.
///
/// **FN root cause (914 FNs):** RuboCop's `ExampleGroup#examples` uses `find_all_in_scope`,
/// which recursively descends into child nodes, stopping only at scope boundaries (nested
/// `describe`/`context`/`shared_examples` blocks). Nitrocop only iterated direct children,
/// missing examples inside iterator blocks like `%i[foo bar].each do |type| ... end`.
///
/// **Fix:** Added recursive `collect_examples` helper that walks the AST collecting example
/// calls, stopping at scope-changing boundaries (example group / shared group blocks).
pub struct RepeatedDescription;

impl Cop for RepeatedDescription {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedDescription"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, STATEMENTS_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();
        if !is_example_group(name) {
            return;
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        // Collect examples recursively, stopping at scope boundaries.
        // RuboCop separates `its` from `it`/`specify` etc., so we use two maps.
        #[allow(clippy::type_complexity)]
        let mut desc_map: HashMap<Vec<u8>, Vec<(usize, usize)>> = HashMap::new();
        #[allow(clippy::type_complexity)]
        let mut its_map: HashMap<Vec<u8>, Vec<(usize, usize)>> = HashMap::new();

        collect_examples(source, &body, &mut desc_map, &mut its_map);

        for locs in desc_map.values().chain(its_map.values()) {
            if locs.len() > 1 {
                for &(line, col) in locs {
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        "Don't repeat descriptions within an example group.".to_string(),
                    ));
                }
            }
        }
    }
}

/// Recursively collect example calls within a node, stopping at scope boundaries
/// (nested describe/context/shared_examples blocks). This matches RuboCop's
/// `find_all_in_scope` behavior.
fn collect_examples(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    desc_map: &mut HashMap<Vec<u8>, Vec<(usize, usize)>>,
    its_map: &mut HashMap<Vec<u8>, Vec<(usize, usize)>>,
) {
    // Handle statements node: iterate each statement
    if let Some(stmts) = node.as_statements_node() {
        for stmt in stmts.body().iter() {
            collect_examples(source, &stmt, desc_map, its_map);
        }
        return;
    }

    // Handle call nodes
    if let Some(c) = node.as_call_node() {
        let m = c.name().as_slice();

        // If this is a scope-changing call (example group) with a block, do NOT recurse
        if is_scope_change(m) && c.block().is_some() {
            return;
        }

        if m == b"its" {
            // For `its`, signature includes block body (RuboCop's its_signature)
            if let Some(s) = its_signature(source, &c) {
                let loc = c.location();
                let (line, col) = source.offset_to_line_col(loc.start_offset());
                its_map.entry(s).or_default().push((line, col));
            }
            return;
        }

        if is_rspec_example(m) {
            // For it/specify/etc, signature is just the arguments
            if let Some(s) = example_signature(source, &c) {
                let loc = c.location();
                let (line, col) = source.offset_to_line_col(loc.start_offset());
                desc_map.entry(s).or_default().push((line, col));
            }
            return;
        }

        // Not an example and not a scope change — recurse into block body if present
        if let Some(block) = c.block() {
            if let Some(block_node) = block.as_block_node() {
                if let Some(body) = block_node.body() {
                    collect_examples(source, &body, desc_map, its_map);
                }
            }
        }
    }
}

/// Build a signature for an example call based on the source text of its arguments
/// (description + metadata), excluding the block body.
fn example_signature(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.is_empty() {
        return None; // No description = one-liner, skip
    }

    // Build signature from the source text of all arguments
    let args_loc = args.location();
    let sig = source.as_bytes()[args_loc.start_offset()..args_loc.end_offset()].to_vec();
    Some(sig)
}

/// Build a signature for an `its` call that includes the block body.
/// This matches RuboCop's `its_signature` which is `[doc_string, example_node]`.
/// Including the block body ensures `its(:x) { be_present }` and `its(:x) { be_blank }`
/// get different signatures.
fn its_signature(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.is_empty() {
        return None;
    }

    // Start with arguments text
    let args_loc = args.location();
    let mut sig = source.as_bytes()[args_loc.start_offset()..args_loc.end_offset()].to_vec();

    // Append block body source text if present
    if let Some(block) = call.block() {
        if let Some(block_node) = block.as_block_node() {
            if let Some(body) = block_node.body() {
                let body_loc = body.location();
                sig.push(b'\0'); // separator
                sig.extend_from_slice(
                    &source.as_bytes()[body_loc.start_offset()..body_loc.end_offset()],
                );
            }
        }
    }

    Some(sig)
}

/// Check if a method name is a scope-changing boundary for RSpec example collection.
/// This includes example groups and shared groups — recursion should stop at these.
fn is_scope_change(name: &[u8]) -> bool {
    is_rspec_example_group(name)
}

fn is_example_group(name: &[u8]) -> bool {
    // NOTE: shared_examples, shared_examples_for, and shared_context are
    // intentionally excluded — RuboCop's RepeatedDescription only fires on
    // ExampleGroups (describe/context/feature), not SharedGroups.
    matches!(
        name,
        b"describe"
            | b"context"
            | b"feature"
            | b"example_group"
            | b"xdescribe"
            | b"xcontext"
            | b"xfeature"
            | b"fdescribe"
            | b"fcontext"
            | b"ffeature"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(RepeatedDescription, "cops/rspec/repeated_description");
}
