use crate::cop::util::{is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashMap;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, PROGRAM_NODE, STATEMENTS_NODE};

/// RSpec/RepeatedExampleGroupBody: Flag example groups with identical bodies.
pub struct RepeatedExampleGroupBody;

impl Cop for RepeatedExampleGroupBody {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedExampleGroupBody"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, PROGRAM_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // We need to look at sibling example groups within a common parent.
        // The parent can be a ProgramNode (top-level) or any block body.
        let stmts = if let Some(program) = node.as_program_node() {
            Some(program.statements())
        } else {
            None
        };

        if stmts.is_none() {
            // Also check inside example group blocks
            let call = match node.as_call_node() {
                Some(c) => c,
                None => return,
            };
            let name = call.name().as_slice();
            if !is_parent_group(name) {
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
            let inner_stmts = match body.as_statements_node() {
                Some(s) => s,
                None => return,
            };
            diagnostics.extend(check_sibling_groups(self, source, &inner_stmts));
            return;
        }

        let program_stmts = stmts.unwrap();
        diagnostics.extend(check_sibling_groups_from_body(self, source, &program_stmts));
    }
}

fn check_sibling_groups(
    cop: &RepeatedExampleGroupBody,
    source: &SourceFile,
    stmts: &ruby_prism::StatementsNode<'_>,
) -> Vec<Diagnostic> {
    check_sibling_groups_iter(cop, source, stmts.body().iter())
}

fn check_sibling_groups_from_body(
    cop: &RepeatedExampleGroupBody,
    source: &SourceFile,
    stmts: &ruby_prism::StatementsNode<'_>,
) -> Vec<Diagnostic> {
    check_sibling_groups_iter(cop, source, stmts.body().iter())
}

fn check_sibling_groups_iter<'a>(
    cop: &RepeatedExampleGroupBody,
    source: &SourceFile,
    stmts: impl Iterator<Item = ruby_prism::Node<'a>>,
) -> Vec<Diagnostic> {
    // body_signature -> list of (line, col, group_type_name)
    let mut body_map: HashMap<Vec<u8>, Vec<(usize, usize, Vec<u8>)>> = HashMap::new();

    for stmt in stmts {
        let call = match stmt.as_call_node() {
            Some(c) => c,
            None => continue,
        };
        let name = call.name().as_slice();
        if !is_rspec_example_group_for_body(&call) {
            continue;
        }

        let block = match call.block() {
            Some(b) => b,
            None => continue,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => continue,
        };
        let body = match block_node.body() {
            Some(b) => b,
            None => continue,
        };

        // Check for skip/pending-only bodies
        if is_skip_or_pending_body(&body) {
            continue;
        }

        // Build body signature from the full body source.
        // Prism heredoc locations only cover the opening delimiter (<<~FOO),
        // not the heredoc content. We must find the max extent including
        // heredoc closing locations so that bodies with different heredoc
        // content produce different signatures.
        let loc = body.location();
        let mut end_offset = loc.end_offset();
        let mut finder = MaxExtentFinder { max_end: end_offset };
        finder.visit(&body);
        end_offset = finder.max_end;
        let body_src = &source.as_bytes()[loc.start_offset()..end_offset];

        // Also include metadata signature to distinguish groups with different metadata
        let meta_sig = metadata_signature(source, &call);
        let mut sig = meta_sig;
        sig.extend_from_slice(body_src);

        let call_loc = call.location();
        let (line, col) = source.offset_to_line_col(call_loc.start_offset());
        body_map.entry(sig).or_default().push((line, col, name.to_vec()));
    }

    let mut diagnostics = Vec::new();
    for (_sig, locs) in &body_map {
        if locs.len() > 1 {
            for (idx, (line, col, group_name)) in locs.iter().enumerate() {
                let other_lines: Vec<String> = locs.iter().enumerate()
                    .filter(|(i, _)| *i != idx)
                    .map(|(_, (l, _, _))| l.to_string())
                    .collect();
                let group_type = std::str::from_utf8(group_name).unwrap_or("describe");
                // Strip f/x prefix for display
                let display_type = group_type
                    .strip_prefix('f').or(group_type.strip_prefix('x'))
                    .unwrap_or(group_type);
                let msg = format!(
                    "Repeated {} block body on line(s) [{}]",
                    display_type,
                    other_lines.join(", ")
                );
                diagnostics.push(cop.diagnostic(source, *line, *col, msg));
            }
        }
    }

    diagnostics
}

fn is_rspec_example_group_for_body(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    // Must be a describe/context/feature - not shared examples
    if name == b"shared_examples"
        || name == b"shared_examples_for"
        || name == b"shared_context"
    {
        return false;
    }
    if !is_rspec_example_group(name) {
        return false;
    }
    // Must be receiverless or RSpec.describe
    match call.receiver() {
        None => true,
        Some(recv) => {
            if let Some(cr) = recv.as_constant_read_node() {
                cr.name().as_slice() == b"RSpec"
            } else if let Some(cp) = recv.as_constant_path_node() {
                cp.name().map_or(false, |n| n.as_slice() == b"RSpec") && cp.parent().is_none()
            } else {
                false
            }
        }
    }
}

fn metadata_signature(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> Vec<u8> {
    let mut sig = Vec::new();
    if let Some(args) = call.arguments() {
        let arg_list: Vec<_> = args.arguments().iter().collect();
        // Include all args (including first description) for body comparison
        // since groups with same body but different descriptions should still be flagged
        // but groups with different metadata should not
        for (i, arg) in arg_list.iter().enumerate() {
            if i == 0 {
                // Include first arg in metadata sig only if it's a constant (class)
                if arg.as_constant_read_node().is_some() || arg.as_constant_path_node().is_some() {
                    let loc = arg.location();
                    sig.extend_from_slice(&source.as_bytes()[loc.start_offset()..loc.end_offset()]);
                }
                continue;
            }
            let loc = arg.location();
            sig.extend_from_slice(&source.as_bytes()[loc.start_offset()..loc.end_offset()]);
        }
    }
    sig
}

fn is_skip_or_pending_body(body: &ruby_prism::Node<'_>) -> bool {
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };
    let nodes: Vec<_> = stmts.body().iter().collect();
    if nodes.len() != 1 {
        return false;
    }
    if let Some(call) = nodes[0].as_call_node() {
        let name = call.name().as_slice();
        if (name == b"skip" || name == b"pending") && call.block().is_none() {
            return true;
        }
    }
    false
}

/// Visitor that finds the maximum source offset among all descendant nodes,
/// including heredoc closing locations which extend beyond the parent node's range.
struct MaxExtentFinder {
    max_end: usize,
}

impl<'pr> Visit<'pr> for MaxExtentFinder {
    fn visit_interpolated_string_node(
        &mut self,
        node: &ruby_prism::InterpolatedStringNode<'pr>,
    ) {
        if let Some(close) = node.closing_loc() {
            let end = close.end_offset();
            if end > self.max_end {
                self.max_end = end;
            }
        }
        ruby_prism::visit_interpolated_string_node(self, node);
    }

    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        if let Some(close) = node.closing_loc() {
            let end = close.end_offset();
            if end > self.max_end {
                self.max_end = end;
            }
        }
        ruby_prism::visit_string_node(self, node);
    }
}

fn is_parent_group(name: &[u8]) -> bool {
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
            | b"shared_examples"
            | b"shared_examples_for"
            | b"shared_context"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(RepeatedExampleGroupBody, "cops/rspec/repeated_example_group_body");
}
