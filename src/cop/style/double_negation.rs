use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/DoubleNegation: Avoid the use of double negation (`!!`).
///
/// Corpus investigation (round 3): 70 FPs, 40 FNs.
///
/// Root cause of FPs: nitrocop used byte-range matching for return positions and
/// unconditionally excluded `!!` inside hash/array/keyword_hash nodes. RuboCop uses
/// a much looser line-based check: if `!!` is on or after the first line of the def
/// body's last statement, it's allowed — regardless of whether it's inside a hash
/// value, method argument, XOR expression, etc. RuboCop only excludes hash/array
/// when the last_child of the def body itself is a pair/hash node or the parent is
/// an array type (i.e., the method returns a literal hash or array).
///
/// Root cause of FNs: nitrocop recursively marked all branch endpoints in nested
/// conditionals as return positions. RuboCop uses a stricter check for nested
/// conditionals: it finds the innermost conditional ancestor and checks if that
/// conditional's last line >= the def body's last child's last line. Additionally,
/// when the `!!` node's parent is a statement sequence (begin_type?), RuboCop checks
/// that `!!` is on the last line of that sequence — otherwise it's not a return value
/// even if it's inside a return-position conditional.
///
/// Fix (round 3): Replaced byte-range approach with line-based checks matching
/// RuboCop's `end_of_method_definition?` / `double_negative_condition_return_value?`
/// logic. Tracks def body info (last child first/last line, hash/array type) and
/// conditional ancestor last lines on stacks.
///
/// Corpus investigation (round 4): 28 FPs, 25 FNs.
///
/// FP root cause: The `stmts_last_line` check (for `begin_type?` parent) was applied
/// unconditionally. In RuboCop, `find_parent_not_enumerable` walks up from the `!!`
/// node skipping pair/hash/array; if the non-enumerable parent is NOT `begin_type?`
/// (e.g., it's a send/if/assignment), the line check is skipped. Additionally, Prism
/// always wraps branch bodies in StatementsNode even for single-statement branches,
/// while Parser AST only creates `begin` wrappers for multi-statement bodies. This
/// caused `!!` inside hash values, method call args, and assignments within
/// conditional branches to be incorrectly flagged.
///
/// FN root cause: For single-statement method bodies, RuboCop calls
/// `node.child_nodes.last` on the expression itself (not just the statements
/// wrapper), which digs into the expression's last child. For a method call, this
/// reaches the keyword hash args. nitrocop wasn't doing this dig-in, so `!!` inside
/// hash args of a single-statement method call was treated as return position.
///
/// Fix (round 4): (1) Track `parent_is_statements` flag — only true when the
/// StatementsNode has >1 statement (matching Parser's `begin` wrapper behavior).
/// Reset to false when entering CallNode children. Only apply the `stmts_last_line`
/// check when true. (2) Added `parser_last_child()` to dig into single-statement
/// method bodies (CallNode → last arg), matching RuboCop's `child_nodes.last`.
///
/// Corpus investigation (round 5): 20 FPs, 17 FNs.
///
/// FP root cause: (1) `parent_is_statements` leaked through assignment nodes.
/// When `!!` was inside an assignment (e.g., `@reversed = !!expr`) within a
/// multi-statement conditional branch, the flag remained true from the enclosing
/// StatementsNode, causing the stmts_last_line check to fire and incorrectly
/// flag the `!!`. RuboCop's `find_parent_not_enumerable` stops at the assignment
/// node (not begin_type?), skipping the strict line check. (2) `parser_last_child`
/// returned None for block calls (CallNode with block), causing the entire block
/// call node to be used as last_child. This made `last_child_last_line` too large,
/// so `last_child_last_line <= cond_last_line` failed for conditionals inside
/// blocks (e.g., `catch_exceptions do if ... !!result ... end end`).
///
/// FN root cause: (1) `parser_last_child` didn't handle OrNode/AndNode, so
/// single-statement methods like `def foo?; !!x && y; end` used the entire
/// and/or expression as last_child instead of the right-hand side. This made
/// `last_child_first_line` too early, allowing `!!` on earlier lines to be
/// incorrectly treated as return position. (2) `parser_last_child` didn't handle
/// `*OrWriteNode`/`*AndWriteNode` (e.g., `@x ||= { ... }`), so the hash value
/// wasn't detected as last_child, missing the hash_or_pair offense path.
///
/// Fix (round 5): (1) Changed `visit_statements_node` to iterate children
/// manually, only setting `parent_is_statements = true` when the direct child
/// IS a CallNode. Assignment nodes and other non-call children get false,
/// preventing the flag from leaking into their subtrees. (2) Updated
/// `parser_last_child` to dig into block bodies (returning the last child of
/// the block's StatementsNode), matching Parser's `child_nodes.last` for blocks.
/// (3) Added OrNode/AndNode handling in `parser_last_child` (returns right side).
/// (4) Added `*OrWriteNode`, `*AndWriteNode`, and other compound assignment
/// handlers to `parser_last_child`.
///
/// Corpus investigation (round 6): 15 FPs, 4 FNs.
///
/// FP root cause: (1) `parser_last_child` for CallNode with block dug too deep,
/// returning the last child of the block body instead of the block body itself.
/// In Parser AST, `child_nodes.last` of a block returns the body (begin wrapper
/// or single expression), NOT the last statement within it. This caused `!!`
/// inside non-define_method blocks (synchronize, alter, map, filter_map) to have
/// a `last_child_first_line` set to a line AFTER the `!!`, incorrectly preventing
/// it from being classified as return position. (2) Three fixture tests were
/// incorrect: the nested-conditional and map-block-hash patterns in single-statement
/// bodies are return position in RuboCop (verified empirically).
///
/// FN root cause: Prism includes the shared `end` keyword in elsif IfNode ranges,
/// while Parser AST excludes it. For `if A; ...; elsif B; !!x; end` in a
/// multi-statement body, `cond_last_line` (from the elsif IfNode) equaled
/// `last_child_last_line` (from the outer IfNode), both including `end`. In
/// Parser, the elsif's range stops at its body's last line (before `end`), so
/// the comparison `last_child_last_line <= cond_last_line` correctly fails.
///
/// Fix (round 6): (1) Changed `parser_last_child` for CallNode with block to
/// return the block body (StatementsNode) directly instead of its last child.
/// (2) Added IfNode handling to `parser_last_child` — returns the subsequent
/// (elsif IfNode or else body) matching Parser's `child_nodes.last` filtering.
/// (3) Added `parser_if_last_line()` to compute Parser-equivalent last line for
/// IfNode, excluding the shared `end` keyword. Used in `visit_if_node` for elsif
/// branches and in `find_last_child_of_stmts` for elsif last-child nodes.
/// (4) Fixed three incorrect fixture tests (moved to no_offense.rb).
/// (5) Added FN test cases for elsif branches in multi-statement bodies.
///
/// Corpus investigation (round 7): 6 FPs, 0 FNs.
///
/// FP root cause: Parenthesized `!!` expressions like `(!!expr) ? a : b` and
/// `(!!data[:key]) == value` were flagged inside def bodies with conditional
/// ancestors. In Parser AST, `(!!expr)` creates a `begin` node, and RuboCop's
/// `find_parent_not_enumerable` returns it. Since `begin_type?` is true, the
/// lenient check `node.loc.line == parent.loc.last_line` applies, which
/// trivially passes for single-line parenthesized expressions. In Prism,
/// parenthesized expressions create `ParenthesesNode`, which nitrocop did not
/// recognize as equivalent to `begin_type?`.
///
/// Fix (round 7): Added `parenthesized_last_line` tracking to the visitor.
/// When visiting a `ParenthesesNode` inside a def body with a conditional
/// ancestor, the parens' last line is recorded. In `is_end_of_method_definition`,
/// when inside a conditional and `parenthesized_last_line` is set, the lenient
/// same-line check is used instead of the strict `last_child_last_line <=
/// cond_last_line` check.
///
/// Corpus investigation (round 8): 3 FPs, 2 FNs.
///
/// FP root cause: `with_def_body` cleared `conditional_last_line_stack`, but
/// RuboCop's `find_conditional_node_from_ascendant` walks past def boundaries.
/// When a def is wrapped in an outer conditional (e.g., `if has_ssl; class X;
/// def foo; ... end; end; end`), RuboCop sees the outer `if` as a conditional
/// ancestor of the `!!` inside the def. Since the outer if's last_line is past
/// the def body, `last_child.last_line <= cond_last_line` trivially passes,
/// making the `!!` allowed. Nitrocop was clearing the conditional stack on def
/// entry, preventing the outer conditional from being visible.
/// Additionally, `visit_if_node` and other conditional visitors only pushed to
/// the stack when `def_info_stack` was non-empty, so outer conditionals wrapping
/// a def were never tracked.
///
/// FN root cause: Round 7's `parenthesized_last_line` fix was too broad.
/// It leaked through `AndNode`/`OrNode` children, so `(expr && !!other)` or
/// `(_options && !!_options[:allow_nil])` inside parens triggered the lenient
/// same-line check. In RuboCop, `find_parent_not_enumerable` returns the `and`
/// node (not `begin`/parens), and `and.begin_type?` is false, so the strict
/// check applies instead.
///
/// Fix (round 8): (1) Stopped clearing `conditional_last_line_stack` in
/// `with_def_body` — outer conditional context now carries through def
/// boundaries, matching RuboCop. (2) Removed `!self.def_info_stack.is_empty()`
/// guards from `visit_if_node`, `visit_unless_node`, `visit_case_node`, and
/// `visit_case_match_node` — conditionals are always tracked so they're visible
/// when a def is entered later. (3) Added `visit_and_node` and `visit_or_node`
/// to clear `parenthesized_last_line`, preventing the lenient parens check from
/// leaking through `and`/`or` nodes.
///
/// Corpus investigation (round 9): remaining FPs were two narrow Parser-vs-Prism
/// mismatches. First, RuboCop treats `define_method`/`define_singleton_method`
/// blocks as method definitions by using the send node itself as the "def"
/// ancestor, so `child_nodes.last` points at the method-name argument instead of
/// the block body. nitrocop used the block body directly, incorrectly flagging
/// `!!` inside non-final statements of those blocks. Second, Parser AST models
/// interpolation as a `begin` node and `case` as a node whose `child_nodes.last`
/// is the last branch body expression. Prism uses `EmbeddedStatementsNode` for
/// interpolation and keeps `CaseNode` opaque, so nitrocop missed RuboCop's
/// begin-like same-line allowance inside conditional interpolation and failed to
/// dig into single-statement `case` bodies.
///
/// Fix (round 9): (1) Treat both receiverless and receiverful
/// `define_method`/`define_singleton_method` calls as RuboCop does by deriving
/// def-body info from the send node's last argument instead of the block body.
/// (2) Treat `EmbeddedStatementsNode` like Parser's `begin` node for the
/// conditional same-line check. (3) Add `CaseNode` dig-in to `parser_last_child`
/// so single-statement case bodies use their last branch expression.
pub struct DoubleNegation;

impl Cop for DoubleNegation {
    fn name(&self) -> &'static str {
        "Style/DoubleNegation"
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
        let enforced_style = config.get_str("EnforcedStyle", "allowed_in_returns");
        let mut visitor = DoubleNegationVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            enforced_style,
            def_info_stack: Vec::new(),
            conditional_last_line_stack: Vec::new(),
            statements_last_line_stack: Vec::new(),
            parent_is_statements: false,
            begin_like_last_line: None,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Info about the enclosing method definition's body.
#[derive(Clone)]
struct DefBodyInfo {
    /// First line of the last child of the def body (1-indexed).
    last_child_first_line: usize,
    /// Last line of the last child of the def body (1-indexed).
    last_child_last_line: usize,
    /// Whether the last child is a hash/pair node (literal hash return).
    last_child_is_hash_or_pair: bool,
    /// Whether the last child is an array or its parent is an array.
    last_child_parent_is_array: bool,
}

struct DoubleNegationVisitor<'a> {
    cop: &'a DoubleNegation,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    enforced_style: &'a str,
    /// Stack of def body info (innermost at top).
    def_info_stack: Vec<DefBodyInfo>,
    /// Stack of conditional ancestor last lines (innermost at top).
    conditional_last_line_stack: Vec<usize>,
    /// Stack of enclosing statements-node last lines. Used for the
    /// `begin_type?` parent check in `double_negative_condition_return_value?`.
    statements_last_line_stack: Vec<usize>,
    /// Whether the current node's non-enumerable parent (skipping pair/hash/
    /// array/keyword_hash) is a StatementsNode. Only when true should the
    /// stmts_last_line check apply — matching RuboCop's
    /// `find_parent_not_enumerable` + `begin_type?` check.
    parent_is_statements: bool,
    /// Last line of the immediately enclosing Parser-`begin` equivalent, if any.
    /// In Parser AST, both parenthesized expressions and interpolation bodies
    /// create `begin` nodes. When this is `Some`, RuboCop uses the lenient
    /// same-line check instead of the strict
    /// `last_child_last_line <= cond_last_line` check.
    begin_like_last_line: Option<usize>,
}

impl DoubleNegationVisitor<'_> {
    fn line_of_offset(&self, offset: usize) -> usize {
        let (line, _) = self.source.offset_to_line_col(offset);
        line
    }

    fn last_line_of_node(&self, start: usize, end: usize) -> usize {
        let adjusted = if end > start { end - 1 } else { start };
        self.line_of_offset(adjusted)
    }

    /// Check if the !! call is preceded by the `return` keyword.
    fn is_after_return_keyword(&self, node: &ruby_prism::CallNode<'_>) -> bool {
        let start = node.location().start_offset();
        let src = self.source.as_bytes();
        if start >= 7 {
            let prefix = &src[..start];
            let trimmed = prefix.trim_ascii_end();
            if trimmed.ends_with(b"return") {
                let before_return = trimmed.len() - 6;
                if before_return == 0 {
                    return true;
                }
                let c = trimmed[before_return - 1];
                if !c.is_ascii_alphanumeric() && c != b'_' {
                    return true;
                }
            }
        }
        false
    }

    fn check_double_negation(&mut self, node: &ruby_prism::CallNode<'_>) {
        // Must be the `!` method
        if node.name().as_slice() != b"!" {
            return;
        }

        // Check the message_loc to ensure it's `!` not `not`
        if let Some(msg_loc) = node.message_loc() {
            if msg_loc.as_slice() == b"not" {
                return;
            }
        }

        // Receiver must also be a `!` call
        let receiver = match node.receiver() {
            Some(r) => r,
            None => return,
        };

        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if inner_call.name().as_slice() != b"!" {
            return;
        }

        // Verify inner is also `!` not `not`
        if let Some(msg_loc) = inner_call.message_loc() {
            if msg_loc.as_slice() == b"not" {
                return;
            }
        }

        // For "allowed_in_returns" style, skip if in return position
        if self.enforced_style == "allowed_in_returns" {
            // Check explicit `return` keyword
            if self.is_after_return_keyword(node) {
                return;
            }

            // Check if in return position using line-based logic matching RuboCop
            if self.is_end_of_method_definition(node) {
                return;
            }
        }

        let loc = node.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Avoid the use of double negation (`!!`).".to_string(),
        ));
    }

    /// RuboCop-compatible `end_of_method_definition?` check.
    fn is_end_of_method_definition(&self, node: &ruby_prism::CallNode<'_>) -> bool {
        let def_info = match self.def_info_stack.last() {
            Some(info) => info,
            None => return false,
        };

        let node_line = self.line_of_offset(node.location().start_offset());

        // If inside a conditional ancestor, use RuboCop's
        // double_negative_condition_return_value? logic
        if let Some(&cond_last_line) = self.conditional_last_line_stack.last() {
            // RuboCop: find_parent_not_enumerable → if parent.begin_type?
            //
            // In Parser AST, parenthesized expressions `(!!expr)` create a
            // `begin` node. `find_parent_not_enumerable` returns this `begin`,
            // and `begin_type?` is true, so the lenient check applies:
            // `node.loc.line == parent.loc.last_line`. For single-line parens,
            // this is trivially true.
            //
            // In Prism, parenthesized expressions create `ParenthesesNode`.
            // When `parenthesized_last_line` is set, apply the same lenient
            // check: `!!` is allowed if on the same line as the parens.
            if let Some(begin_last_line) = self.begin_like_last_line {
                return node_line == begin_last_line;
            }

            // Only apply the statements line check when the !! node's
            // non-enumerable parent IS a StatementsNode (begin_type? in
            // Parser AST). When !! is inside another expression (method call,
            // assignment, hash value, etc.), skip this check.
            if self.parent_is_statements {
                if let Some(&stmts_last_line) = self.statements_last_line_stack.last() {
                    // The "parent" of the !! node in RuboCop terms:
                    // If the parent is a begin node (statement sequence), check if !! is
                    // on the last line of that sequence. This prevents treating `!!foo`
                    // followed by `bar` as a return value even if inside a return-position
                    // conditional.
                    if stmts_last_line != node_line {
                        // !! is not on the last line of its enclosing statements → not a return
                        return false;
                    }
                }
            }
            // Check if the conditional covers the def body's last child
            return def_info.last_child_last_line <= cond_last_line;
        }

        // Not inside a conditional — use the simple line-based check
        // RuboCop: if last_child is pair/hash or parent is array → always offense
        if def_info.last_child_is_hash_or_pair || def_info.last_child_parent_is_array {
            return false;
        }

        // RuboCop: last_child.first_line <= node.first_line
        def_info.last_child_first_line <= node_line
    }

    /// Find the "last child" of a body node, recursing through rescue/ensure.
    fn find_last_child_info(&self, node: &ruby_prism::Node<'_>) -> Option<DefBodyInfo> {
        // Handle StatementsNode: get last child
        if let Some(stmts) = node.as_statements_node() {
            return self.find_last_child_of_stmts(&stmts);
        }

        // Handle BeginNode: may have rescue/ensure
        // RuboCop recurses: rescue → body, ensure → first child
        // In Prism, BeginNode wraps the whole structure; main body is in statements()
        if let Some(begin) = node.as_begin_node() {
            if let Some(stmts) = begin.statements() {
                return self.find_last_child_of_stmts(&stmts);
            }
            return None;
        }

        // Default: this node itself is the "last child"
        Some(self.node_to_def_body_info(node))
    }

    fn find_last_child_of_stmts(
        &self,
        stmts: &ruby_prism::StatementsNode<'_>,
    ) -> Option<DefBodyInfo> {
        let body: Vec<_> = stmts.body().iter().collect();
        let last = body.last()?;

        // In RuboCop's Parser AST, a single-expression def body doesn't get a
        // `begin` wrapper, so `find_last_child` calls `child_nodes.last` directly
        // on the expression (hash → last pair, array → last element, send → last
        // arg). With multiple statements there IS a `begin` wrapper and
        // `child_nodes.last` returns the last statement without digging in.
        //
        // Prism always wraps in StatementsNode. To match RuboCop, when there's
        // exactly one statement, dig into its last child.
        if body.len() == 1 {
            if let Some(hash) = last.as_hash_node() {
                let elements: Vec<_> = hash.elements().iter().collect();
                if let Some(last_pair) = elements.last() {
                    return Some(self.node_to_def_body_info(last_pair));
                }
                // Empty hash — treat the hash itself as last child
                return Some(self.node_to_def_body_info(last));
            }
            if let Some(array) = last.as_array_node() {
                let elements: Vec<_> = array.elements().iter().collect();
                if let Some(last_elem) = elements.last() {
                    // Set parent_is_array = true since we dug into the array
                    let mut info = self.node_to_def_body_info(last_elem);
                    info.last_child_parent_is_array = true;
                    return Some(info);
                }
                return Some(self.node_to_def_body_info(last));
            }
            // For other single-statement bodies (method calls, assignments, etc.),
            // dig into the "last child" to match Parser AST's child_nodes.last.
            // For a CallNode, the last child is the last argument (or block body).
            // If that last child is a hash/keyword_hash, it causes the offense.
            if let Some(last_child) = self.parser_last_child(last) {
                let mut info = self.node_to_def_body_info(&last_child);
                // If the last child is an elsif IfNode, adjust last_child_last_line
                // to exclude the shared `end` keyword (matching Parser AST range).
                if let Some(if_node) = last_child.as_if_node() {
                    let is_elsif = if_node
                        .if_keyword_loc()
                        .is_some_and(|kw| kw.as_slice() == b"elsif");
                    if is_elsif {
                        info.last_child_last_line = self.parser_if_last_line(&if_node);
                    }
                }
                return Some(info);
            }
        }

        Some(self.node_to_def_body_info(last))
    }

    /// Approximate Parser AST's `node.child_nodes.last` for a given Prism node.
    /// Returns the "last child" in Parser AST terms, which for call nodes is
    /// the last argument (or block body), for assignments is the value, etc.
    fn parser_last_child<'n>(&self, node: &ruby_prism::Node<'n>) -> Option<ruby_prism::Node<'n>> {
        // CallNode: last argument (keyword hash or positional), or block body
        if let Some(call) = node.as_call_node() {
            // In Parser AST, blocks wrap the send: (block (send ...) (args) body).
            // child_nodes.last of a block is the body.
            if let Some(block) = call.block() {
                if let Some(block_node) = block.as_block_node() {
                    // Block call: child_nodes.last of a Parser block = body.
                    // Return the block body (StatementsNode) directly, matching
                    // Parser's child_nodes.last which returns the begin wrapper
                    // (or single expression) — NOT the last statement within it.
                    // The caller uses the body's first_line for the return-position
                    // check: `last_child_first_line <= node_line`.
                    return block_node.body();
                }
            }
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                return arg_list.into_iter().last();
            }
            // No arguments: last child is receiver (if any)
            return call.receiver();
        }

        // IfNode: child_nodes.last in Parser = else clause (inner if for elsif,
        // else body, or nil). child_nodes filters out nil, so for `if` without
        // else: last child = the then-body.
        if let Some(if_node) = node.as_if_node() {
            if let Some(subsequent) = if_node.subsequent() {
                // elsif → another IfNode; else → ElseNode
                if let Some(inner_if) = subsequent.as_if_node() {
                    return Some(inner_if.as_node());
                }
                if let Some(else_node) = subsequent.as_else_node() {
                    // Parser's child_nodes.last of an if-with-else = the else body
                    // (expression or begin). In Prism, ElseNode wraps statements.
                    if let Some(stmts) = else_node.statements() {
                        return Some(stmts.as_node());
                    }
                    // Empty else: no last child from the else
                    return if_node.statements().map(|s| s.as_node());
                }
            }
            // No else/elsif: last child = the then-body (statements)
            return if_node.statements().map(|s| s.as_node());
        }

        // CaseNode: child_nodes.last in Parser = else body if present, otherwise
        // the last when-branch body.
        if let Some(case_node) = node.as_case_node() {
            if let Some(else_node) = case_node.else_clause() {
                if let Some(stmts) = else_node.statements() {
                    return Some(stmts.as_node());
                }
            }

            if let Some(last_when) = case_node.conditions().iter().last() {
                if let Some(when_node) = last_when.as_when_node() {
                    return when_node.statements().map(|s| s.as_node());
                }
                return Some(last_when);
            }
        }

        // OrNode: child_nodes.last = right side
        if let Some(or_node) = node.as_or_node() {
            return Some(or_node.right());
        }

        // AndNode: child_nodes.last = right side
        if let Some(and_node) = node.as_and_node() {
            return Some(and_node.right());
        }

        // LocalVariableWriteNode: value is the last child
        if let Some(lvar) = node.as_local_variable_write_node() {
            return Some(lvar.value());
        }

        // InstanceVariableWriteNode
        if let Some(ivar) = node.as_instance_variable_write_node() {
            return Some(ivar.value());
        }

        // InstanceVariableOrWriteNode (||=)
        if let Some(ivar_or) = node.as_instance_variable_or_write_node() {
            return Some(ivar_or.value());
        }

        // InstanceVariableAndWriteNode (&&=)
        if let Some(ivar_and) = node.as_instance_variable_and_write_node() {
            return Some(ivar_and.value());
        }

        // LocalVariableOrWriteNode (||=)
        if let Some(lvar_or) = node.as_local_variable_or_write_node() {
            return Some(lvar_or.value());
        }

        // LocalVariableAndWriteNode (&&=)
        if let Some(lvar_and) = node.as_local_variable_and_write_node() {
            return Some(lvar_and.value());
        }

        // ClassVariableOrWriteNode / AndWriteNode
        if let Some(cv_or) = node.as_class_variable_or_write_node() {
            return Some(cv_or.value());
        }
        if let Some(cv_and) = node.as_class_variable_and_write_node() {
            return Some(cv_and.value());
        }

        // GlobalVariableOrWriteNode / AndWriteNode
        if let Some(gv_or) = node.as_global_variable_or_write_node() {
            return Some(gv_or.value());
        }
        if let Some(gv_and) = node.as_global_variable_and_write_node() {
            return Some(gv_and.value());
        }

        // ConstantOrWriteNode / AndWriteNode
        if let Some(c_or) = node.as_constant_or_write_node() {
            return Some(c_or.value());
        }
        if let Some(c_and) = node.as_constant_and_write_node() {
            return Some(c_and.value());
        }

        // ConstantPathWriteNode
        if let Some(cpw) = node.as_constant_path_write_node() {
            return Some(cpw.value());
        }

        // ClassVariableWriteNode
        if let Some(cvw) = node.as_class_variable_write_node() {
            return Some(cvw.value());
        }

        // GlobalVariableWriteNode
        if let Some(gvw) = node.as_global_variable_write_node() {
            return Some(gvw.value());
        }

        // ConstantWriteNode
        if let Some(cw) = node.as_constant_write_node() {
            return Some(cw.value());
        }

        None
    }

    fn node_to_def_body_info(&self, node: &ruby_prism::Node<'_>) -> DefBodyInfo {
        let first_line = self.line_of_offset(node.location().start_offset());
        let last_line =
            self.last_line_of_node(node.location().start_offset(), node.location().end_offset());

        let is_hash_or_pair = node.as_hash_node().is_some()
            || node.as_keyword_hash_node().is_some()
            || node.as_assoc_node().is_some()
            || node.as_assoc_splat_node().is_some();

        // parent_is_array is set by the caller when digging into an array;
        // by default it's false
        DefBodyInfo {
            last_child_first_line: first_line,
            last_child_last_line: last_line,
            last_child_is_hash_or_pair: is_hash_or_pair,
            last_child_parent_is_array: false,
        }
    }

    /// Compute the "Parser-equivalent" last line for an IfNode.
    ///
    /// In Parser AST, elsif creates a nested if node whose range excludes the
    /// shared `end` keyword. For `if a; 1; elsif b; 2; end`, Parser gives the
    /// inner if (from elsif) a range of L3-4 (body only), while Prism gives
    /// L3-5 (including `end`). For `if a; ...; elsif b; ...; else; ...; end`,
    /// Parser gives the inner if L3-6 (through else body), Prism gives L3-7.
    ///
    /// This method returns the content-based last line (matching Parser) by
    /// walking the subsequent chain without including the `end` keyword.
    fn parser_if_last_line(&self, node: &ruby_prism::IfNode<'_>) -> usize {
        if let Some(subsequent) = node.subsequent() {
            if let Some(inner_if) = subsequent.as_if_node() {
                // Another elsif: recurse
                return self.parser_if_last_line(&inner_if);
            }
            if let Some(else_node) = subsequent.as_else_node() {
                // else clause: use the else body's last line
                if let Some(stmts) = else_node.statements() {
                    return self.last_line_of_node(
                        stmts.location().start_offset(),
                        stmts.location().end_offset(),
                    );
                }
                // Empty else: use the else keyword's line
                return self.line_of_offset(else_node.else_keyword_loc().start_offset());
            }
        }
        // No subsequent: use the body's last line
        if let Some(stmts) = node.statements() {
            return self.last_line_of_node(
                stmts.location().start_offset(),
                stmts.location().end_offset(),
            );
        }
        // Fallback: use the predicate's line
        self.line_of_offset(node.predicate().location().start_offset())
    }

    /// Enter a method body: compute last-child info, push to stack, visit body, pop.
    fn with_def_info<F>(&mut self, info: Option<DefBodyInfo>, visit_fn: F)
    where
        F: FnOnce(&mut Self),
    {
        let prev_def_len = self.def_info_stack.len();

        if let Some(info) = info {
            self.def_info_stack.push(info);
        }

        // Save and clear statements stack and flags — these don't cross def boundaries.
        // IMPORTANT: conditional_last_line_stack is NOT cleared here. RuboCop's
        // find_conditional_node_from_ascendant walks past def boundaries, so a
        // conditional wrapping the def (e.g., `if has_ssl; class X; def foo; ...`)
        // is visible inside the def. This matches RuboCop's behavior where the
        // outer conditional's last_line makes `last_child.last_line <= cond_last_line`
        // true, allowing `!!` inside the def.
        let saved_stmts = std::mem::take(&mut self.statements_last_line_stack);
        let saved_parent_is_statements = self.parent_is_statements;
        let saved_begin_like = self.begin_like_last_line;
        self.parent_is_statements = false;
        self.begin_like_last_line = None;

        visit_fn(self);

        self.def_info_stack.truncate(prev_def_len);
        self.statements_last_line_stack = saved_stmts;
        self.parent_is_statements = saved_parent_is_statements;
        self.begin_like_last_line = saved_begin_like;
    }

    fn with_def_body<F>(&mut self, body: Option<ruby_prism::Node<'_>>, visit_fn: F)
    where
        F: FnOnce(&mut Self),
    {
        let info = body
            .as_ref()
            .and_then(|body_node| self.find_last_child_info(body_node));
        self.with_def_info(info, visit_fn);
    }

    fn define_method_call_info(&self, node: &ruby_prism::CallNode<'_>) -> Option<DefBodyInfo> {
        if let Some(args) = node.arguments() {
            if let Some(last_arg) = args.arguments().iter().last() {
                return Some(self.node_to_def_body_info(&last_arg));
            }
        }

        node.receiver()
            .map(|receiver| self.node_to_def_body_info(&receiver))
    }
}

impl<'pr> Visit<'pr> for DoubleNegationVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.check_double_negation(node);

        // After checking this node, clear parent_is_statements for children.
        // Children of a call node are not direct children of the StatementsNode.
        let saved_parent = self.parent_is_statements;
        // Clear begin-like context for children: `!!` inside a deeper
        // call is no longer directly wrapped by Parser's `begin` equivalent.
        let saved_begin_like = self.begin_like_last_line;
        self.parent_is_statements = false;
        self.begin_like_last_line = None;

        // Check if this is a define_method or define_singleton_method call with a block
        if let Some(block) = node.block() {
            if block.as_block_node().is_some() {
                let method_name = node.name().as_slice();
                if method_name == b"define_method" || method_name == b"define_singleton_method" {
                    let info = self.define_method_call_info(node);
                    self.with_def_info(info, |this| {
                        ruby_prism::visit_call_node(this, node);
                    });
                    self.parent_is_statements = saved_parent;
                    self.begin_like_last_line = saved_begin_like;
                    return;
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
        self.parent_is_statements = saved_parent;
        self.begin_like_last_line = saved_begin_like;
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let body = node.body();
        self.with_def_body(body, |this| {
            ruby_prism::visit_def_node(this, node);
        });
    }

    fn visit_parentheses_node(&mut self, node: &ruby_prism::ParenthesesNode<'pr>) {
        // In Parser AST, parenthesized expressions `(expr)` create a `begin`
        // node. RuboCop's `find_parent_not_enumerable` returns this `begin`,
        // and `begin_type?` allows the lenient same-line check in
        // `double_negative_condition_return_value?`. Track the parentheses
        // last line so `is_end_of_method_definition` can apply the same logic.
        if !self.def_info_stack.is_empty() && !self.conditional_last_line_stack.is_empty() {
            let saved = self.begin_like_last_line;
            self.begin_like_last_line =
                Some(self.last_line_of_node(
                    node.location().start_offset(),
                    node.location().end_offset(),
                ));
            ruby_prism::visit_parentheses_node(self, node);
            self.begin_like_last_line = saved;
        } else {
            ruby_prism::visit_parentheses_node(self, node);
        }
    }

    fn visit_embedded_statements_node(&mut self, node: &ruby_prism::EmbeddedStatementsNode<'pr>) {
        // Parser wraps interpolation bodies (`"#{expr}"`) in a `begin` node.
        // Under a conditional ancestor, RuboCop uses the same lenient same-line
        // check as it does for parenthesized expressions.
        if !self.def_info_stack.is_empty() && !self.conditional_last_line_stack.is_empty() {
            let saved = self.begin_like_last_line;
            self.begin_like_last_line =
                Some(self.last_line_of_node(
                    node.location().start_offset(),
                    node.location().end_offset(),
                ));
            ruby_prism::visit_embedded_statements_node(self, node);
            self.begin_like_last_line = saved;
        } else {
            ruby_prism::visit_embedded_statements_node(self, node);
        }
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        // Always push conditional context, even outside def bodies.
        // RuboCop's find_conditional_node_from_ascendant walks past def
        // boundaries, so outer conditionals wrapping a def are visible
        // inside it. We must track them here so they're on the stack when
        // a def is entered later.
        //
        // For elsif IfNodes, use the Parser-equivalent last line that
        // excludes the shared `end` keyword. This is critical because
        // Prism includes `end` in elsif IfNode ranges while Parser doesn't,
        // causing incorrect return-position detection.
        let is_elsif = node
            .if_keyword_loc()
            .is_some_and(|kw| kw.as_slice() == b"elsif");
        let last_line = if is_elsif {
            self.parser_if_last_line(node)
        } else {
            self.last_line_of_node(node.location().start_offset(), node.location().end_offset())
        };
        self.conditional_last_line_stack.push(last_line);
        // Clear statements stack: the condition is not inside a StatementsNode
        // within this conditional, so the begin_type? check should not apply.
        // StatementsNodes inside branches will re-push as they're visited.
        let saved_stmts = std::mem::take(&mut self.statements_last_line_stack);
        ruby_prism::visit_if_node(self, node);
        self.statements_last_line_stack = saved_stmts;
        self.conditional_last_line_stack.pop();
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let last_line =
            self.last_line_of_node(node.location().start_offset(), node.location().end_offset());
        self.conditional_last_line_stack.push(last_line);
        let saved_stmts = std::mem::take(&mut self.statements_last_line_stack);
        ruby_prism::visit_unless_node(self, node);
        self.statements_last_line_stack = saved_stmts;
        self.conditional_last_line_stack.pop();
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        let last_line =
            self.last_line_of_node(node.location().start_offset(), node.location().end_offset());
        self.conditional_last_line_stack.push(last_line);
        let saved_stmts = std::mem::take(&mut self.statements_last_line_stack);
        ruby_prism::visit_case_node(self, node);
        self.statements_last_line_stack = saved_stmts;
        self.conditional_last_line_stack.pop();
    }

    fn visit_case_match_node(&mut self, node: &ruby_prism::CaseMatchNode<'pr>) {
        let last_line =
            self.last_line_of_node(node.location().start_offset(), node.location().end_offset());
        self.conditional_last_line_stack.push(last_line);
        let saved_stmts = std::mem::take(&mut self.statements_last_line_stack);
        ruby_prism::visit_case_match_node(self, node);
        self.statements_last_line_stack = saved_stmts;
        self.conditional_last_line_stack.pop();
    }

    fn visit_and_node(&mut self, node: &ruby_prism::AndNode<'pr>) {
        // In RuboCop, `find_parent_not_enumerable` returns the `and` node as a
        // non-enumerable parent. `and.begin_type?` is false, so the lenient
        // same-line check does NOT apply. Clear `parenthesized_last_line` so
        // that `!!` inside `(expr && !!other)` uses the strict check.
        let saved = self.begin_like_last_line;
        self.begin_like_last_line = None;
        ruby_prism::visit_and_node(self, node);
        self.begin_like_last_line = saved;
    }

    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'pr>) {
        // Same as visit_and_node: `or` is non-enumerable, not begin_type?.
        let saved = self.begin_like_last_line;
        self.begin_like_last_line = None;
        ruby_prism::visit_or_node(self, node);
        self.begin_like_last_line = saved;
    }

    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        if !self.def_info_stack.is_empty() {
            let last_line = self
                .last_line_of_node(node.location().start_offset(), node.location().end_offset());
            self.statements_last_line_stack.push(last_line);

            // In Parser AST, only multi-statement bodies get a `begin` wrapper.
            // Single-statement bodies are unwrapped. Prism always wraps in
            // StatementsNode. To match RuboCop's `begin_type?` check, only set
            // parent_is_statements when there are multiple statements.
            //
            // IMPORTANT: parent_is_statements must only be true for DIRECT
            // children of this StatementsNode. When a child (e.g., assignment)
            // visits its own subtree, parent_is_statements must be false.
            // We achieve this by visiting each child manually: set the flag
            // true before each direct child, then restore it after.
            let stmt_count = node.body().iter().count();
            let is_multi = stmt_count > 1;
            let saved = self.parent_is_statements;

            for child in node.body().iter() {
                // Only set parent_is_statements true when the direct child
                // IS a CallNode (the only type that checks the flag via
                // check_double_negation). For non-CallNode children
                // (assignments, etc.), set false so nested CallNodes in
                // their subtrees don't incorrectly see the flag as true.
                self.parent_is_statements = is_multi && child.as_call_node().is_some();
                self.visit(&child);
            }

            self.parent_is_statements = saved;
            self.statements_last_line_stack.pop();
        } else {
            ruby_prism::visit_statements_node(self, node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DoubleNegation, "cops/style/double_negation");
}
