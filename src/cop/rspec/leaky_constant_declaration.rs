use crate::cop::util::{self, is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, CLASS_NODE, CONSTANT_READ_NODE, CONSTANT_WRITE_NODE, MODULE_NODE, STATEMENTS_NODE};

pub struct LeakyConstantDeclaration;

/// Flags constant assignments (`CONST = ...`), class definitions, and module
/// definitions inside RSpec example groups. These leak into the global namespace.
impl Cop for LeakyConstantDeclaration {
    fn name(&self) -> &'static str {
        "RSpec/LeakyConstantDeclaration"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, CLASS_NODE, CONSTANT_READ_NODE, CONSTANT_WRITE_NODE, MODULE_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Look for describe/context/shared_examples blocks and check their body
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();

        let is_example_group = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).map_or(false, |n| n == b"RSpec") && is_rspec_example_group(method_name)
        } else {
            is_rspec_example_group(method_name)
        };

        if !is_example_group {
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

        let mut diagnostics = Vec::new();
        scan_for_leaky_constants(source, block_node, &mut diagnostics, self);
        diagnostics
    }
}

fn scan_for_leaky_constants(
    source: &SourceFile,
    block: ruby_prism::BlockNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &LeakyConstantDeclaration,
) {
    let body = match block.body() {
        Some(b) => b,
        None => return,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return,
    };

    for stmt in stmts.body().iter() {
        check_leaky_node(source, &stmt, diagnostics, cop);
    }
}

fn check_leaky_node(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &LeakyConstantDeclaration,
) {
    // Check for CONSTANT = value
    if let Some(cw) = node.as_constant_write_node() {
        let loc = cw.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            "Stub constant instead of declaring explicitly.".to_string(),
        ));
        return;
    }

    // Check for class Foo (but not class self::Foo, class Foo::Bar, class ::Foo).
    // Note: constant_path_node is intentionally not matched here — qualified constants
    // like `class Foo::Bar` don't leak in the same way as bare `class Foo`.
    if let Some(class_node) = node.as_class_node() {
        let const_path = class_node.constant_path();
        // If the constant path is a simple ConstantReadNode, it's a bare class name
        if const_path.as_constant_read_node().is_some() {
            let loc = class_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(cop.diagnostic(
                source,
                line,
                column,
                "Stub class constant instead of declaring explicitly.".to_string(),
            ));
        }
        return;
    }

    // Check for module Foo (but not module self::Foo, module Foo::Bar, module ::Foo)
    if let Some(module_node) = node.as_module_node() {
        let const_path = module_node.constant_path();
        if const_path.as_constant_read_node().is_some() {
            let loc = module_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(cop.diagnostic(
                source,
                line,
                column,
                "Stub module constant instead of declaring explicitly.".to_string(),
            ));
        }
        return;
    }

    // Recurse into nested example groups and example blocks
    if let Some(call) = node.as_call_node() {
        if let Some(block) = call.block() {
            if let Some(bn) = block.as_block_node() {
                let name = call.name().as_slice();
                if is_rspec_example_group(name) {
                    // Nested example group — also scan it
                    scan_for_leaky_constants(source, bn, diagnostics, cop);
                } else {
                    // Example/hook blocks — also scan for constants inside
                    scan_for_leaky_constants(source, bn, diagnostics, cop);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        LeakyConstantDeclaration,
        "cops/rspec/leaky_constant_declaration"
    );
}
