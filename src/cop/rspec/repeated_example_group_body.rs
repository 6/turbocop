use crate::cop::node_type::{CALL_NODE, PROGRAM_NODE};
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
///
/// Investigation (2026-03-11):
/// - FP=12: AstHashVisitor was missing handlers for RangeNode (exclude_end flag)
///   and XStringNode (backtick string content). Bodies differing only in `..`
///   vs `...` or different backtick command strings were hashed identically.
/// - FN=66: check_node only recursed into known parent groups (is_parent_group
///   list). Example groups inside non-RSpec blocks (e.g. InSpec's `control`)
///   were not compared. Fixed by checking siblings inside ANY block body,
///   matching RuboCop's on_begin approach.
///
/// ## Corpus investigation (2026-03-14)
///
/// Corpus oracle reported FP=2, FN=28.
///
/// FN=28: All from describe blocks inside `if`/`elsif`/`else` branches at the top level
/// of spec files (e.g., InSpec control specs with platform conditionals). The cop only
/// checked sibling groups at the ProgramNode level and inside block bodies, but never
/// recursed into if/elsif/else/case branch bodies. Fixed by adding
/// `collect_branch_diagnostics()` which recursively walks if/elsif/else/unless/case
/// branches and calls `check_sibling_groups` on each branch's StatementsNode.
///
/// FP=2: Both in rom-rb/rom `core/spec/unit/rom/commands/pre_and_post_processors_spec.rb`
/// at lines 100 and 148. Two `context` blocks with bodies containing
/// `Class.new(ROM::Commands::Create[:memory]) do result :many; before :prepare;
/// after :finalize; def execute(tuples)... end end`. Both contexts start with identical
/// `subject(:command) do Class.new(...) do result :many; before :prepare; after :finalize;
/// def execute(tuples)` but the `def execute` method bodies are cut off in the corpus
/// context. Source file not in local corpus — cannot inspect full bodies.
/// Root cause hypothesis: the `def execute` bodies differ in some way that the
/// AstHashVisitor does not distinguish. Likely a missing hash handler for a node type
/// that appears in the `execute` body (e.g., multi-assign, splat, or complex literal).
/// No fix attempted without concrete reproduction.
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
        &[PROGRAM_NODE, CALL_NODE]
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
        // Handle top-level statements
        if let Some(program) = node.as_program_node() {
            diagnostics.extend(check_sibling_groups(self, source, &program.statements()));
            return;
        }

        // For ANY CallNode with a block: check inside the block body for sibling
        // example groups. This matches RuboCop's on_begin approach — Parser creates
        // `begin` nodes for multi-statement bodies at every level. Previously we
        // only checked inside known parent groups (is_parent_group list), missing
        // example groups inside non-RSpec blocks like InSpec's `control`.
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
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
    }
}

fn check_sibling_groups(
    cop: &RepeatedExampleGroupBody,
    source: &SourceFile,
    stmts: &ruby_prism::StatementsNode<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = check_sibling_groups_iter(cop, source, stmts.body().iter());
    // Recurse into if/elsif/else/unless/case branches to find sibling groups there.
    // RuboCop's on_begin fires for every multi-statement body including branch bodies.
    for stmt in stmts.body().iter() {
        collect_branch_diagnostics(cop, source, &stmt, &mut diagnostics);
    }
    diagnostics
}

/// Recursively collect diagnostics from if/elsif/else/unless/case branch bodies.
fn collect_branch_diagnostics(
    cop: &RepeatedExampleGroupBody,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            diagnostics.extend(check_sibling_groups(cop, source, &stmts));
        }
        if let Some(subsequent) = if_node.subsequent() {
            collect_branch_diagnostics(cop, source, &subsequent, diagnostics);
        }
    } else if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            diagnostics.extend(check_sibling_groups(cop, source, &stmts));
        }
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                diagnostics.extend(check_sibling_groups(cop, source, &stmts));
            }
        }
    } else if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            diagnostics.extend(check_sibling_groups(cop, source, &stmts));
        }
    } else if let Some(case_node) = node.as_case_node() {
        for condition in case_node.conditions().iter() {
            if let Some(when_node) = condition.as_when_node() {
                if let Some(stmts) = when_node.statements() {
                    diagnostics.extend(check_sibling_groups(cop, source, &stmts));
                }
            }
        }
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                diagnostics.extend(check_sibling_groups(cop, source, &stmts));
            }
        }
    }
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
        let name = call.name().as_slice();
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

    fn visit_range_node(&mut self, node: &ruby_prism::RangeNode<'pr>) {
        // Hash the operator source (.. vs ...) to distinguish inclusive/exclusive ranges.
        // Without this, `1..99` and `1...99` hash identically (same RangeNode discriminant
        // and same children) causing false positives.
        let op_loc = node.operator_loc();
        self.src[op_loc.start_offset()..op_loc.end_offset()].hash(self.hasher);
        ruby_prism::visit_range_node(self, node);
    }

    fn visit_x_string_node(&mut self, node: &ruby_prism::XStringNode<'pr>) {
        // Hash backtick string content. Without this, all XStringNode values
        // hash identically (same discriminant, no children) causing false positives.
        node.unescaped().hash(self.hasher);
    }

    fn visit_interpolated_x_string_node(
        &mut self,
        node: &ruby_prism::InterpolatedXStringNode<'pr>,
    ) {
        // Hash interpolated backtick string content via default child traversal.
        ruby_prism::visit_interpolated_x_string_node(self, node);
    }

    fn visit_required_parameter_node(&mut self, node: &ruby_prism::RequiredParameterNode<'pr>) {
        // Hash parameter names — without this, `def foo(a)` and `def foo(b)` hash identically.
        node.name().as_slice().hash(self.hasher);
    }

    fn visit_optional_parameter_node(&mut self, node: &ruby_prism::OptionalParameterNode<'pr>) {
        node.name().as_slice().hash(self.hasher);
        ruby_prism::visit_optional_parameter_node(self, node);
    }

    fn visit_rest_parameter_node(&mut self, node: &ruby_prism::RestParameterNode<'pr>) {
        if let Some(name) = node.name() {
            name.as_slice().hash(self.hasher);
        }
    }

    fn visit_keyword_rest_parameter_node(
        &mut self,
        node: &ruby_prism::KeywordRestParameterNode<'pr>,
    ) {
        if let Some(name) = node.name() {
            name.as_slice().hash(self.hasher);
        }
    }

    fn visit_block_parameter_node(&mut self, node: &ruby_prism::BlockParameterNode<'pr>) {
        if let Some(name) = node.name() {
            name.as_slice().hash(self.hasher);
        }
    }

    fn visit_required_keyword_parameter_node(
        &mut self,
        node: &ruby_prism::RequiredKeywordParameterNode<'pr>,
    ) {
        node.name().as_slice().hash(self.hasher);
    }

    fn visit_optional_keyword_parameter_node(
        &mut self,
        node: &ruby_prism::OptionalKeywordParameterNode<'pr>,
    ) {
        node.name().as_slice().hash(self.hasher);
        ruby_prism::visit_optional_keyword_parameter_node(self, node);
    }

    fn visit_block_local_variable_node(&mut self, node: &ruby_prism::BlockLocalVariableNode<'pr>) {
        node.name().as_slice().hash(self.hasher);
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        node.name().as_slice().hash(self.hasher);
    }

    fn visit_instance_variable_target_node(
        &mut self,
        node: &ruby_prism::InstanceVariableTargetNode<'pr>,
    ) {
        node.name().as_slice().hash(self.hasher);
    }

    fn visit_class_variable_target_node(
        &mut self,
        node: &ruby_prism::ClassVariableTargetNode<'pr>,
    ) {
        node.name().as_slice().hash(self.hasher);
    }

    fn visit_global_variable_target_node(
        &mut self,
        node: &ruby_prism::GlobalVariableTargetNode<'pr>,
    ) {
        node.name().as_slice().hash(self.hasher);
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        node.name().as_slice().hash(self.hasher);
        ruby_prism::visit_constant_write_node(self, node);
    }

    fn visit_constant_target_node(&mut self, node: &ruby_prism::ConstantTargetNode<'pr>) {
        node.name().as_slice().hash(self.hasher);
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
    fn skip_with_args_excluded() {
        let source = b"
describe '#load' do
  skip 'storage feature needed'
end

describe '#save' do
  skip 'storage feature needed'
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            0,
            "skip with args should be excluded: {:?}",
            diags
        );
    }

    #[test]
    fn pending_with_args_excluded() {
        let source = b"
describe '#get_foo' do
  pending 'foo feature is broken'
end

describe '#set_foo' do
  pending 'foo feature is broken'
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            0,
            "pending with args should be excluded: {:?}",
            diags
        );
    }

    #[test]
    fn skip_with_block_not_excluded() {
        let source = b"
describe '#load' do
  skip { cool_predicate_method }
end

describe '#save' do
  skip { cool_predicate_method }
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            2,
            "skip with block should NOT be excluded: {:?}",
            diags
        );
    }

    #[test]
    fn cross_group_type_detection() {
        // describe and context with same body should match
        let source = b"
describe 'doing x' do
  it { cool_predicate_method }
end

context 'when a is true' do
  it { cool_predicate_method }
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            2,
            "describe and context with same body should match: {:?}",
            diags
        );
    }

    #[test]
    fn different_metadata_no_offense() {
        let source = b"
describe 'doing x' do
  it { cool_predicate_method }
end

describe 'doing x', :request do
  it { cool_predicate_method }
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            0,
            "different metadata should not match: {:?}",
            diags
        );
    }

    #[test]
    fn different_const_arg_no_offense() {
        let source = b"
describe CSV::Row do
  it { is_expected.to respond_to :headers }
end

describe CSV::Table do
  it { is_expected.to respond_to :headers }
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            0,
            "different const args should not match: {:?}",
            diags
        );
    }

    #[test]
    fn same_const_arg_offense() {
        let source = b"
context Net::HTTP do
  it { expect(described_class).to respond_to :start }
end

context Net::HTTP do
  it { expect(described_class).to respond_to :start }
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            2,
            "same const args with same body should match: {:?}",
            diags
        );
    }

    #[test]
    fn different_scopes_no_offense() {
        // Groups at different nesting levels should not match
        let source = b"
describe 'A' do
  describe '.b' do
    context 'when this' do
      it { do_something }
    end
  end
  context 'when this' do
    it { do_something }
  end
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            0,
            "groups at different nesting levels should not match: {:?}",
            diags
        );
    }

    #[test]
    fn separated_by_non_group_siblings() {
        // Groups separated by non-example-group code should still match
        let source = b"
describe 'repeated' do
  it { is_expected.to be_truthy }
end

before { do_something }

describe 'this is repeated' do
  it { is_expected.to be_truthy }
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            2,
            "groups separated by non-group code should still match: {:?}",
            diags
        );
    }

    #[test]
    fn no_descriptions_same_body() {
        // context without descriptions but same body should match
        let source = b"
context do
  let(:preferences) { %w[a] }

  it { is_expected.to eq true }
end

context do
  let(:preferences) { %w[a] }

  it { is_expected.to eq true }
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            2,
            "context without descriptions but same body should match: {:?}",
            diags
        );
    }

    #[test]
    fn rspec_prefix_mixed_with_bare() {
        // RSpec.describe and bare context should match if same body
        let source = b"
RSpec.describe 'doing x' do
  it { cool_predicate_method }
end

context 'when a is true' do
  it { cool_predicate_method }
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            2,
            "RSpec.describe and bare context with same body should match: {:?}",
            diags
        );
    }

    #[test]
    fn helpers_describe_excluded() {
        // helpers.describe should be excluded from comparison
        let source = b"
helpers.describe 'doing x' do
  it { cool_predicate_method }
end

RSpec.describe 'doing x' do
  it { cool_predicate_method }
end

context 'when a is true' do
  it { cool_predicate_method }
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            2,
            "helpers.describe should be excluded, RSpec.describe and context should match: {:?}",
            diags
        );
    }

    #[test]
    fn nested_repeated_groups() {
        // Repeated groups nested inside another group
        let source = b"
RSpec.describe 'A' do
  stub_all_http_calls()
  before { create(:admin) }

  describe '#load' do
    it { cool_predicate_method }
  end

  describe '#load' do
    it { cool_predicate_method }
  end
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            2,
            "nested repeated groups should match: {:?}",
            diags
        );
    }

    #[test]
    fn groups_inside_non_example_group_block() {
        // FN root cause: describe blocks inside a non-example-group block like `control`
        // RuboCop's on_begin fires at any level; nitrocop must check siblings everywhere
        let source = b"
control 'test-01' do
  describe 'first check' do
    it { should eq 0 }
  end
  describe 'second check' do
    it { should eq 0 }
  end
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            2,
            "describe blocks inside non-example-group block should be compared: {:?}",
            diags
        );
    }

    #[test]
    fn range_operator_difference_not_flagged() {
        // FP root cause: .. vs ... range operators produce same RangeNode discriminant
        // but differ in exclude_end flag which must be hashed
        let source = b"
describe 'included' do
  before { @range = 1..99 }
  it { @range.should include 50 }
end

describe 'excluded' do
  before { @range = 1...99 }
  it { @range.should include 50 }
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            0,
            "bodies with .. vs ... ranges should not match: {:?}",
            diags
        );
    }

    #[test]
    fn xstring_content_difference_not_flagged() {
        // FP root cause: backtick strings (XStringNode) content not hashed
        let source = b"
context 'case a' do
  before { `echo hello` }
  it { should be_truthy }
end

context 'case b' do
  before { `echo world` }
  it { should be_truthy }
end
";
        let diags = crate::testutil::run_cop_full(&RepeatedExampleGroupBody, source);
        assert_eq!(
            diags.len(),
            0,
            "bodies with different backtick strings should not match: {:?}",
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
