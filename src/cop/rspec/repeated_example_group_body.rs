use crate::cop::node_type::{
    BLOCK_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, PROGRAM_NODE, STATEMENTS_NODE,
};
use crate::cop::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// RSpec/RepeatedExampleGroupBody: Flag example groups with identical bodies.
///
/// Compares example group bodies using AST-based structural hashing rather than
/// raw source bytes. This matches RuboCop's Parser gem behavior where:
/// - `'foo'` and `"foo"` (no interpolation) are considered identical
/// - `foo(1)` and `foo 1` (optional parens) are considered identical
/// - Comments are ignored (Prism separates them from the AST)
///
/// Root cause of 82 FN was source-byte comparison failing on syntactically
/// equivalent but textually different bodies.
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
        &[
            BLOCK_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            PROGRAM_NODE,
            STATEMENTS_NODE,
        ]
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
        // We need to look at sibling example groups within a common parent.
        // The parent can be a ProgramNode (top-level) or any block body.
        let stmts = node.as_program_node().map(|program| program.statements());

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
    #[allow(clippy::type_complexity)] // internal collection used only in this function
    let mut body_map: HashMap<u64, Vec<(usize, usize, Vec<u8>)>> = HashMap::new();

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

        // Build AST-based body signature. This matches RuboCop's behavior of comparing
        // AST structure rather than source text, so bodies that differ only in:
        // - string quoting ('foo' vs "foo")
        // - optional parentheses (eq(1) vs eq 1)
        // - whitespace/formatting
        // are considered identical.
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let mut visitor = AstHashVisitor {
            hasher: &mut hasher,
            src: source.as_bytes(),
        };
        visitor.visit(&body);

        // Also include metadata signature to distinguish groups with different metadata
        metadata_hash(source, &call, &mut hasher);

        let sig = hasher.finish();

        let call_loc = call.location();
        let (line, col) = source.offset_to_line_col(call_loc.start_offset());
        body_map
            .entry(sig)
            .or_default()
            .push((line, col, name.to_vec()));
    }

    let mut diagnostics = Vec::new();
    for locs in body_map.values() {
        if locs.len() > 1 {
            for (idx, (line, col, group_name)) in locs.iter().enumerate() {
                let other_lines: Vec<String> = locs
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != idx)
                    .map(|(_, (l, _, _))| l.to_string())
                    .collect();
                let group_type = std::str::from_utf8(group_name).unwrap_or("describe");
                // Strip f/x prefix for display
                let display_type = group_type
                    .strip_prefix('f')
                    .or(group_type.strip_prefix('x'))
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
    if name == b"shared_examples" || name == b"shared_examples_for" || name == b"shared_context" {
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
                cp.name().is_some_and(|n| n.as_slice() == b"RSpec") && cp.parent().is_none()
            } else {
                false
            }
        }
    }
}

fn metadata_hash(source: &SourceFile, call: &ruby_prism::CallNode<'_>, hasher: &mut impl Hasher) {
    if let Some(args) = call.arguments() {
        let arg_list: Vec<_> = args.arguments().iter().collect();
        for (i, arg) in arg_list.iter().enumerate() {
            if i == 0 {
                // Include first arg in signature only if it's a constant (class)
                // RuboCop's const_arg matcher: (block (send _ _ $const ...) ...)
                if arg.as_constant_read_node().is_some() || arg.as_constant_path_node().is_some() {
                    b"CONST_ARG:".hash(hasher);
                    let mut visitor = AstHashVisitor {
                        hasher,
                        src: source.as_bytes(),
                    };
                    visitor.visit(arg);
                }
                continue;
            }
            // Metadata args (everything after the first arg)
            b"META:".hash(hasher);
            let mut visitor = AstHashVisitor {
                hasher,
                src: source.as_bytes(),
            };
            visitor.visit(arg);
        }
    }
}

/// AST-based structural hasher that produces identical hashes for
/// syntactically equivalent code regardless of formatting.
///
/// Uses Prism's Visit trait to traverse the AST. For each node,
/// `visit_branch_node_enter` / `visit_leaf_node_enter` hashes the node type.
/// Specific visitor overrides hash additional semantic content (names, values).
/// This means:
/// - `'foo'` and `"foo"` hash identically (unescaped content is the same)
/// - `foo(1)` and `foo 1` hash identically (paren presence is not hashed)
/// - Comments are not part of the Prism AST, so naturally ignored
struct AstHashVisitor<'a, H: Hasher> {
    hasher: &'a mut H,
    src: &'a [u8],
}

impl<'a, 'pr, H: Hasher> Visit<'pr> for AstHashVisitor<'a, H> {
    // These two callbacks fire for every node during default traversal,
    // providing the type discriminant hash for both handled and unhandled nodes.
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        std::mem::discriminant(&node).hash(self.hasher);
    }
    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        std::mem::discriminant(&node).hash(self.hasher);
    }

    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        // Hash unescaped content — makes 'foo' and "foo" equivalent
        node.unescaped().hash(self.hasher);
        // Leaf: no children to recurse into
    }

    fn visit_symbol_node(&mut self, node: &ruby_prism::SymbolNode<'pr>) {
        node.unescaped().hash(self.hasher);
    }

    fn visit_integer_node(&mut self, node: &ruby_prism::IntegerNode<'pr>) {
        let loc = node.location();
        self.src[loc.start_offset()..loc.end_offset()].hash(self.hasher);
    }

    fn visit_float_node(&mut self, node: &ruby_prism::FloatNode<'pr>) {
        let loc = node.location();
        self.src[loc.start_offset()..loc.end_offset()].hash(self.hasher);
    }

    fn visit_regular_expression_node(&mut self, node: &ruby_prism::RegularExpressionNode<'pr>) {
        node.unescaped().hash(self.hasher);
        let close = node.closing_loc();
        self.src[close.start_offset()..close.end_offset()].hash(self.hasher);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Hash method name
        node.name().as_slice().hash(self.hasher);
        // Hash call operator type (&. vs . vs none)
        if let Some(op) = node.call_operator_loc() {
            let op_bytes = &self.src[op.start_offset()..op.end_offset()];
            op_bytes.hash(self.hasher);
        }
        // Recurse into receiver, arguments, and block — but NOT parens.
        // Parser gem treats foo(1) and foo 1 as identical AST, so we
        // intentionally skip opening_loc/closing_loc.
        if let Some(recv) = node.receiver() {
            b"R".hash(self.hasher);
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                b"A".hash(self.hasher);
                self.visit(&arg);
            }
        }
        if let Some(block) = node.block() {
            b"B".hash(self.hasher);
            self.visit(&block);
        }
        // Do NOT call ruby_prism::visit_call_node — we handle children ourselves
    }

    fn visit_constant_read_node(&mut self, node: &ruby_prism::ConstantReadNode<'pr>) {
        node.name().as_slice().hash(self.hasher);
    }

    fn visit_constant_path_node(&mut self, node: &ruby_prism::ConstantPathNode<'pr>) {
        if let Some(parent) = node.parent() {
            b"P".hash(self.hasher);
            self.visit(&parent);
        }
        if let Some(name) = node.name() {
            name.as_slice().hash(self.hasher);
        }
        // Do NOT call default recursion — handled above
    }

    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        node.name().as_slice().hash(self.hasher);
    }

    fn visit_instance_variable_read_node(
        &mut self,
        node: &ruby_prism::InstanceVariableReadNode<'pr>,
    ) {
        node.name().as_slice().hash(self.hasher);
    }

    fn visit_class_variable_read_node(&mut self, node: &ruby_prism::ClassVariableReadNode<'pr>) {
        node.name().as_slice().hash(self.hasher);
    }

    fn visit_global_variable_read_node(&mut self, node: &ruby_prism::GlobalVariableReadNode<'pr>) {
        node.name().as_slice().hash(self.hasher);
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        node.name().as_slice().hash(self.hasher);
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        node.name().as_slice().hash(self.hasher);
        ruby_prism::visit_instance_variable_write_node(self, node);
    }

    fn visit_class_variable_write_node(&mut self, node: &ruby_prism::ClassVariableWriteNode<'pr>) {
        node.name().as_slice().hash(self.hasher);
        ruby_prism::visit_class_variable_write_node(self, node);
    }

    fn visit_global_variable_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableWriteNode<'pr>,
    ) {
        node.name().as_slice().hash(self.hasher);
        ruby_prism::visit_global_variable_write_node(self, node);
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        node.name().as_slice().hash(self.hasher);
        ruby_prism::visit_def_node(self, node);
    }
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

    crate::cop_fixture_tests!(
        RepeatedExampleGroupBody,
        "cops/rspec/repeated_example_group_body"
    );

    #[test]
    fn detects_identical_bodies_with_different_string_quoting() {
        // RuboCop's AST comparison treats 'foo' and "foo" (no interpolation) as identical
        let source = br#"
describe 'case a' do
  it { expect(subject).to eq('hello') }
end

describe 'case b' do
  it { expect(subject).to eq("hello") }
end
"#;
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            2,
            "Expected 2 offenses for identical bodies with different quoting, got: {:?}",
            diags
        );
    }

    #[test]
    fn detects_identical_bodies_with_optional_parens() {
        // RuboCop's AST comparison treats foo(1) and foo 1 as identical
        let source = b"
describe 'case a' do
  it { expect(subject).to eq(1) }
end

describe 'case b' do
  it { expect(subject).to eq 1 }
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            2,
            "Expected 2 offenses for identical bodies with different parens, got: {:?}",
            diags
        );
    }

    #[test]
    fn detects_identical_bodies_with_comments_diff() {
        // RuboCop's AST ignores comments; bodies differing only in comments should match
        let source = b"
describe 'case a' do
  # this is a comment
  it { do_something }
end

describe 'case b' do
  it { do_something }
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            2,
            "Expected 2 offenses for bodies differing only in comments, got: {:?}",
            diags
        );
    }
}
