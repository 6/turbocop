use crate::cop::util::{is_rspec_example, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use std::collections::HashMap;

/// RSpec/RepeatedExample: Don't repeat examples (same body) within an example group.
pub struct RepeatedExample;

impl Cop for RepeatedExample {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedExample"
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

        let name = call.name().as_slice();
        if !is_example_group(name) {
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
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Collect examples: body_signature -> list of (line, col)
        // Body signature = source bytes of the block body + all metadata args
        let mut body_map: HashMap<Vec<u8>, Vec<(usize, usize)>> = HashMap::new();

        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                let m = c.name().as_slice();
                if is_rspec_example(m) || m == b"its" {
                    if let Some(sig) = example_body_signature(source, &c) {
                        let loc = c.location();
                        let (line, col) = source.offset_to_line_col(loc.start_offset());
                        body_map.entry(sig).or_default().push((line, col));
                    }
                }
            }
        }

        let mut diagnostics = Vec::new();
        for (_sig, locs) in &body_map {
            if locs.len() > 1 {
                for (idx, &(line, col)) in locs.iter().enumerate() {
                    let other_lines: Vec<String> = locs.iter().enumerate()
                        .filter(|(i, _)| *i != idx)
                        .map(|(_, (l, _))| l.to_string())
                        .collect();
                    let msg = format!(
                        "Don't repeat examples within an example group. Repeated on line(s) {}.",
                        other_lines.join(", ")
                    );
                    diagnostics.push(self.diagnostic(source, line, col, msg));
                }
            }
        }

        diagnostics
    }
}

/// Build a signature from the example's block body + metadata (excluding description).
/// Two examples with same body and metadata are duplicates.
fn example_body_signature(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let mut sig = Vec::new();

    // Include metadata args (skip the first string/symbol description if present)
    if let Some(args) = call.arguments() {
        let arg_list: Vec<_> = args.arguments().iter().collect();
        for (i, arg) in arg_list.iter().enumerate() {
            // Skip first argument if it's a string (description)
            if i == 0 && (arg.as_string_node().is_some() || arg.as_interpolated_string_node().is_some()) {
                continue;
            }
            let loc = arg.location();
            sig.extend_from_slice(&source.as_bytes()[loc.start_offset()..loc.end_offset()]);
            sig.push(b',');
        }
    }

    // Include block body
    if let Some(block) = call.block() {
        if let Some(block_node) = block.as_block_node() {
            if let Some(body) = block_node.body() {
                let loc = body.location();
                sig.extend_from_slice(&source.as_bytes()[loc.start_offset()..loc.end_offset()]);
            }
        }
    }

    if sig.is_empty() {
        return None;
    }

    Some(sig)
}

fn is_example_group(name: &[u8]) -> bool {
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

    crate::cop_fixture_tests!(RepeatedExample, "cops/rspec/repeated_example");
}
