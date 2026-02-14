use crate::cop::util::{is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use std::collections::HashMap;

/// RSpec/RepeatedExampleGroupDescription: Flag example groups with identical descriptions.
pub struct RepeatedExampleGroupDescription;

impl Cop for RepeatedExampleGroupDescription {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedExampleGroupDescription"
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
        // Check top-level siblings or siblings inside an example group
        let stmts: Vec<ruby_prism::Node<'_>> = if let Some(program) = node.as_program_node() {
            program.statements().body().iter().collect()
        } else if let Some(call) = node.as_call_node() {
            let name = call.name().as_slice();
            if !is_parent_group(name) {
                return Vec::new();
            }
            let block = match call.block() {
                Some(b) => b,
                None => return Vec::new(),
            };
            let block_node = match block.as_block_node() {
                Some(b) => b,
                None => return Vec::new(),
            };
            let body = match block_node.body() {
                Some(b) => b,
                None => return Vec::new(),
            };
            let inner_stmts = match body.as_statements_node() {
                Some(s) => s,
                None => return Vec::new(),
            };
            inner_stmts.body().iter().collect()
        } else {
            return Vec::new();
        };

        // desc_signature -> list of (line, col, group_type_name)
        let mut desc_map: HashMap<Vec<u8>, Vec<(usize, usize, Vec<u8>)>> = HashMap::new();

        for stmt in stmts {
            let call = match stmt.as_call_node() {
                Some(c) => c,
                None => continue,
            };
            if !is_rspec_group_for_desc(&call) {
                continue;
            }

            let name = call.name().as_slice();

            // Extract the description signature (all args)
            let desc_sig = match description_signature(source, &call) {
                Some(s) => s,
                None => continue, // No description
            };

            let loc = call.location();
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            desc_map.entry(desc_sig).or_default().push((line, col, name.to_vec()));
        }

        let mut diagnostics = Vec::new();
        for (_sig, locs) in &desc_map {
            if locs.len() > 1 {
                for (idx, (line, col, group_name)) in locs.iter().enumerate() {
                    let other_lines: Vec<String> = locs.iter().enumerate()
                        .filter(|(i, _)| *i != idx)
                        .map(|(_, (l, _, _))| l.to_string())
                        .collect();
                    let group_type = std::str::from_utf8(group_name).unwrap_or("describe");
                    let display_type = group_type
                        .strip_prefix('f').or(group_type.strip_prefix('x'))
                        .unwrap_or(group_type);
                    let msg = format!(
                        "Repeated {} block description on line(s) [{}]",
                        display_type,
                        other_lines.join(", ")
                    );
                    diagnostics.push(self.diagnostic(source, *line, *col, msg));
                }
            }
        }

        diagnostics
    }
}

fn description_signature(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.is_empty() {
        return None;
    }

    // Build signature from all arguments source text
    let args_loc = args.location();
    let sig = source.as_bytes()[args_loc.start_offset()..args_loc.end_offset()].to_vec();
    Some(sig)
}

fn is_rspec_group_for_desc(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    if name == b"shared_examples"
        || name == b"shared_examples_for"
        || name == b"shared_context"
    {
        return false;
    }
    if !is_rspec_example_group(name) {
        return false;
    }
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

    crate::cop_fixture_tests!(RepeatedExampleGroupDescription, "cops/rspec/repeated_example_group_description");
}
