use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use std::collections::HashMap;

/// RSpec/RepeatedIncludeExample: Flag duplicate include_examples/it_behaves_like calls.
pub struct RepeatedIncludeExample;

const INCLUDE_METHODS: &[&[u8]] = &[
    b"include_examples",
    b"it_behaves_like",
    b"it_should_behave_like",
];

impl Cop for RepeatedIncludeExample {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedIncludeExample"
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

        // signature -> list of (line, col)
        let mut include_map: HashMap<Vec<u8>, Vec<(usize, usize)>> = HashMap::new();

        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                let m = c.name().as_slice();
                if !INCLUDE_METHODS.contains(&m) {
                    continue;
                }
                if c.receiver().is_some() {
                    continue;
                }
                // Skip if has a block (block makes each call unique)
                if c.block().is_some() {
                    continue;
                }

                if let Some(sig) = include_signature(source, &c) {
                    let loc = c.location();
                    let (line, col) = source.offset_to_line_col(loc.start_offset());
                    include_map.entry(sig).or_default().push((line, col));
                }
            }
        }

        let mut diagnostics = Vec::new();
        for (sig_bytes, locs) in &include_map {
            if locs.len() > 1 {
                // Extract the shared example name from the signature
                let shared_name = extract_shared_name(sig_bytes);
                for (idx, &(line, col)) in locs.iter().enumerate() {
                    let other_lines: Vec<String> = locs.iter().enumerate()
                        .filter(|(i, _)| *i != idx)
                        .map(|(_, (l, _))| l.to_string())
                        .collect();
                    let msg = format!(
                        "Repeated include of shared_examples '{}' on line(s) [{}]",
                        shared_name,
                        other_lines.join(", ")
                    );
                    diagnostics.push(self.diagnostic(source, line, col, msg));
                }
            }
        }

        diagnostics
    }
}

fn include_signature(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.is_empty() {
        return None;
    }

    // Check all arguments are literals (no variables)
    for arg in &arg_list {
        if arg.as_call_node().is_some()
            || arg.as_local_variable_read_node().is_some()
            || arg.as_instance_variable_read_node().is_some()
            || arg.as_class_variable_read_node().is_some()
            || arg.as_global_variable_read_node().is_some()
            || arg.as_block_argument_node().is_some()
        {
            return None;
        }
    }

    // Build signature from all argument source text
    let args_loc = args.location();
    Some(source.as_bytes()[args_loc.start_offset()..args_loc.end_offset()].to_vec())
}

fn extract_shared_name(sig_bytes: &[u8]) -> String {
    let s = std::str::from_utf8(sig_bytes).unwrap_or("?");
    // Extract first quoted string
    if let Some(start) = s.find('\'') {
        if let Some(end) = s[start + 1..].find('\'') {
            return s[start + 1..start + 1 + end].to_string();
        }
    }
    if let Some(start) = s.find('"') {
        if let Some(end) = s[start + 1..].find('"') {
            return s[start + 1..start + 1 + end].to_string();
        }
    }
    s.to_string()
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

    crate::cop_fixture_tests!(RepeatedIncludeExample, "cops/rspec/repeated_include_example");
}
