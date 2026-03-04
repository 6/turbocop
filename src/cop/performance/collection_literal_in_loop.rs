use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashSet;
use std::sync::LazyLock;

/// ## FP/FN history
///
/// A previous attempt to fix FPs (commit 2ffed5a7) was reverted because the
/// source-byte comparison for structural equality was too aggressive — it
/// matched literals that happened to have the same text as a loop receiver
/// in an unrelated ancestor scope, suppressing valid offenses.
///
/// Current fixes applied:
///   1. Safe navigation exclusion: `items&.each { }` does not count as a
///      loop (RuboCop's enumerable_loop? only matches `send`, not `csend`).
///   2. Added regex, rational, and imaginary node types to
///      `is_recursive_basic_literal` to match RuboCop's `recursive_basic_literal?`.
///   3. RuboCop value-equality FP fix: track source bytes of each enclosing
///      loop receiver in a stack. When a literal's source bytes match an
///      enclosing loop receiver, skip the offense. This mirrors RuboCop's
///      `receiver != node` check which uses AST value equality (same content
///      = not inside loop). Only applies to enumerable loops, not keyword
///      loops (while/until/for) or Kernel.loop.
///   4. Added RangeNode and ParenthesesNode to `is_recursive_basic_literal`
///      to match RuboCop's LITERAL_RECURSIVE_TYPES (irange, erange, begin).
///   5. Safe navigation in include? arguments: `a&.parent&.name` is NOT
///      optimized by Ruby 3.4 (only `send` chains, not `csend`), so still
///      flag the offense.
///   6. Loop scope for arguments: RuboCop considers the entire block AST
///      node (including receiver and arguments of the send) as "within"
///      the loop. Previously we only set loop context for the block body,
///      missing literals in arguments (e.g., `[1,2].zip([0].cycle) { }`
///      where `[0]` was not flagged). Fixed by entering loop context
///      before visiting receiver/arguments.
///   7. Descendant exclusion: RuboCop's `!receiver.descendants.include?(node)`
///      excludes literals that are part of the loop receiver expression
///      (e.g., `[1,2].sort.each { }` — `[1,2]` is a descendant of receiver
///      `[1,2].sort`). Implemented via byte-range containment check.
pub struct CollectionLiteralInLoop;

const ENUMERABLE_METHODS: &[&[u8]] = &[
    b"all?",
    b"any?",
    b"chain",
    b"chunk",
    b"chunk_while",
    b"collect",
    b"collect_concat",
    b"compact",
    b"count",
    b"cycle",
    b"detect",
    b"drop",
    b"drop_while",
    b"each",
    b"each_cons",
    b"each_entry",
    b"each_slice",
    b"each_with_index",
    b"each_with_object",
    b"entries",
    b"filter",
    b"filter_map",
    b"find",
    b"find_all",
    b"find_index",
    b"first",
    b"flat_map",
    b"grep",
    b"grep_v",
    b"group_by",
    b"include?",
    b"inject",
    b"lazy",
    b"map",
    b"max",
    b"max_by",
    b"member?",
    b"min",
    b"min_by",
    b"minmax",
    b"minmax_by",
    b"none?",
    b"one?",
    b"partition",
    b"reduce",
    b"reject",
    b"reverse_each",
    b"select",
    b"slice_after",
    b"slice_before",
    b"slice_when",
    b"sort",
    b"sort_by",
    b"sum",
    b"take",
    b"take_while",
    b"tally",
    b"to_a",
    b"to_h",
    b"to_set",
    b"uniq",
    b"zip",
];

/// Non-mutating Array methods (safe to call on a literal without modifying it)
const NONMUTATING_ARRAY_METHODS: &[&[u8]] = &[
    b"&",
    b"*",
    b"+",
    b"-",
    b"<=>",
    b"==",
    b"[]",
    b"all?",
    b"any?",
    b"assoc",
    b"at",
    b"bsearch",
    b"bsearch_index",
    b"collect",
    b"combination",
    b"compact",
    b"count",
    b"cycle",
    b"deconstruct",
    b"difference",
    b"dig",
    b"drop",
    b"drop_while",
    b"each",
    b"each_index",
    b"empty?",
    b"eql?",
    b"fetch",
    b"filter",
    b"find_index",
    b"first",
    b"flatten",
    b"hash",
    b"include?",
    b"index",
    b"inspect",
    b"intersection",
    b"join",
    b"last",
    b"length",
    b"map",
    b"max",
    b"min",
    b"minmax",
    b"none?",
    b"one?",
    b"pack",
    b"permutation",
    b"product",
    b"rassoc",
    b"reject",
    b"repeated_combination",
    b"repeated_permutation",
    b"reverse",
    b"reverse_each",
    b"rindex",
    b"rotate",
    b"sample",
    b"select",
    b"shuffle",
    b"size",
    b"slice",
    b"sort",
    b"sum",
    b"take",
    b"take_while",
    b"to_a",
    b"to_ary",
    b"to_h",
    b"to_s",
    b"transpose",
    b"union",
    b"uniq",
    b"values_at",
    b"zip",
    b"|",
];

/// Non-mutating Hash methods
const NONMUTATING_HASH_METHODS: &[&[u8]] = &[
    b"<",
    b"<=",
    b"==",
    b">",
    b">=",
    b"[]",
    b"any?",
    b"assoc",
    b"compact",
    b"dig",
    b"each",
    b"each_key",
    b"each_pair",
    b"each_value",
    b"empty?",
    b"eql?",
    b"fetch",
    b"fetch_values",
    b"filter",
    b"flatten",
    b"has_key?",
    b"has_value?",
    b"hash",
    b"include?",
    b"inspect",
    b"invert",
    b"key",
    b"key?",
    b"keys?",
    b"length",
    b"member?",
    b"merge",
    b"rassoc",
    b"rehash",
    b"reject",
    b"select",
    b"size",
    b"slice",
    b"to_a",
    b"to_h",
    b"to_hash",
    b"to_proc",
    b"to_s",
    b"transform_keys",
    b"transform_values",
    b"value?",
    b"values",
    b"values_at",
];

fn build_method_set(methods: &[&[u8]]) -> HashSet<Vec<u8>> {
    methods.iter().map(|m| m.to_vec()).collect()
}

/// Pre-compiled method sets — built once, reused across all files.
static ARRAY_METHOD_SET: LazyLock<HashSet<Vec<u8>>> = LazyLock::new(|| {
    let mut set = build_method_set(ENUMERABLE_METHODS);
    for m in NONMUTATING_ARRAY_METHODS {
        set.insert(m.to_vec());
    }
    set
});

static HASH_METHOD_SET: LazyLock<HashSet<Vec<u8>>> = LazyLock::new(|| {
    let mut set = build_method_set(ENUMERABLE_METHODS);
    for m in NONMUTATING_HASH_METHODS {
        set.insert(m.to_vec());
    }
    set
});

static ENUMERABLE_METHOD_SET: LazyLock<HashSet<Vec<u8>>> =
    LazyLock::new(|| build_method_set(ENUMERABLE_METHODS));

impl Cop for CollectionLiteralInLoop {
    fn name(&self) -> &'static str {
        "Performance/CollectionLiteralInLoop"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let min_size = config.get_usize("MinSize", 1);
        let target_ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(2.7);

        let mut visitor = CollectionLiteralVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            loop_depth: 0,
            loop_receiver_sources: Vec::new(),
            min_size,
            target_ruby_version,
            array_methods: &ARRAY_METHOD_SET,
            hash_methods: &HASH_METHOD_SET,
            enumerable_methods: &ENUMERABLE_METHOD_SET,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct CollectionLiteralVisitor<'a, 'src> {
    cop: &'a CollectionLiteralInLoop,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    loop_depth: usize,
    /// Source byte ranges of receivers of enclosing enumerable loop calls.
    /// Used to implement RuboCop's value-equality exclusion: if a literal's
    /// source bytes match an enclosing loop receiver, it is NOT considered
    /// "inside" that loop (it's the same value used to drive the loop).
    loop_receiver_sources: Vec<(usize, usize)>,
    min_size: usize,
    target_ruby_version: f64,
    array_methods: &'a HashSet<Vec<u8>>,
    hash_methods: &'a HashSet<Vec<u8>>,
    enumerable_methods: &'a HashSet<Vec<u8>>,
}

impl<'pr> Visit<'pr> for CollectionLiteralVisitor<'_, '_> {
    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        self.loop_depth += 1;
        ruby_prism::visit_while_node(self, node);
        self.loop_depth -= 1;
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        self.loop_depth += 1;
        ruby_prism::visit_until_node(self, node);
        self.loop_depth -= 1;
    }

    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'pr>) {
        self.loop_depth += 1;
        ruby_prism::visit_for_node(self, node);
        self.loop_depth -= 1;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        // Check if this call has a block and is a loop-like method
        let is_loop_call = if let Some(block) = node.block() {
            if block.as_block_node().is_some() {
                self.is_loop_method(node)
            } else {
                false
            }
        } else {
            false
        };

        // Check if this call's receiver is a collection literal inside a loop
        if self.loop_depth > 0 {
            self.check_call(node, method_name);
        }

        // When this call is a loop (has block + enumerable method), RuboCop
        // considers the entire block AST node — including receiver and
        // arguments — as being "within" the loop. So we enter loop context
        // before visiting receiver/arguments, not just the block body.
        if is_loop_call {
            self.loop_depth += 1;
            // Track the receiver's source bytes for value-equality exclusion.
            // RuboCop's `node_within_enumerable_loop?` checks
            // `receiver != node` using AST value equality — if the literal
            // has the same source text as the loop receiver, skip it.
            // Also excludes descendants of the receiver.
            if let Some(recv) = node.receiver() {
                let loc = recv.location();
                self.loop_receiver_sources
                    .push((loc.start_offset(), loc.end_offset()));
            }
        }

        // Visit receiver
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        // Visit arguments
        if let Some(args) = node.arguments() {
            self.visit(&args.as_node());
        }

        // Visit block body (already in loop context if is_loop_call)
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                // Visit block parameters
                if let Some(params) = block_node.parameters() {
                    self.visit(&params);
                }
                // Visit block body
                if let Some(body) = block_node.body() {
                    self.visit(&body);
                }
            } else {
                self.visit(&block);
            }
        }

        if is_loop_call {
            self.loop_depth -= 1;
            if node.receiver().is_some() {
                self.loop_receiver_sources.pop();
            }
        }
    }
}

impl CollectionLiteralVisitor<'_, '_> {
    /// Check if a call node is a loop-like method (Kernel.loop or enumerable method).
    /// RuboCop's `enumerable_loop?` pattern only matches `send`, not `csend` (safe
    /// navigation `&.`), so `items&.each { }` is NOT treated as a loop.
    fn is_loop_method(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        let method_name = call.name().as_slice();

        // Check for Kernel.loop or bare `loop`
        // Handle both simple constant (Kernel) and qualified constant (::Kernel)
        if method_name == b"loop" {
            match call.receiver() {
                None => return true,
                Some(recv) => {
                    if let Some(cr) = recv.as_constant_read_node() {
                        if cr.name().as_slice() == b"Kernel" {
                            return true;
                        }
                    }
                    if let Some(cp) = recv.as_constant_path_node() {
                        if let Some(cp_name) = cp.name() {
                            if cp_name.as_slice() == b"Kernel" {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        // Safe navigation (&.) calls are NOT loops — RuboCop's enumerable_loop?
        // pattern only matches `send`, not `csend`.
        if let Some(op) = call.call_operator_loc() {
            if op.as_slice() == b"&." {
                return false;
            }
        }

        // Enumerable methods
        self.enumerable_methods.contains(method_name)
    }

    /// Check if a literal node matches or is a descendant of any enclosing
    /// loop receiver. This implements RuboCop's two exclusion checks:
    ///
    ///   1. `receiver != node` — value equality (same source text)
    ///   2. `!receiver.descendants.include?(node)` — node is part of receiver
    ///
    /// Both are approximated via byte-range checks: exact match OR containment.
    fn matches_enclosing_loop_receiver(&self, node_start: usize, node_end: usize) -> bool {
        let node_bytes = &self.source.as_bytes()[node_start..node_end];
        for &(recv_start, recv_end) in &self.loop_receiver_sources {
            // Exact match (value equality: receiver == node)
            let recv_bytes = &self.source.as_bytes()[recv_start..recv_end];
            if node_bytes == recv_bytes {
                return true;
            }
            // Containment (node is a descendant of receiver)
            if node_start >= recv_start && node_end <= recv_end {
                return true;
            }
        }
        false
    }

    fn check_call(&mut self, call: &ruby_prism::CallNode<'_>, method_name: &[u8]) {
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Check if receiver is an Array literal with a non-mutating array method
        if let Some(array) = recv.as_array_node() {
            if !self.array_methods.contains(method_name) {
                return;
            }
            if array.elements().len() < self.min_size {
                return;
            }
            if !is_recursive_basic_literal(&recv) {
                return;
            }
            // Ruby 3.4+ optimizes Array#include? with simple arguments at the VM level,
            // so no allocation occurs and no offense should be registered.
            if self.target_ruby_version >= 3.4
                && method_name == b"include?"
                && is_optimized_include_arg(call)
            {
                return;
            }
            let loc = recv.location();
            // RuboCop value-equality exclusion: if this literal's source matches
            // an enclosing loop receiver, skip it.
            if self.matches_enclosing_loop_receiver(loc.start_offset(), loc.end_offset()) {
                return;
            }
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.".to_string(),
            ));
            return;
        }

        // Check if receiver is a Hash literal with a non-mutating hash method
        if let Some(hash) = recv.as_hash_node() {
            if !self.hash_methods.contains(method_name) {
                return;
            }
            if hash.elements().len() < self.min_size {
                return;
            }
            if !is_recursive_basic_literal(&recv) {
                return;
            }
            let loc = recv.location();
            if self.matches_enclosing_loop_receiver(loc.start_offset(), loc.end_offset()) {
                return;
            }
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Avoid immutable Hash literals in loops. It is better to extract it into a local variable or a constant.".to_string(),
            ));
        }
    }
}

/// Check if a node is a recursive basic literal (all children are basic literals too).
/// Matches RuboCop's `recursive_basic_literal?` which includes: int, float, str, sym,
/// nil, true, false, complex (ImaginaryNode), rational (RationalNode),
/// regexp (non-interpolated RegularExpressionNode), ranges (irange/erange),
/// and parenthesized expressions (begin).
///
/// RuboCop also recurses through `LITERAL_RECURSIVE_METHODS` (==, *, <, etc.)
/// so that expressions like `"str" * 100` are considered recursive basic literals.
fn is_recursive_basic_literal(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_regular_expression_node().is_some()
    {
        return true;
    }

    if let Some(array) = node.as_array_node() {
        return array
            .elements()
            .iter()
            .all(|e| is_recursive_basic_literal(&e));
    }

    if let Some(hash) = node.as_hash_node() {
        return hash.elements().iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_recursive_basic_literal(&assoc.key())
                    && is_recursive_basic_literal(&assoc.value())
            } else {
                false
            }
        });
    }

    // KeywordHashNode (keyword args like `foo(a: 1)`) cannot appear as a
    // method receiver, so this branch is unreachable in practice, but we
    // handle as_keyword_hash_node to satisfy the prism pitfalls check.
    if let Some(kh) = node.as_keyword_hash_node() {
        return kh.elements().iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_recursive_basic_literal(&assoc.key())
                    && is_recursive_basic_literal(&assoc.value())
            } else {
                false
            }
        });
    }

    // Range literals (1..10, 1...10) — RuboCop's LITERAL_RECURSIVE_TYPES
    // includes :irange and :erange.
    if let Some(range) = node.as_range_node() {
        let left_ok = range
            .left()
            .map(|l| is_recursive_basic_literal(&l))
            .unwrap_or(true);
        let right_ok = range
            .right()
            .map(|r| is_recursive_basic_literal(&r))
            .unwrap_or(true);
        return left_ok && right_ok;
    }

    // Parenthesized expressions like `(1..32)` — RuboCop's :begin type.
    // In Prism this is a ParenthesesNode wrapping the inner expression.
    if let Some(parens) = node.as_parentheses_node() {
        return parens
            .body()
            .map(|body| {
                // The body is typically a StatementsNode with one child
                if let Some(stmts) = body.as_statements_node() {
                    stmts.body().iter().all(|s| is_recursive_basic_literal(&s))
                } else {
                    is_recursive_basic_literal(&body)
                }
            })
            .unwrap_or(true);
    }

    // Method calls with literal recursive methods (==, *, <, etc.)
    // e.g. `"str" * 100` is considered a recursive basic literal in RuboCop.
    if let Some(call) = node.as_call_node() {
        let method = call.name().as_slice();
        if is_literal_recursive_method(method) {
            let recv_ok = call
                .receiver()
                .map(|r| is_recursive_basic_literal(&r))
                .unwrap_or(true);
            let args_ok = call
                .arguments()
                .map(|args| {
                    args.arguments()
                        .iter()
                        .all(|a| is_recursive_basic_literal(&a))
                })
                .unwrap_or(true);
            return recv_ok && args_ok;
        }
    }

    false
}

/// Methods that RuboCop treats as recursive literal boundaries.
/// Matches RuboCop's `LITERAL_RECURSIVE_METHODS`.
fn is_literal_recursive_method(method: &[u8]) -> bool {
    matches!(
        method,
        b"==" | b"===" | b"!=" | b"<=" | b">=" | b">" | b"<" | b"*" | b"!" | b"<=>"
    )
}

/// Check if a call to `include?` on an array literal has a single "simple" argument
/// that Ruby 3.4+ optimizes (no allocation). Simple arguments are: string literals,
/// `self`, local variables, instance variables, and method call chains without arguments.
fn is_optimized_include_arg(call: &ruby_prism::CallNode<'_>) -> bool {
    let args = match call.arguments() {
        Some(a) => a,
        None => return false,
    };
    let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
    if arg_list.len() != 1 {
        return false;
    }
    is_simple_argument(&arg_list[0])
}

/// Check if a node is a "simple" argument for the Ruby 3.4+ include? optimization.
/// Matches: string literals, `self`, local variables, instance variables, and
/// method call chains where no call in the chain has arguments AND uses regular
/// dispatch (not safe navigation `&.`). RuboCop's implementation checks
/// `arg.send_type?` which only matches `send`, not `csend`.
fn is_simple_argument(node: &ruby_prism::Node<'_>) -> bool {
    // String literal
    if node.as_string_node().is_some() {
        return true;
    }
    // self
    if node.as_self_node().is_some() {
        return true;
    }
    // Local variable read
    if node.as_local_variable_read_node().is_some() {
        return true;
    }
    // Instance variable read
    if node.as_instance_variable_read_node().is_some() {
        return true;
    }
    // Ruby 3.4+ 'it' implicit block parameter
    if node.as_it_local_variable_read_node().is_some() {
        return true;
    }
    // Method call (possibly chained) with no arguments at any level.
    // Safe navigation (&.) calls are NOT optimized — RuboCop checks
    // `arg.send_type?` which only matches `send`, not `csend`.
    if let Some(call) = node.as_call_node() {
        // Safe navigation breaks the optimization
        if let Some(op) = call.call_operator_loc() {
            if op.as_slice() == b"&." {
                return false;
            }
        }
        // Disallow if this call has arguments
        if call.arguments().is_some() {
            return false;
        }
        // If there's a receiver, it must also be simple
        match call.receiver() {
            Some(recv) => return is_simple_argument(&recv),
            None => return true, // bare method call like `method_call`
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        CollectionLiteralInLoop,
        "cops/performance/collection_literal_in_loop"
    );

    fn ruby34_config() -> CopConfig {
        let mut config = CopConfig::default();
        config.options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::Value::Number(3.4.into()),
        );
        config
    }

    #[test]
    fn ruby34_skips_include_with_local_variable() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &CollectionLiteralInLoop,
            b"items.each do |item|\n  next if %w[foo bar baz].include?(item)\nend\n",
            ruby34_config(),
        );
    }

    #[test]
    fn ruby34_skips_include_with_method_chain() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &CollectionLiteralInLoop,
            b"items.each do |item|\n  next if [1, 2, 3].include?(item.name)\nend\n",
            ruby34_config(),
        );
    }

    #[test]
    fn ruby34_skips_include_with_double_method_chain() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &CollectionLiteralInLoop,
            b"items.each do |item|\n  next if [1, 2, 3].include?(item.name.downcase)\nend\n",
            ruby34_config(),
        );
    }

    #[test]
    fn ruby34_skips_include_with_self() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &CollectionLiteralInLoop,
            b"items.each do |item|\n  next if %w[a b c].include?(self)\nend\n",
            ruby34_config(),
        );
    }

    #[test]
    fn ruby34_skips_include_with_instance_variable() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &CollectionLiteralInLoop,
            b"items.each do |item|\n  next if [1, 2, 3].include?(@ivar)\nend\n",
            ruby34_config(),
        );
    }

    #[test]
    fn ruby34_skips_include_with_string_literal() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &CollectionLiteralInLoop,
            b"items.each do |item|\n  next if [1, 2, 3].include?(\"str\")\nend\n",
            ruby34_config(),
        );
    }

    #[test]
    fn ruby34_skips_include_with_bare_method_call() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &CollectionLiteralInLoop,
            b"items.each do |item|\n  next if [1, 2, 3].include?(method_call)\nend\n",
            ruby34_config(),
        );
    }

    #[test]
    fn ruby34_still_flags_include_with_method_call_with_args() {
        // include?(foo.call(true)) is NOT optimized — still an offense
        crate::testutil::assert_cop_offenses_full_with_config(
            &CollectionLiteralInLoop,
            b"items.each do |item|\n  [1, 2, 3].include?(item.call(true))\n  ^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\nend\n",
            ruby34_config(),
        );
    }

    #[test]
    fn ruby34_still_flags_hash_include() {
        // Hash#include? is NOT optimized in Ruby 3.4 — only Array#include? is
        crate::testutil::assert_cop_offenses_full_with_config(
            &CollectionLiteralInLoop,
            b"items.each do |item|\n  { foo: :bar }.include?(:foo)\n  ^^^^^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Hash literals in loops. It is better to extract it into a local variable or a constant.\nend\n",
            ruby34_config(),
        );
    }

    #[test]
    fn ruby34_still_flags_array_index_method() {
        // Other array methods like `index` are NOT optimized — still an offense
        crate::testutil::assert_cop_offenses_full_with_config(
            &CollectionLiteralInLoop,
            b"items.each do |item|\n  [1, 2, 3].index(item)\n  ^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\nend\n",
            ruby34_config(),
        );
    }

    #[test]
    fn ruby34_skips_include_with_it_implicit_param() {
        // Ruby 3.4+ 'it' implicit block parameter is parsed as ItLocalVariableReadNode
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &CollectionLiteralInLoop,
            b"items.each { [1, 2, 3].include?(it) }\n",
            ruby34_config(),
        );
    }

    #[test]
    fn detects_inside_no_receiver_each() {
        // Bare `each` (no receiver) should be treated as a loop
        crate::testutil::assert_cop_offenses_full(
            &CollectionLiteralInLoop,
            b"each do |e|\n  [1, 2, 3].include?(e)\n  ^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\nend\n",
        );
    }

    #[test]
    fn detects_inside_select() {
        crate::testutil::assert_cop_offenses_full(
            &CollectionLiteralInLoop,
            b"items.select do |item|\n  [1, 2, 3].include?(item)\n  ^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\nend\n",
        );
    }

    #[test]
    fn detects_inside_map_brace_block() {
        crate::testutil::assert_cop_offenses_full(
            &CollectionLiteralInLoop,
            b"items.map { |item| [1, 2, 3].include?(item) }\n                   ^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\n",
        );
    }

    #[test]
    fn detects_post_while_loop() {
        crate::testutil::assert_cop_offenses_full(
            &CollectionLiteralInLoop,
            b"begin\n  [1, 2, 3].include?(e)\n  ^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\nend while condition\n",
        );
    }

    #[test]
    fn detects_post_until_loop() {
        crate::testutil::assert_cop_offenses_full(
            &CollectionLiteralInLoop,
            b"begin\n  [1, 2, 3].include?(e)\n  ^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\nend until condition\n",
        );
    }

    #[test]
    fn detects_literal_receiver_of_enumerable_inside_loop() {
        // [1, 2, 3].map { } inside an each loop should be flagged:
        // the literal array is allocated on every iteration of the outer loop
        crate::testutil::assert_cop_offenses_full(
            &CollectionLiteralInLoop,
            b"items.each do |item|\n  [1, 2, 3].map { |x| x + 1 }\n  ^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\nend\n",
        );
    }

    #[test]
    fn detects_percent_i_literal() {
        crate::testutil::assert_cop_offenses_full(
            &CollectionLiteralInLoop,
            b"items.each do |item|\n  %i[foo bar baz].include?(item)\n  ^^^^^^^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\nend\n",
        );
    }

    #[test]
    fn detects_percent_w_literal() {
        crate::testutil::assert_cop_offenses_full(
            &CollectionLiteralInLoop,
            b"items.each do |item|\n  %w[foo bar baz].include?(item)\n  ^^^^^^^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\nend\n",
        );
    }

    #[test]
    fn detects_nested_block_in_loop() {
        // Collection literal inside a non-loop block inside a loop should still be flagged
        crate::testutil::assert_cop_offenses_full(
            &CollectionLiteralInLoop,
            b"items.each do |item|\n  something do\n    [1, 2, 3].include?(item)\n    ^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\n  end\nend\n",
        );
    }

    #[test]
    fn ruby33_still_flags_include_with_simple_arg() {
        // Ruby < 3.4 does NOT optimize include?, so still an offense
        let mut config = CopConfig::default();
        config.options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::Value::Number(3.3.into()),
        );
        crate::testutil::assert_cop_offenses_full_with_config(
            &CollectionLiteralInLoop,
            b"items.each do |item|\n  [1, 2, 3].include?(item)\n  ^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\nend\n",
            config,
        );
    }

    #[test]
    fn detects_regex_array_in_loop() {
        // Array of regex literals should be detected (regex is a basic literal in RuboCop)
        crate::testutil::assert_cop_offenses_full(
            &CollectionLiteralInLoop,
            b"items.each do |str|\n  [/foo/, /bar/].any? { |r| str.match?(r) }\n  ^^^^^^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\nend\n",
        );
    }

    #[test]
    fn no_offense_safe_navigation_loop() {
        // Safe navigation (&.) should NOT be treated as a loop
        crate::testutil::assert_cop_no_offenses_full(
            &CollectionLiteralInLoop,
            b"items&.each { |item| [1, 2, 3].include?(item) }\n",
        );
    }

    #[test]
    fn ruby34_still_flags_include_with_safe_nav_chain() {
        // Safe navigation (&.) in the argument is NOT optimized by Ruby 3.4.
        // RuboCop checks `arg.send_type?` which only matches `send`, not `csend`.
        crate::testutil::assert_cop_offenses_full_with_config(
            &CollectionLiteralInLoop,
            b"items.each do |a|\n  %w[video audio].include?(a&.parent&.name)\n  ^^^^^^^^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\nend\n",
            ruby34_config(),
        );
    }

    #[test]
    fn no_offense_same_literal_as_loop_receiver() {
        // When a literal inside a loop has the same source text as the loop receiver,
        // RuboCop uses value equality to exclude it (receiver != node returns false).
        crate::testutil::assert_cop_no_offenses_full(
            &CollectionLiteralInLoop,
            b"[1].each { |x| [1].each { puts x } }\n",
        );
    }

    #[test]
    fn offense_different_literal_from_loop_receiver() {
        // Different literal from the loop receiver should still be flagged
        crate::testutil::assert_cop_offenses_full(
            &CollectionLiteralInLoop,
            b"[1].each { |x| [2].each { puts x } }\n               ^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\n",
        );
    }

    #[test]
    fn detects_array_with_range_in_loop() {
        // Arrays containing ranges should be detected (ranges are basic literals)
        crate::testutil::assert_cop_offenses_full(
            &CollectionLiteralInLoop,
            b"items.each do |item|\n  [1..10, 20..30].include?(item)\n  ^^^^^^^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\nend\n",
        );
    }

    #[test]
    fn detects_literal_as_loop_receiver_inside_outer_loop() {
        // When a literal array is the receiver of .each inside an outer loop,
        // it gets re-allocated on each iteration of the outer loop.
        crate::testutil::assert_cop_offenses_full(
            &CollectionLiteralInLoop,
            b"records.each do |record|\n  ['en', 'pt', 'fr'].each do |locale|\n  ^^^^^^^^^^^^^^^^^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\n    puts locale\n  end\nend\n",
        );
    }

    #[test]
    fn detects_literal_arg_in_zip_with_block() {
        // [0].cycle inside a .zip() call that has a block — the block makes
        // zip a loop-like call, and [0] is a literal argument inside it.
        crate::testutil::assert_cop_offenses_full(
            &CollectionLiteralInLoop,
            b"[1,2].zip([0].cycle){|a| arr << a}\n          ^^^ Performance/CollectionLiteralInLoop: Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.\n",
        );
    }
}
