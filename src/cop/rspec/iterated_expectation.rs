use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct IteratedExpectation;

impl Cop for IteratedExpectation {
    fn name(&self) -> &'static str {
        "RSpec/IteratedExpectation"
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
        // Flag `.each { |x| expect(x)... }` — suggest using `all` matcher
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"each" {
            return Vec::new();
        }

        // Must have a receiver (the array/collection)
        if call.receiver().is_none() {
            return Vec::new();
        }

        // Must have a block with a parameter
        let block_raw = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block = match block_raw.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Must have block parameters
        let params = match block.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let block_params = match params.as_block_parameters_node() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let inner_params = match block_params.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let requireds: Vec<_> = inner_params.requireds().iter().collect();
        if requireds.is_empty() {
            return Vec::new();
        }

        // Check if the parameter starts with _ (unused)
        if let Some(first_param) = requireds.first() {
            if let Some(req) = first_param.as_required_parameter_node() {
                if req.name().as_slice().starts_with(b"_") {
                    return Vec::new();
                }
            }
        }

        // Check if the block body contains an `expect` call
        let body = match block.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        if contains_expect(&body) {
            // Flag the receiver + `.each` part
            let recv = call.receiver().unwrap();
            let loc = recv.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let each_end = call.message_loc().map(|m| m.end_offset()).unwrap_or(loc.end_offset());
            let _ = each_end;
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Prefer using the `all` matcher instead of iterating over an array.".to_string(),
            )];
        }

        Vec::new()
    }
}

fn contains_expect(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none() {
            let name = call.name().as_slice();
            if name == b"expect" || name == b"expect_any_instance_of" {
                return true;
            }
        }
        // Check the receiver chain
        if let Some(recv) = call.receiver() {
            if contains_expect(&recv) {
                return true;
            }
        }
    }

    if let Some(stmts) = node.as_statements_node() {
        for child in stmts.body().iter() {
            if contains_expect(&child) {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IteratedExpectation, "cops/rspec/iterated_expectation");
}
