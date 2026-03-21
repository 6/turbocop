use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03):
///
/// ## FPs (3 cases): `::STDOUT = expr` patterns
/// In Prism, `::STDOUT = expr` creates a ConstantPathWriteNode whose target is a
/// ConstantPathNode. The visitor visits the target ConstantPathNode, and since
/// parent() is None (top-level `::STDOUT`), the cop was flagging it. But RuboCop's
/// `on_const` callback is NOT called for constant assignment targets.
///
/// Fix: track `in_const_path_write` flag to suppress flagging ConstantPathNode
/// targets inside ConstantPathWriteNode/OrWriteNode/AndWriteNode/OperatorWriteNode.
///
/// ## FNs (7 cases): Assignment patterns
/// The cop was missing cases like `$stderr = STDOUT` and `$stderr = @stderr = STDERR`.
///
/// Root cause: The `in_gvar_assignment` flag was set for ANY std gvar assignment,
/// which incorrectly suppressed flagging when the assigned constant didn't match
/// the gvar. E.g., `$stderr = STDOUT` should flag STDOUT (since $stderr != $stdout),
/// but the old code suppressed it.
///
/// Fix: Changed `in_gvar_assignment` (bool) to `in_std_gvar_assignment` (Option<&str>)
/// which stores the actual gvar name. A constant is only skipped if its matching
/// gvar equals the gvar being assigned to.
///
/// Additionally, added handlers for InstanceVariableWriteNode, ClassVariableWriteNode,
/// LocalVariableWriteNode, and ConstantWriteNode to clear the gvar context when
/// entering non-gvar assignments. This ensures constants inside chained assignments
/// like `$stderr = @stderr = STDERR` are properly flagged.
pub struct GlobalStdStream;

impl Cop for GlobalStdStream {
    fn name(&self) -> &'static str {
        "Style/GlobalStdStream"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = GlobalStdStreamVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_std_gvar_assignment: None,
            in_const_path_write: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct GlobalStdStreamVisitor<'a, 'src> {
    cop: &'a GlobalStdStream,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// The name of the std gvar being assigned to, if any (e.g., "$stdout")
    /// Used to suppress flagging only when assigning a constant TO its matching gvar.
    in_std_gvar_assignment: Option<&'static str>,
    /// True when visiting inside a ConstantPathWriteNode (target is not a const read)
    in_const_path_write: bool,
}

impl GlobalStdStreamVisitor<'_, '_> {
    fn check_std_stream(&mut self, name_bytes: &[u8], loc: &ruby_prism::Location<'_>) {
        // Check if we're in an assignment to the matching std gvar for this constant.
        // E.g., if we're assigning to $stdout and the constant is STDOUT, skip it.
        // But $stderr = STDOUT should be flagged because $stderr != $stdout.
        if let Some(assigning_to) = self.in_std_gvar_assignment {
            if let Some(matching_gvar) = std_stream_gvar(name_bytes) {
                if matching_gvar == assigning_to {
                    // Assigning the matching constant to its matching gvar - OK
                    return;
                }
            }
        }
        if let Some(gvar) = std_stream_gvar(name_bytes) {
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            let const_name = std::str::from_utf8(name_bytes).unwrap_or("");
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!("Use `{}` instead of `{}`.", gvar, const_name),
            ));
        }
    }
}

impl Visit<'_> for GlobalStdStreamVisitor<'_, '_> {
    fn visit_global_variable_write_node(&mut self, node: &ruby_prism::GlobalVariableWriteNode<'_>) {
        let var_name = node.name();
        let var_bytes = var_name.as_slice();
        // Check if this is $stdout = ..., $stderr = ..., or $stdin = ...
        let std_gvar_name: Option<&'static str> = match var_bytes {
            b"$stdout" => Some("$stdout"),
            b"$stderr" => Some("$stderr"),
            b"$stdin" => Some("$stdin"),
            _ => None,
        };
        if std_gvar_name.is_some() {
            self.in_std_gvar_assignment = std_gvar_name;
        }
        // Visit the value node (the default visitor only visits value)
        self.visit(&node.value());
        if std_gvar_name.is_some() {
            self.in_std_gvar_assignment = None;
        }
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'_>) {
        // When entering a local variable assignment like `foo = STDERR`,
        // temporarily clear the gvar assignment context. The constant on the RHS
        // should still be flagged (unless it's directly assigned to its matching gvar).
        let saved = self.in_std_gvar_assignment.take();
        self.visit(&node.value());
        self.in_std_gvar_assignment = saved;
    }

    fn visit_instance_variable_write_node(&mut self, node: &ruby_prism::InstanceVariableWriteNode<'_>) {
        // When entering an instance variable assignment like `@stderr = STDERR`,
        // temporarily clear the gvar assignment context. The constant on the RHS
        // should still be flagged.
        let saved = self.in_std_gvar_assignment.take();
        self.visit(&node.value());
        self.in_std_gvar_assignment = saved;
    }

    fn visit_class_variable_write_node(&mut self, node: &ruby_prism::ClassVariableWriteNode<'_>) {
        // When entering a class variable assignment like `@@stderr = STDERR`,
        // temporarily clear the gvar assignment context. The constant on the RHS
        // should still be flagged.
        let saved = self.in_std_gvar_assignment.take();
        self.visit(&node.value());
        self.in_std_gvar_assignment = saved;
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'_>) {
        // When entering a constant assignment like `FOO = STDERR`,
        // temporarily clear the gvar assignment context. The constant on the RHS
        // should still be flagged.
        let saved = self.in_std_gvar_assignment.take();
        self.visit(&node.value());
        self.in_std_gvar_assignment = saved;
    }

    fn visit_constant_read_node(&mut self, node: &ruby_prism::ConstantReadNode<'_>) {
        let name = node.name();
        self.check_std_stream(name.as_slice(), &node.location());
    }

    fn visit_constant_path_node(&mut self, node: &ruby_prism::ConstantPathNode<'_>) {
        // Skip constant path write targets (::STDOUT = expr is not a const read)
        if self.in_const_path_write {
            return;
        }
        // Must be top-level (::STDOUT) — parent is None
        if node.parent().is_some() {
            ruby_prism::visit_constant_path_node(self, node);
            return;
        }
        if let Some(name) = node.name() {
            self.check_std_stream(name.as_slice(), &node.location());
        }
        // Don't visit children — we already handled it
    }

    fn visit_constant_path_write_node(&mut self, node: &ruby_prism::ConstantPathWriteNode<'_>) {
        // The target ConstantPathNode is a write target, not a const read.
        // Set flag so visit_constant_path_node skips it, then visit value normally.
        self.in_const_path_write = true;
        self.visit_constant_path_node(&node.target());
        self.in_const_path_write = false;
        // Visit the value side normally (it may contain STDOUT references)
        self.visit(&node.value());
    }

    fn visit_constant_path_or_write_node(
        &mut self,
        node: &ruby_prism::ConstantPathOrWriteNode<'_>,
    ) {
        self.in_const_path_write = true;
        self.visit_constant_path_node(&node.target());
        self.in_const_path_write = false;
        self.visit(&node.value());
    }

    fn visit_constant_path_and_write_node(
        &mut self,
        node: &ruby_prism::ConstantPathAndWriteNode<'_>,
    ) {
        self.in_const_path_write = true;
        self.visit_constant_path_node(&node.target());
        self.in_const_path_write = false;
        self.visit(&node.value());
    }

    fn visit_constant_path_operator_write_node(
        &mut self,
        node: &ruby_prism::ConstantPathOperatorWriteNode<'_>,
    ) {
        self.in_const_path_write = true;
        self.visit_constant_path_node(&node.target());
        self.in_const_path_write = false;
        self.visit(&node.value());
    }
}

fn std_stream_gvar(name: &[u8]) -> Option<&'static str> {
    match name {
        b"STDOUT" => Some("$stdout"),
        b"STDERR" => Some("$stderr"),
        b"STDIN" => Some("$stdin"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(GlobalStdStream, "cops/style/global_std_stream");
}
