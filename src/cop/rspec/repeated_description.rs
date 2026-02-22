use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use std::collections::HashMap;

/// RSpec/RepeatedDescription: Don't repeat descriptions within an example group.
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
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        // Collect example descriptions: signature -> list of (line, col)
        let mut desc_map: HashMap<Vec<u8>, Vec<(usize, usize, usize, usize)>> = HashMap::new();

        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                let m = c.name().as_slice();
                if is_rspec_example(m) || m == b"its" {
                    // Build a signature from the full source of the call (without block)
                    let sig = example_signature(source, &c);
                    if let Some(s) = sig {
                        let loc = c.location();
                        let (line, col) = source.offset_to_line_col(loc.start_offset());
                        let end_off = loc.end_offset();
                        desc_map.entry(s).or_default().push((
                            line,
                            col,
                            loc.start_offset(),
                            end_off,
                        ));
                    }
                }
            }
        }

        for (_sig, locs) in &desc_map {
            if locs.len() > 1 {
                for &(line, col, start, end) in locs {
                    let _ = (start, end);
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
