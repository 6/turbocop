use crate::cop::util::{is_rspec_hook, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};

pub struct ExpectInHook;

/// Expectation methods to flag inside hooks.
const EXPECT_METHODS: &[&[u8]] = &[b"expect", b"is_expected", b"expect_any_instance_of"];

impl Cop for ExpectInHook {
    fn name(&self) -> &'static str {
        "RSpec/ExpectInHook"
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
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.receiver().is_some() {
            return Vec::new();
        }

        let method_name = call.name().as_slice();
        if !is_rspec_hook(method_name) {
            return Vec::new();
        }

        let hook_name = std::str::from_utf8(method_name).unwrap_or("hook");

        // Check the block body for expect calls
        let block_raw = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block = match block_raw.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let body = match block.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();
        find_expects_in_node(&body, source, self, hook_name, &mut diagnostics);
        diagnostics
    }
}

fn find_expects_in_node(
    node: &ruby_prism::Node<'_>,
    source: &SourceFile,
    cop: &ExpectInHook,
    hook_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none() {
            let name = call.name().as_slice();
            if EXPECT_METHODS.iter().any(|m| name == *m) {
                let method_str = std::str::from_utf8(name).unwrap_or("expect");
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(cop.diagnostic(
                    source,
                    line,
                    column,
                    format!("Do not use `{method_str}` in `{hook_name}` hook"),
                ));
                return;
            }
        }
        // Check receiver chain (e.g., expect(x).to eq(...))
        if let Some(recv) = call.receiver() {
            find_expects_in_node(&recv, source, cop, hook_name, diagnostics);
        }
    }

    if let Some(stmts) = node.as_statements_node() {
        for child in stmts.body().iter() {
            find_expects_in_node(&child, source, cop, hook_name, diagnostics);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExpectInHook, "cops/rspec/expect_in_hook");
}
