use crate::cop::node_type::{BLOCK_NODE, CALL_NODE};
use crate::cop::util::{
    RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group, is_rspec_shared_group,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashMap;

/// RSpec/RepeatedExample: Don't repeat examples (same body) within an example group.
///
/// **Investigation (2026-03-04):** 88 FPs caused by `its()` calls with different string
/// attributes but same block body being treated as duplicates. The `example_body_signature()`
/// function was skipping the first string arg (treating it as a description like `it`), but
/// for `its`, the first string arg is the attribute accessor (e.g., `its('Server.Version')`).
/// Fix: include the first string arg in the signature when the method is `its`.
///
/// **Investigation (2026-03-05):** 893 FNs and 22 FPs caused by raw source-text comparison
/// for example body signatures. RuboCop uses AST structural equality, meaning examples with
/// the same AST but different formatting (e.g., `do..end` vs `{ }`, different indentation,
/// semicolons vs newlines) are correctly identified as duplicates. Raw source comparison
/// missed all of these.
///
/// Root cause of FNs: identical example bodies with different whitespace/formatting produced
/// different raw source signatures, so they were not detected as duplicates.
///
/// Root cause of FPs: metadata args (like `:focus` tags) were compared as raw source text
/// which could accidentally match in edge cases.
///
/// Fix: replaced raw source comparison with AST-based structural fingerprinting. The new
/// `AstFingerprinter` walks the AST recursively, emitting node type tags and literal values
/// (strings, symbols, integers, identifiers) but ignoring whitespace and source locations.
/// This produces a canonical representation matching RuboCop's AST equality semantics.
///
/// The signature now consists of:
/// 1. Metadata args (everything after the first string description arg) — AST fingerprint
/// 2. Block body (the implementation) — AST fingerprint
/// 3. For `its()` calls, the first arg (attribute accessor) is also included
///
/// **Investigation (2026-03-10):** 302 FPs and 705 FNs remaining.
///
/// FN root cause: nitrocop only checked direct children of the example group's StatementsNode
/// for examples. RuboCop's `ExampleGroup#examples` uses `find_all_in_scope` which recursively
/// searches the AST for examples, stopping only at scope changes (nested example groups,
/// shared groups, includes) or at other examples. Examples nested inside control flow
/// (if/unless/case), arbitrary method call blocks, or other non-scope-changing constructs
/// were missed entirely.
/// Fix: implemented recursive example collection matching RuboCop's `find_all_in_scope` logic.
///
/// FP root cause 1: block-less example calls (e.g., `it "is pending"` without a block) were
/// treated as examples. RuboCop's `example?` matcher requires a block: `(block (send nil? ...))`.
/// Two block-less calls with similar metadata would produce false duplicate reports.
/// Fix: only consider calls that have a block node as examples.
///
/// FP root cause 2: example calls with explicit receivers (e.g., `object.it { ... }`) were
/// treated as examples. RuboCop requires `nil?` receiver (bare method call).
/// Fix: skip calls that have a receiver.
///
/// **Investigation (2026-03-11):** 354 FPs and 217 FNs remaining.
///
/// FP root cause: The fingerprinter did not distinguish between "no arguments" (metadata=nil
/// in RuboCop) and "has doc string only" (metadata=[] in RuboCop). In Ruby, `nil != []`, so
/// `it { body }` and `it "desc" { body }` are NOT duplicates. The old code treated both as
/// having empty metadata, producing the same fingerprint.
/// Fix: emit different marker bytes for no-args (0xFE) vs has-args (0xFD).
///
/// FN root cause: The fingerprinter only skipped the first argument when it was a string type
/// (StringNode or InterpolatedStringNode). RuboCop's `extract_metadata` pattern `(send _ _ _ $...)`
/// always skips the first argument regardless of type (it's the doc string). When the first arg
/// was a symbol, hash, or other non-string type, nitrocop included it in the fingerprint,
/// making structurally identical examples (differing only in their doc string arg) appear unique.
/// Fix: always skip the first argument for non-`its` calls, regardless of type.
///
/// For `its()` calls, the first argument is an attribute accessor and IS significant. RuboCop
/// appends `definition.arguments` to the signature separately. Updated to match: skip first arg
/// in metadata (like all examples), then append the full arguments list in a separate section.
///
/// **Investigation (2026-03-11, second pass):** 346 FPs and 36 FNs remaining.
///
/// FP root cause 1: Block-pass example calls (e.g., `it(&method(:validate_name))`) were treated
/// as examples. In Prism, `call.block()` returns a `BlockArgumentNode` for `&proc` syntax, not
/// a `BlockNode`. RuboCop's `example?` pattern requires `(block ...)` — a literal block node —
/// so block-pass calls are NOT examples. Multiple block-pass calls with same metadata but
/// different procs produced identical fingerprints (no body section since `as_block_node()` was
/// None), falsely flagging them as duplicates.
/// Fix: require `block.as_block_node()` in both `is_example_node` and `is_scope_change`.
///
/// FP root cause 2: Safe navigation operator (`&.`) was not distinguished from regular dot (`.`)
/// in the AST fingerprinter. The fingerprinter emitted byte 1 for both `call_operator_loc`
/// present cases. RuboCop's Parser gem uses different node types: `send` for `.`/`::` and
/// `csend` for `&.`. So `expect(user.name)` and `expect(user&.name)` have different ASTs in
/// RuboCop but produced the same fingerprint in nitrocop.
/// Fix: emit byte 2 for `&.` and byte 1 for `.`/`::` in `visit_call_node`.
///
/// Remaining FPs/FNs: likely from uncommon patterns not reproducible without corpus examples.
/// `iter_child_nodes` does not handle `WhileNode`, `ForNode`, `RescueNode`, `EnsureNode` etc.,
/// which could cause FNs for examples nested in those constructs (very rare in practice).
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();
        if !is_example_group(name) {
            return;
        }

        // RuboCop checks `#rspec?` which means nil receiver or explicit `RSpec` receiver
        if call.receiver().is_some() {
            // Allow `RSpec.describe` but skip other receivers
            if !is_rspec_receiver(&call) {
                return;
            }
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Collect examples recursively (matching RuboCop's find_all_in_scope)
        let mut examples: Vec<(Vec<u8>, usize, usize)> = Vec::new();
        collect_examples_in_scope(&block_node, source, &mut examples);

        // Group by signature
        let mut body_map: HashMap<Vec<u8>, Vec<(usize, usize)>> = HashMap::new();
        for (sig, line, col) in examples {
            body_map.entry(sig).or_default().push((line, col));
        }

        for locs in body_map.values() {
            if locs.len() > 1 {
                for (idx, &(line, col)) in locs.iter().enumerate() {
                    let other_lines: Vec<String> = locs
                        .iter()
                        .enumerate()
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
    }
}

/// Check if a CallNode has an explicit `RSpec` receiver (e.g., `RSpec.describe`).
fn is_rspec_receiver(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(receiver) = call.receiver() {
        if let Some(c) = receiver.as_constant_read_node() {
            return c.name().as_slice() == b"RSpec";
        }
    }
    false
}

/// Check if a node is a scope change (nested example group, shared group, or include).
/// Matches RuboCop's `ExampleGroup#scope_change?` pattern.
/// In Prism, blocks are children of CallNodes, so we check CallNodes that have a block.
fn is_scope_change(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    // Must have a literal block (scope changes are block-based in RuboCop)
    // Block-pass arguments (e.g., `describe(&proc)`) are NOT scope changes.
    let block = match call.block() {
        Some(b) => b,
        None => return false,
    };
    if block.as_block_node().is_none() {
        return false;
    }

    let name = call.name().as_slice();

    // Example groups and shared groups (with nil or RSpec receiver)
    if (is_example_group(name) || is_rspec_example_group(name) || is_rspec_shared_group(name))
        && (call.receiver().is_none() || is_rspec_receiver(&call))
    {
        return true;
    }

    // Includes: include_examples, it_behaves_like, it_should_behave_like, include_context
    if is_rspec_include(name) && call.receiver().is_none() {
        return true;
    }

    false
}

/// Check if a method name is an RSpec include method.
fn is_rspec_include(name: &[u8]) -> bool {
    matches!(
        name,
        b"include_examples" | b"it_behaves_like" | b"it_should_behave_like" | b"include_context"
    )
}

/// Check if a node is an RSpec example (a call with a block, example method name, nil receiver).
/// Returns the CallNode if it matches.
/// In Prism, `it "x" do ... end` is a CallNode with a block child.
fn is_example_node<'a>(node: &ruby_prism::Node<'a>) -> Option<ruby_prism::CallNode<'a>> {
    let call = node.as_call_node()?;

    // Must have a literal block (RuboCop requires: `(block (send nil? ...) ...)`)
    // Block-pass arguments (e.g., `it(&proc)`) are BlockArgumentNode, not BlockNode,
    // and are NOT considered examples by RuboCop.
    let block = call.block()?;
    block.as_block_node()?;

    // Must have nil receiver (bare method call)
    if call.receiver().is_some() {
        return None;
    }

    let name = call.name().as_slice();
    if is_rspec_example(name) || name == b"its" {
        Some(call)
    } else {
        None
    }
}

/// Recursively collect examples within a block node's scope.
/// Matches RuboCop's `ExampleGroup#find_all_in_scope` which recursively searches
/// for examples, stopping at scope changes (nested example groups, shared groups, includes)
/// and at other examples (doesn't recurse into them).
fn collect_examples_in_scope(
    block_node: &ruby_prism::BlockNode<'_>,
    source: &SourceFile,
    examples: &mut Vec<(Vec<u8>, usize, usize)>,
) {
    let body = match block_node.body() {
        Some(b) => b,
        None => return,
    };

    // RuboCop's find_all_in_scope starts by iterating child nodes of the block
    collect_examples_from_children(&body, source, examples);
}

/// Recursively find examples in child nodes, stopping at scope changes and examples.
fn collect_examples_from_children(
    node: &ruby_prism::Node<'_>,
    source: &SourceFile,
    examples: &mut Vec<(Vec<u8>, usize, usize)>,
) {
    // Iterate over direct children using the node's child nodes
    for child in iter_child_nodes(node) {
        collect_examples_from_node(&child, source, examples);
    }
}

/// Process a single node: if it's an example, collect it; if it's a scope change, stop;
/// otherwise recurse into its children.
fn collect_examples_from_node(
    node: &ruby_prism::Node<'_>,
    source: &SourceFile,
    examples: &mut Vec<(Vec<u8>, usize, usize)>,
) {
    // Is this an example? (call with block, example method, nil receiver)
    if let Some(call) = is_example_node(node) {
        let m = call.name().as_slice();
        if let Some(sig) = example_body_signature(&call, m) {
            // Report at the CallNode location (covers `it "..." do ... end`)
            let loc = call.location();
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            examples.push((sig, line, col));
        }
        return; // Don't recurse into examples
    }

    // Is this a scope change? (nested example group, shared group, include)
    if is_scope_change(node) {
        return; // Don't recurse into scope changes
    }

    // Otherwise, recurse into children
    collect_examples_from_children(node, source, examples);
}

/// Iterate over the child nodes of a node.
/// This is a helper since ruby_prism doesn't expose a generic children iterator.
fn iter_child_nodes<'a>(node: &ruby_prism::Node<'a>) -> Vec<ruby_prism::Node<'a>> {
    // Use statements node's body if available, otherwise use a visitor approach
    if let Some(stmts) = node.as_statements_node() {
        return stmts.body().iter().collect();
    }
    if let Some(block) = node.as_block_node() {
        let mut children = Vec::new();
        if let Some(body) = block.body() {
            children.push(body);
        }
        return children;
    }
    if let Some(if_node) = node.as_if_node() {
        let mut children = Vec::new();
        if let Some(stmts) = if_node.statements() {
            children.push(stmts.as_node());
        }
        if let Some(subsequent) = if_node.subsequent() {
            children.push(subsequent);
        }
        return children;
    }
    if let Some(unless_node) = node.as_unless_node() {
        let mut children = Vec::new();
        if let Some(stmts) = unless_node.statements() {
            children.push(stmts.as_node());
        }
        if let Some(else_clause) = unless_node.else_clause() {
            children.push(else_clause.as_node());
        }
        return children;
    }
    if let Some(else_node) = node.as_else_node() {
        let mut children = Vec::new();
        if let Some(stmts) = else_node.statements() {
            children.push(stmts.as_node());
        }
        return children;
    }
    if let Some(case_node) = node.as_case_node() {
        let mut children: Vec<ruby_prism::Node<'a>> = Vec::new();
        for cond in case_node.conditions().iter() {
            children.push(cond);
        }
        if let Some(else_clause) = case_node.else_clause() {
            children.push(else_clause.as_node());
        }
        return children;
    }
    if let Some(when_node) = node.as_when_node() {
        let mut children = Vec::new();
        if let Some(stmts) = when_node.statements() {
            children.push(stmts.as_node());
        }
        return children;
    }
    if let Some(begin_node) = node.as_begin_node() {
        let mut children = Vec::new();
        if let Some(stmts) = begin_node.statements() {
            children.push(stmts.as_node());
        }
        return children;
    }
    // For CallNode with a block (non-example, non-scope-change), look inside the block
    if let Some(call) = node.as_call_node() {
        if let Some(block) = call.block() {
            if let Some(block_node) = block.as_block_node() {
                let mut children = Vec::new();
                if let Some(body) = block_node.body() {
                    children.push(body);
                }
                return children;
            }
        }
    }
    // For parentheses
    if let Some(paren) = node.as_parentheses_node() {
        let mut children = Vec::new();
        if let Some(body) = paren.body() {
            children.push(body);
        }
        return children;
    }
    // Default: no children to recurse into
    Vec::new()
}

/// Build a structural AST signature from the example's metadata + block body.
///
/// Two examples with the same AST structure (ignoring whitespace/formatting) and
/// same metadata are considered duplicates, matching RuboCop's behavior.
///
/// RuboCop's `build_example_signature` returns `[metadata, implementation]` where:
/// - `metadata` = args after the first string description (tags like `:focus`)
/// - `implementation` = block body AST node
///
/// Both are compared using Ruby's AST structural equality.
///
/// For `its()` calls, the first arg (attribute accessor) is included per RuboCop behavior.
fn example_body_signature(call: &ruby_prism::CallNode<'_>, method_name: &[u8]) -> Option<Vec<u8>> {
    let mut fp = AstFingerprinter::new();

    // Separator between metadata and body sections
    const SECTION_SEP: u8 = 0xFF;
    // Markers to distinguish "no arguments at all" vs "has arguments"
    // RuboCop's extract_metadata returns nil when no first arg exists,
    // vs [] when there's a first arg (doc string) but nothing after it.
    // nil != [] in Ruby, so `it { body }` and `it "desc" { body }` are not duplicates.
    const NO_ARGS_MARKER: u8 = 0xFE;
    const HAS_ARGS_MARKER: u8 = 0xFD;

    // Include metadata args.
    // RuboCop's extract_metadata pattern `(send _ _ _ $...)` always skips the first
    // argument (treating it as a doc string) regardless of its type. For non-`its` calls,
    // the first arg is never part of the comparison signature.
    // For `its()`, the first arg is an attribute accessor, and the full arguments list
    // is appended to the signature separately.
    let is_its = method_name == b"its";
    if let Some(args) = call.arguments() {
        fp.buf.push(HAS_ARGS_MARKER);
        let arg_list: Vec<_> = args.arguments().iter().collect();
        // Always skip first argument — RuboCop's extract_metadata `(send _ _ _ $...)`
        // treats the first arg as a doc string and excludes it from metadata,
        // regardless of its type (string, symbol, hash, etc.)
        for (i, arg) in arg_list.iter().enumerate() {
            if i == 0 {
                continue;
            }
            fp.fingerprint_node(arg);
            fp.buf.push(b',');
        }
    } else {
        fp.buf.push(NO_ARGS_MARKER);
    }

    // For `its()`, append the full arguments list (matching RuboCop's
    // `signature << example.definition.arguments`). This includes the first arg
    // (attribute accessor) which distinguishes `its(:x)` from `its(:y)`.
    if is_its {
        if let Some(args) = call.arguments() {
            fp.buf.push(b'A'); // arguments section marker
            for arg in args.arguments().iter() {
                fp.fingerprint_node(&arg);
                fp.buf.push(b',');
            }
        }
    }

    fp.buf.push(SECTION_SEP);

    // Include block body AST fingerprint
    if let Some(block) = call.block() {
        if let Some(block_node) = block.as_block_node() {
            // Fingerprint the body (StatementsNode), not the entire block
            // (which includes do/end or { } delimiters that differ by formatting)
            if let Some(ref body) = block_node.body() {
                fp.fingerprint_node(body);
            }
        }
    }

    if fp.buf.len() <= 1 {
        // Only the section separator — no meaningful content
        return None;
    }

    Some(fp.buf)
}

/// AST fingerprinter that produces a canonical byte representation of an AST subtree.
///
/// Walks the AST recursively, emitting:
/// - Node type tag (u8) for structural comparison
/// - Literal content for leaf nodes (string values, symbol names, integer literals, etc.)
/// - Child count markers for composite nodes
///
/// This is whitespace-independent: `do\n  expr\nend` and `{ expr }` produce
/// the same fingerprint because they have the same AST structure.
struct AstFingerprinter {
    buf: Vec<u8>,
}

impl AstFingerprinter {
    fn new() -> Self {
        Self {
            buf: Vec::with_capacity(128),
        }
    }

    fn fingerprint_node(&mut self, node: &ruby_prism::Node<'_>) {
        // Emit node type tag
        self.buf.push(crate::cop::node_type::node_type_tag(node));

        // For leaf nodes with literal content, emit the content
        // For composite nodes, the Visit traversal handles children
        self.visit(node);
    }

    fn emit_bytes(&mut self, bytes: &[u8]) {
        // Length-prefixed to avoid ambiguity
        let len = bytes.len() as u32;
        self.buf.extend_from_slice(&len.to_le_bytes());
        self.buf.extend_from_slice(bytes);
    }
}

impl<'pr> Visit<'pr> for AstFingerprinter {
    // For most nodes, the default visit implementation recurses into children,
    // and we emit the node type tag for each child we visit.

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Emit method name for method calls
        self.emit_bytes(node.name().as_slice());
        // Distinguish call operators: none (0), regular dot/:: (1), safe navigation &. (2)
        // RuboCop's Parser gem uses different node types: `send` for `.`/`::` and `csend` for `&.`
        // So `foo.bar` and `foo&.bar` have different ASTs and must produce different fingerprints.
        if let Some(op_loc) = node.call_operator_loc() {
            if op_loc.as_slice() == b"&." {
                self.buf.push(2); // safe navigation (&.)
            } else {
                self.buf.push(1); // regular dot (.) or ::
            }
        } else {
            self.buf.push(0); // no call operator
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        self.emit_bytes(node.unescaped());
        ruby_prism::visit_string_node(self, node);
    }

    fn visit_symbol_node(&mut self, node: &ruby_prism::SymbolNode<'pr>) {
        self.emit_bytes(node.unescaped());
        ruby_prism::visit_symbol_node(self, node);
    }

    fn visit_integer_node(&mut self, node: &ruby_prism::IntegerNode<'pr>) {
        // Use the source representation for integer values
        self.emit_bytes(node.location().as_slice());
        ruby_prism::visit_integer_node(self, node);
    }

    fn visit_float_node(&mut self, node: &ruby_prism::FloatNode<'pr>) {
        self.emit_bytes(node.location().as_slice());
        ruby_prism::visit_float_node(self, node);
    }

    fn visit_constant_read_node(&mut self, node: &ruby_prism::ConstantReadNode<'pr>) {
        self.emit_bytes(node.name().as_slice());
        ruby_prism::visit_constant_read_node(self, node);
    }

    fn visit_constant_path_node(&mut self, node: &ruby_prism::ConstantPathNode<'pr>) {
        if let Some(name) = node.name() {
            self.emit_bytes(name.as_slice());
        }
        ruby_prism::visit_constant_path_node(self, node);
    }

    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        self.emit_bytes(node.name().as_slice());
        ruby_prism::visit_local_variable_read_node(self, node);
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.emit_bytes(node.name().as_slice());
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_instance_variable_read_node(
        &mut self,
        node: &ruby_prism::InstanceVariableReadNode<'pr>,
    ) {
        self.emit_bytes(node.name().as_slice());
        ruby_prism::visit_instance_variable_read_node(self, node);
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        self.emit_bytes(node.name().as_slice());
        ruby_prism::visit_instance_variable_write_node(self, node);
    }

    fn visit_class_variable_read_node(&mut self, node: &ruby_prism::ClassVariableReadNode<'pr>) {
        self.emit_bytes(node.name().as_slice());
        ruby_prism::visit_class_variable_read_node(self, node);
    }

    fn visit_global_variable_read_node(&mut self, node: &ruby_prism::GlobalVariableReadNode<'pr>) {
        self.emit_bytes(node.name().as_slice());
        ruby_prism::visit_global_variable_read_node(self, node);
    }

    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        ruby_prism::visit_interpolated_string_node(self, node);
    }

    fn visit_interpolated_symbol_node(&mut self, node: &ruby_prism::InterpolatedSymbolNode<'pr>) {
        ruby_prism::visit_interpolated_symbol_node(self, node);
    }

    fn visit_regular_expression_node(&mut self, node: &ruby_prism::RegularExpressionNode<'pr>) {
        self.emit_bytes(node.unescaped());
        ruby_prism::visit_regular_expression_node(self, node);
    }

    fn visit_true_node(&mut self, _node: &ruby_prism::TrueNode<'pr>) {
        self.buf.push(1);
    }

    fn visit_false_node(&mut self, _node: &ruby_prism::FalseNode<'pr>) {
        self.buf.push(0);
    }

    fn visit_nil_node(&mut self, _node: &ruby_prism::NilNode<'pr>) {
        self.buf.push(0);
    }

    fn visit_self_node(&mut self, _node: &ruby_prism::SelfNode<'pr>) {
        self.buf.push(0);
    }

    // For block nodes, we only want to fingerprint the body, not the delimiters
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Fingerprint parameters if present
        if let Some(ref params) = node.parameters() {
            self.buf.push(crate::cop::node_type::node_type_tag(params));
            self.visit(params);
        }
        // Fingerprint body if present
        if let Some(ref body) = node.body() {
            self.buf.push(crate::cop::node_type::node_type_tag(body));
            self.visit(body);
        }
    }

    // For nodes we visit by default traversal, we need to emit child type tags
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        for child in node.body().iter() {
            self.buf.push(crate::cop::node_type::node_type_tag(&child));
            self.visit(&child);
        }
    }

    fn visit_arguments_node(&mut self, node: &ruby_prism::ArgumentsNode<'pr>) {
        for child in node.arguments().iter() {
            self.buf.push(crate::cop::node_type::node_type_tag(&child));
            self.visit(&child);
        }
    }

    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        for child in node.elements().iter() {
            self.buf.push(crate::cop::node_type::node_type_tag(&child));
            self.visit(&child);
        }
    }

    fn visit_hash_node(&mut self, node: &ruby_prism::HashNode<'pr>) {
        for child in node.elements().iter() {
            self.buf.push(crate::cop::node_type::node_type_tag(&child));
            self.visit(&child);
        }
    }

    fn visit_keyword_hash_node(&mut self, node: &ruby_prism::KeywordHashNode<'pr>) {
        for child in node.elements().iter() {
            self.buf.push(crate::cop::node_type::node_type_tag(&child));
            self.visit(&child);
        }
    }

    fn visit_assoc_node(&mut self, node: &ruby_prism::AssocNode<'pr>) {
        self.buf
            .push(crate::cop::node_type::node_type_tag(&node.key()));
        self.visit(&node.key());
        self.buf
            .push(crate::cop::node_type::node_type_tag(&node.value()));
        self.visit(&node.value());
    }

    fn visit_parentheses_node(&mut self, node: &ruby_prism::ParenthesesNode<'pr>) {
        // Parentheses are transparent — just visit the body
        if let Some(ref body) = node.body() {
            self.buf.push(crate::cop::node_type::node_type_tag(body));
            self.visit(body);
        }
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.emit_bytes(node.name().as_slice());
        ruby_prism::visit_def_node(self, node);
    }
}

fn is_example_group(name: &[u8]) -> bool {
    // RuboCop only checks ExampleGroups (describe/context/feature),
    // NOT SharedGroups (shared_examples/shared_context).
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
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(RepeatedExample, "cops/rspec/repeated_example");
}
