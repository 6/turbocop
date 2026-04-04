use crate::cop::shared::node_type::{CASE_MATCH_NODE, CASE_NODE, IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Style/IdenticalConditionalBranches
///
/// Checks for identical expressions at the beginning (head) or end (tail) of
/// each branch of a conditional expression: `if/elsif/else`, `unless/else`,
/// `case/when/else`, and `case/in/else` (pattern matching).
///
/// ## Investigation findings (round 2)
///
/// 1. **FN: `unless/else` support** — Prism uses a separate `UnlessNode` type
///    (not `IfNode`). The cop now handles `UNLESS_NODE` to detect identical
///    heads/tails in `unless/else` blocks.
///
/// 2. **FP: assignment value vs condition variable** — RuboCop's
///    `duplicated_expressions?` suppresses identical assignments when the
///    value (RHS) matches a variable in the condition (e.g.,
///    `if obj.is_a?(X); @y = obj; else; @y = obj; end` inside a method
///    where `obj` is a local variable). Added `assignment_child_source`
///    check for both heads and tails.
///
/// 3. **FP: conditional inside assignment** — `y = if cond; ...; end`
///    makes the conditional the "last child" of the assignment node.
///    RuboCop's `last_child_of_parent?` returns true, suppressing single-
///    child-branch head checks. Fixed `is_last_child_of_parent` to also
///    check write nodes (LocalVariableWriteNode, etc.).
///
/// 4. **FP: setter-call assignments reusing the condition receiver** —
///    RuboCop treats `object.foo = value` like an assignment whose compared
///    child node is the receiver (`object`). nitrocop only handled simple
///    write nodes and `[]=`. Added setter-call receiver extraction so cases
///    like `if object.present?; object.attributes = ...; else ... end` are
///    suppressed.
///
/// 5. **FN: deep condition call chains** — RuboCop only checks direct child
///    nodes of the condition when suppressing duplicated assignments.
///    nitrocop walked all descendants, which over-suppressed offenses like
///    `if str.to_s.strip.empty?; @distance_string = str; else ... end`.
///    Narrowed the condition-variable check to direct child variables only.
///
/// 6. **FP: single-child branch tails that are nested conditionals** —
///    RuboCop does not report `if`/`unless` expressions when they are the only
///    statement in every branch and the enclosing conditional is the last
///    expression of its parent. nitrocop compared those nested conditionals as
///    ordinary tail statements and flagged them anyway.
///
/// 7. **FP: regex literals with whitespace-sensitive bodies** — nitrocop's
///    source normalizer collapsed spaces inside `/.../`, so distinct regexes
///    like `/[^\d ]/` and `/[^\d]/` compared equal even though RuboCop keeps
///    regexp bodies whitespace-sensitive. Regex literals now keep their raw
///    trimmed source as the comparison key.
///
/// 8. **FP/FN: terminal `else` with a single nested conditional** —
///    RuboCop expands `else` bodies that contain a single nested `if` like an
///    `elsif` chain, but it bails entirely when that nested conditional is not
///    exhaustive (for example, an `unless` without `else`, or a modifier `if`).
///    Prism keeps that structure inside an explicit `ElseNode`, so nitrocop has
///    to flatten or bail explicitly to match RuboCop.
///
/// ## Investigation findings (round 3) — AST fingerprinting
///
/// Replaced source-text comparison (`normalized_source_key`) with AST-based
/// fingerprinting (`node_fp`). RuboCop compares AST nodes for equality, not
/// source text, so expressions that differ only in surface syntax (optional
/// parens, string escape representation, trailing comments) must compare
/// equal, while expressions with the same text but different AST structure
/// (`__LINE__` on different lines, lvar vs bare method call) must differ.
///
/// 9.  **FN: string escape representation** — `"\u2028"` and `"\342\200\250"`
///     have identical unescaped bytes. AST fingerprinting uses `unescaped()`
///     for `StringNode`/`SymbolNode`, matching RuboCop's AST equality.
///
/// 10. **FN: optional parentheses** — `set_header RACK_REQUEST_FORM_PAIRS, x`
///     and `set_header(RACK_REQUEST_FORM_PAIRS, x)` are the same `CallNode`.
///     `call_node_fp` fingerprints structurally, ignoring parens.
///
/// 11. **FN: comment-only differences** — Comments are not AST nodes, so
///     `do_x # comment A` and `do_x # comment B` have the same AST.
///     `node_fp` operates on the AST, naturally ignoring comments.
///
/// 12. **FP: `__LINE__` on different lines** — `__LINE__` is a
///     `SourceLineNode` that evaluates to its line number. Two branches with
///     `__LINE__` on different lines are semantically different. Fingerprint
///     includes the actual line number.
///
/// 13. **FP: lvar vs bare method call** — After assignment in one branch,
///     `response` becomes a `LocalVariableReadNode`; in the other branch
///     (before the parser sees the assignment) it remains a `CallNode`.
///     Fingerprinting distinguishes `LVR:` from `C:` prefixes.
///
/// Remaining FP (1): RuboCop crashes on single-line ternary `chars.shift`
/// inside `if` assignment — this is a RuboCop bug, not fixable on our side.
pub struct IdenticalConditionalBranches;

struct StatementInfo {
    src: String,
    key: Vec<u8>,
    line: usize,
    col: usize,
    has_heredoc: bool,
    index_assignment_receiver: Option<String>,
    /// Source of the "first child node" for assignments, used by RuboCop's
    /// `duplicated_expressions?` to suppress when the value (for simple writes)
    /// or LHS variable name (for operator writes) matches a condition variable.
    assignment_child_source: Option<String>,
    is_if_or_unless: bool,
}

fn node_source(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    let loc = node.location();
    let src = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
    String::from_utf8_lossy(src).trim().to_string()
}

// ---------------------------------------------------------------------------
// AST fingerprinting — produces a canonical byte key for each statement that
// matches RuboCop's AST-level comparison semantics:
//   • ignores optional parentheses in method calls
//   • ignores comments
//   • compares string content (unescaped), not source representation
//   • distinguishes local variable reads from bare method calls
//   • uses actual line number for __LINE__
// ---------------------------------------------------------------------------

/// Build a fingerprint for a Prism AST node into `out`.
fn node_fp(source: &SourceFile, bytes: &[u8], node: &ruby_prism::Node<'_>, out: &mut Vec<u8>) {
    // String literal: compare by unescaped content
    if let Some(s) = node.as_string_node() {
        out.extend_from_slice(b"S:");
        out.extend_from_slice(s.unescaped());
        return;
    }

    // Symbol literal: compare by unescaped name
    if let Some(s) = node.as_symbol_node() {
        out.extend_from_slice(b"Y:");
        out.extend_from_slice(s.unescaped());
        return;
    }

    // CallNode: structural fingerprint independent of optional parentheses
    if let Some(call) = node.as_call_node() {
        call_node_fp(source, bytes, &call, out);
        return;
    }

    // Local variable read: distinguished from bare method calls
    if let Some(lvar) = node.as_local_variable_read_node() {
        out.extend_from_slice(b"LVR:");
        out.extend_from_slice(lvar.name().as_slice());
        return;
    }

    // Instance variable read
    if let Some(ivar) = node.as_instance_variable_read_node() {
        out.extend_from_slice(b"IVR:");
        out.extend_from_slice(ivar.name().as_slice());
        return;
    }

    // Class variable read
    if let Some(cv) = node.as_class_variable_read_node() {
        out.extend_from_slice(b"CVR:");
        out.extend_from_slice(cv.name().as_slice());
        return;
    }

    // Global variable read
    if let Some(gv) = node.as_global_variable_read_node() {
        out.extend_from_slice(b"GVR:");
        out.extend_from_slice(gv.name().as_slice());
        return;
    }

    // Local variable write
    if let Some(w) = node.as_local_variable_write_node() {
        out.extend_from_slice(b"LVW:");
        out.extend_from_slice(w.name().as_slice());
        out.push(b'=');
        node_fp(source, bytes, &w.value(), out);
        return;
    }

    // Instance variable write
    if let Some(w) = node.as_instance_variable_write_node() {
        out.extend_from_slice(b"IVW:");
        out.extend_from_slice(w.name().as_slice());
        out.push(b'=');
        node_fp(source, bytes, &w.value(), out);
        return;
    }

    // Class variable write
    if let Some(w) = node.as_class_variable_write_node() {
        out.extend_from_slice(b"CVW:");
        out.extend_from_slice(w.name().as_slice());
        out.push(b'=');
        node_fp(source, bytes, &w.value(), out);
        return;
    }

    // Global variable write
    if let Some(w) = node.as_global_variable_write_node() {
        out.extend_from_slice(b"GVW:");
        out.extend_from_slice(w.name().as_slice());
        out.push(b'=');
        node_fp(source, bytes, &w.value(), out);
        return;
    }

    // Constant write
    if let Some(w) = node.as_constant_write_node() {
        out.extend_from_slice(b"CW:");
        out.extend_from_slice(w.name().as_slice());
        out.push(b'=');
        node_fp(source, bytes, &w.value(), out);
        return;
    }

    // Constant read
    if let Some(c) = node.as_constant_read_node() {
        out.extend_from_slice(b"CR:");
        out.extend_from_slice(c.name().as_slice());
        return;
    }

    // Constant path (e.g. Foo::Bar): use source text since it has no child()
    if node.as_constant_path_node().is_some() {
        out.extend_from_slice(b"CP:");
        let loc = node.location();
        out.extend_from_slice(&bytes[loc.start_offset()..loc.end_offset()]);
        return;
    }

    // Array
    if let Some(array) = node.as_array_node() {
        out.push(b'[');
        for (i, elem) in array.elements().iter().enumerate() {
            if i > 0 {
                out.push(b',');
            }
            node_fp(source, bytes, &elem, out);
        }
        out.push(b']');
        return;
    }

    // Hash
    if let Some(hash) = node.as_hash_node() {
        out.extend_from_slice(b"H{");
        for (i, elem) in hash.elements().iter().enumerate() {
            if i > 0 {
                out.push(b',');
            }
            node_fp(source, bytes, &elem, out);
        }
        out.push(b'}');
        return;
    }

    // Keyword hash (implicit hash in arguments)
    if let Some(hash) = node.as_keyword_hash_node() {
        out.extend_from_slice(b"H{");
        for (i, elem) in hash.elements().iter().enumerate() {
            if i > 0 {
                out.push(b',');
            }
            node_fp(source, bytes, &elem, out);
        }
        out.push(b'}');
        return;
    }

    // Assoc (key => value pair)
    if let Some(assoc) = node.as_assoc_node() {
        out.extend_from_slice(b"P(");
        node_fp(source, bytes, &assoc.key(), out);
        out.extend_from_slice(b"=>");
        node_fp(source, bytes, &assoc.value(), out);
        out.push(b')');
        return;
    }

    // Assoc splat (**hash)
    if let Some(splat) = node.as_assoc_splat_node() {
        out.extend_from_slice(b"AS:");
        if let Some(value) = splat.value() {
            node_fp(source, bytes, &value, out);
        }
        return;
    }

    // Return node
    if let Some(ret) = node.as_return_node() {
        out.extend_from_slice(b"RET(");
        if let Some(args) = ret.arguments() {
            for (i, arg) in args.arguments().iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                node_fp(source, bytes, &arg, out);
            }
        }
        out.push(b')');
        return;
    }

    // Block node
    if let Some(block) = node.as_block_node() {
        out.extend_from_slice(b"BLK(");
        match block.parameters() {
            Some(params) => node_fp(source, bytes, &params, out),
            None => out.push(b'-'),
        }
        out.push(b'|');
        if let Some(body) = block.body() {
            node_fp(source, bytes, &body, out);
        }
        out.push(b')');
        return;
    }

    // If node (also handles nested conditionals in heads/tails)
    if let Some(if_node) = node.as_if_node() {
        out.extend_from_slice(b"IF(");
        node_fp(source, bytes, &if_node.predicate(), out);
        out.push(b'|');
        if let Some(stmts) = if_node.statements() {
            stmts_fp(source, bytes, &stmts, out);
        }
        out.push(b'|');
        if let Some(sub) = if_node.subsequent() {
            node_fp(source, bytes, &sub, out);
        }
        out.push(b')');
        return;
    }

    // Unless node
    if let Some(unless) = node.as_unless_node() {
        out.extend_from_slice(b"UNLESS(");
        node_fp(source, bytes, &unless.predicate(), out);
        out.push(b'|');
        if let Some(stmts) = unless.statements() {
            stmts_fp(source, bytes, &stmts, out);
        }
        out.push(b'|');
        if let Some(ec) = unless.else_clause() {
            node_fp(source, bytes, &ec.as_node(), out);
        }
        out.push(b')');
        return;
    }

    // Else node
    if let Some(else_node) = node.as_else_node() {
        out.extend_from_slice(b"ELSE(");
        if let Some(stmts) = else_node.statements() {
            stmts_fp(source, bytes, &stmts, out);
        }
        out.push(b')');
        return;
    }

    // Statements node
    if let Some(stmts) = node.as_statements_node() {
        stmts_fp(source, bytes, &stmts, out);
        return;
    }

    // __LINE__: include actual line number so different lines differ
    if node.as_source_line_node().is_some() {
        let (line, _) = source.offset_to_line_col(node.location().start_offset());
        out.extend_from_slice(b"SL:");
        out.extend_from_slice(line.to_string().as_bytes());
        return;
    }

    // __FILE__
    if node.as_source_file_node().is_some() {
        out.extend_from_slice(b"SF");
        return;
    }

    // __ENCODING__
    if node.as_source_encoding_node().is_some() {
        out.extend_from_slice(b"SE");
        return;
    }

    // Integer literal
    if let Some(int) = node.as_integer_node() {
        out.extend_from_slice(b"I:");
        let val = int.value();
        let (neg, digits) = val.to_u32_digits();
        if neg {
            out.push(b'-');
        }
        for d in digits {
            out.extend_from_slice(&d.to_le_bytes());
        }
        return;
    }

    // Float literal
    if let Some(f) = node.as_float_node() {
        out.extend_from_slice(b"F:");
        out.extend_from_slice(f.value().to_bits().to_le_bytes().as_slice());
        return;
    }

    // nil / true / false / self
    if node.as_nil_node().is_some() {
        out.extend_from_slice(b"nil");
        return;
    }
    if node.as_true_node().is_some() {
        out.extend_from_slice(b"true");
        return;
    }
    if node.as_false_node().is_some() {
        out.extend_from_slice(b"false");
        return;
    }
    if node.as_self_node().is_some() {
        out.extend_from_slice(b"self");
        return;
    }

    // Splat (*expr)
    if let Some(splat) = node.as_splat_node() {
        out.push(b'*');
        if let Some(expr) = splat.expression() {
            node_fp(source, bytes, &expr, out);
        }
        return;
    }

    // Regex: use unescaped content (whitespace-sensitive)
    if let Some(regex) = node.as_regular_expression_node() {
        out.extend_from_slice(b"RE:");
        out.extend_from_slice(regex.unescaped());
        return;
    }

    // Interpolated regex: use source (whitespace-sensitive)
    if node.as_interpolated_regular_expression_node().is_some() {
        let loc = node.location();
        out.extend_from_slice(b"IRE:");
        out.extend_from_slice(&bytes[loc.start_offset()..loc.end_offset()]);
        return;
    }

    // Interpolated string: structural with parts
    if let Some(istr) = node.as_interpolated_string_node() {
        out.extend_from_slice(b"IS:");
        for (i, part) in istr.parts().iter().enumerate() {
            if i > 0 {
                out.push(b'+');
            }
            node_fp(source, bytes, &part, out);
        }
        return;
    }

    // Embedded statements (#{...} inside strings)
    if let Some(es) = node.as_embedded_statements_node() {
        out.extend_from_slice(b"ES(");
        if let Some(stmts) = es.statements() {
            stmts_fp(source, bytes, &stmts, out);
        }
        out.push(b')');
        return;
    }

    // Multi-write (parallel assignment like X, Y = ...)
    if let Some(mw) = node.as_multi_write_node() {
        out.extend_from_slice(b"MW(");
        for (i, target) in mw.lefts().iter().enumerate() {
            if i > 0 {
                out.push(b',');
            }
            node_fp(source, bytes, &target, out);
        }
        out.push(b'=');
        node_fp(source, bytes, &mw.value(), out);
        out.push(b')');
        return;
    }

    // Parentheses node
    if let Some(pn) = node.as_parentheses_node() {
        out.push(b'(');
        if let Some(body) = pn.body() {
            node_fp(source, bytes, &body, out);
        }
        out.push(b')');
        return;
    }

    // Block argument node
    if let Some(ba) = node.as_block_argument_node() {
        out.push(b'&');
        if let Some(expr) = ba.expression() {
            node_fp(source, bytes, &expr, out);
        }
        return;
    }

    // Range node
    if let Some(range) = node.as_range_node() {
        out.extend_from_slice(b"R(");
        if let Some(left) = range.left() {
            node_fp(source, bytes, &left, out);
        }
        // Use operator to distinguish .. from ...
        out.extend_from_slice(range.operator_loc().as_slice());
        if let Some(right) = range.right() {
            node_fp(source, bytes, &right, out);
        }
        out.push(b')');
        return;
    }

    // Operator write nodes (+=, ||=, &&=)
    if let Some(w) = node.as_local_variable_operator_write_node() {
        out.extend_from_slice(b"LVOW:");
        out.extend_from_slice(w.name().as_slice());
        out.extend_from_slice(w.binary_operator_loc().as_slice());
        node_fp(source, bytes, &w.value(), out);
        return;
    }
    if let Some(w) = node.as_instance_variable_operator_write_node() {
        out.extend_from_slice(b"IVOW:");
        out.extend_from_slice(w.name().as_slice());
        out.extend_from_slice(w.binary_operator_loc().as_slice());
        node_fp(source, bytes, &w.value(), out);
        return;
    }
    if let Some(w) = node.as_local_variable_or_write_node() {
        out.extend_from_slice(b"LVORW:");
        out.extend_from_slice(w.name().as_slice());
        out.extend_from_slice(b"||=");
        node_fp(source, bytes, &w.value(), out);
        return;
    }
    if let Some(w) = node.as_local_variable_and_write_node() {
        out.extend_from_slice(b"LVANW:");
        out.extend_from_slice(w.name().as_slice());
        out.extend_from_slice(b"&&=");
        node_fp(source, bytes, &w.value(), out);
        return;
    }
    if let Some(w) = node.as_instance_variable_or_write_node() {
        out.extend_from_slice(b"IVORW:");
        out.extend_from_slice(w.name().as_slice());
        out.extend_from_slice(b"||=");
        node_fp(source, bytes, &w.value(), out);
        return;
    }
    if let Some(w) = node.as_instance_variable_and_write_node() {
        out.extend_from_slice(b"IVANW:");
        out.extend_from_slice(w.name().as_slice());
        out.extend_from_slice(b"&&=");
        node_fp(source, bytes, &w.value(), out);
        return;
    }

    // Defined? node
    if let Some(d) = node.as_defined_node() {
        out.extend_from_slice(b"DEF?(");
        node_fp(source, bytes, &d.value(), out);
        out.push(b')');
        return;
    }

    // Yield node
    if let Some(y) = node.as_yield_node() {
        out.extend_from_slice(b"YIELD(");
        if let Some(args) = y.arguments() {
            for (i, arg) in args.arguments().iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                node_fp(source, bytes, &arg, out);
            }
        }
        out.push(b')');
        return;
    }

    // Fallback: source-based fingerprint with comment stripping
    source_based_fp(bytes, node, out);
}

/// Fingerprint a StatementsNode body.
fn stmts_fp(
    source: &SourceFile,
    bytes: &[u8],
    stmts: &ruby_prism::StatementsNode<'_>,
    out: &mut Vec<u8>,
) {
    for (i, node) in stmts.body().iter().enumerate() {
        if i > 0 {
            out.push(b'\x00');
        }
        node_fp(source, bytes, &node, out);
    }
}

/// Build a structural fingerprint for a CallNode, independent of optional
/// parentheses. `foo(x)` and `foo x` produce the same fingerprint.
fn call_node_fp(
    source: &SourceFile,
    bytes: &[u8],
    call: &ruby_prism::CallNode<'_>,
    out: &mut Vec<u8>,
) {
    out.extend_from_slice(b"C:");
    if let Some(recv) = call.receiver() {
        node_fp(source, bytes, &recv, out);
        if let Some(op) = call.call_operator_loc() {
            out.extend_from_slice(op.as_slice());
        } else {
            out.push(b'.');
        }
    }
    out.extend_from_slice(call.name().as_slice());
    out.push(b'(');
    if let Some(args) = call.arguments() {
        for (i, arg) in args.arguments().iter().enumerate() {
            if i > 0 {
                out.push(b',');
            }
            node_fp(source, bytes, &arg, out);
        }
    }
    out.push(b')');
    if let Some(block) = call.block() {
        out.push(b'{');
        node_fp(source, bytes, &block, out);
        out.push(b'}');
    }
}

/// Fallback: source-based fingerprint that strips comments and normalizes
/// whitespace, preserving content inside string/regex literals.
fn source_based_fp(bytes: &[u8], node: &ruby_prism::Node<'_>, out: &mut Vec<u8>) {
    let loc = node.location();
    let start = loc.start_offset();
    let end = loc.end_offset().min(bytes.len());
    if start >= end {
        return;
    }
    let raw = &bytes[start..end];
    let stripped = strip_comments(raw);
    let normalized = normalize_ws(&stripped);
    out.extend_from_slice(&normalized);
}

/// Strip single-line `#` comments from source bytes.
/// Tracks quote state to avoid stripping inside strings.
fn strip_comments(src: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(src.len());
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    let mut i = 0;
    while i < src.len() {
        let b = src[i];
        if escaped {
            result.push(b);
            escaped = false;
            i += 1;
            continue;
        }
        if b == b'\\' && (in_single_quote || in_double_quote) {
            escaped = true;
            result.push(b);
            i += 1;
            continue;
        }
        if !in_double_quote && b == b'\'' {
            in_single_quote = !in_single_quote;
            result.push(b);
            i += 1;
            continue;
        }
        if !in_single_quote && b == b'"' {
            in_double_quote = !in_double_quote;
            result.push(b);
            i += 1;
            continue;
        }
        if !in_single_quote && !in_double_quote && b == b'#' {
            while i < src.len() && src[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        result.push(b);
        i += 1;
    }
    result
}

/// Normalize whitespace: collapse runs of whitespace, inserting a single space
/// only between word characters to prevent identifier merging.
fn normalize_ws(src: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(src.len());
    let mut pending_ws = false;
    for &b in src {
        if b.is_ascii_whitespace() {
            pending_ws = true;
        } else {
            if pending_ws && !result.is_empty() {
                let prev = *result.last().unwrap();
                if is_word_byte(prev) && is_word_byte(b) {
                    result.push(b' ');
                }
            }
            result.push(b);
            pending_ws = false;
        }
    }
    result
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Check if a node contains any heredoc string nodes.
fn contains_heredoc(node: &ruby_prism::Node<'_>) -> bool {
    struct HeredocChecker {
        found: bool,
    }
    impl<'pr> Visit<'pr> for HeredocChecker {
        fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
            if let Some(opening) = node.opening_loc() {
                if opening.as_slice().starts_with(b"<<") {
                    self.found = true;
                    return;
                }
            }
            ruby_prism::visit_string_node(self, node);
        }
        fn visit_interpolated_string_node(
            &mut self,
            node: &ruby_prism::InterpolatedStringNode<'pr>,
        ) {
            if let Some(opening) = node.opening_loc() {
                if opening.as_slice().starts_with(b"<<") {
                    self.found = true;
                    return;
                }
            }
            ruby_prism::visit_interpolated_string_node(self, node);
        }
        fn visit_interpolated_x_string_node(
            &mut self,
            node: &ruby_prism::InterpolatedXStringNode<'pr>,
        ) {
            if node.opening_loc().as_slice().starts_with(b"<<") {
                self.found = true;
                return;
            }
            ruby_prism::visit_interpolated_x_string_node(self, node);
        }
        fn visit_x_string_node(&mut self, node: &ruby_prism::XStringNode<'pr>) {
            if node.opening_loc().as_slice().starts_with(b"<<") {
                self.found = true;
                return;
            }
            ruby_prism::visit_x_string_node(self, node);
        }
    }
    let mut checker = HeredocChecker { found: false };
    checker.visit(node);
    checker.found
}

/// Extract the source that RuboCop's `duplicated_expressions?` compares against
/// condition variables.  For simple writes (lvasgn, ivasgn, …) this is the
/// VALUE (RHS); for operator writes (op_asgn) it is the variable NAME (LHS).
fn assignment_child_source(source: &SourceFile, node: &ruby_prism::Node<'_>) -> Option<String> {
    // Simple writes: child_nodes.first in RuboCop = value (RHS)
    if let Some(w) = node.as_local_variable_write_node() {
        return Some(node_source(source, &w.value()));
    }
    if let Some(w) = node.as_instance_variable_write_node() {
        return Some(node_source(source, &w.value()));
    }
    if let Some(w) = node.as_class_variable_write_node() {
        return Some(node_source(source, &w.value()));
    }
    if let Some(w) = node.as_global_variable_write_node() {
        return Some(node_source(source, &w.value()));
    }
    if let Some(w) = node.as_constant_write_node() {
        return Some(node_source(source, &w.value()));
    }
    // Operator writes: child_nodes.first in RuboCop = LHS variable name
    if let Some(w) = node.as_local_variable_operator_write_node() {
        return Some(String::from_utf8_lossy(w.name().as_slice()).to_string());
    }
    if let Some(w) = node.as_instance_variable_operator_write_node() {
        return Some(String::from_utf8_lossy(w.name().as_slice()).to_string());
    }
    if let Some(w) = node.as_local_variable_or_write_node() {
        return Some(String::from_utf8_lossy(w.name().as_slice()).to_string());
    }
    if let Some(w) = node.as_local_variable_and_write_node() {
        return Some(String::from_utf8_lossy(w.name().as_slice()).to_string());
    }
    if let Some(w) = node.as_instance_variable_or_write_node() {
        return Some(String::from_utf8_lossy(w.name().as_slice()).to_string());
    }
    if let Some(w) = node.as_instance_variable_and_write_node() {
        return Some(String::from_utf8_lossy(w.name().as_slice()).to_string());
    }
    if let Some(call) = node.as_call_node() {
        if call.equal_loc().is_some() {
            return call
                .receiver()
                .map(|receiver| node_source(source, &receiver));
        }
    }
    None
}

/// Extract the source text, location, and heredoc flag for a specific statement
/// in a StatementsNode (by index).
fn stmt_info(
    source: &SourceFile,
    stmts: &ruby_prism::StatementsNode<'_>,
    index: usize,
) -> Option<StatementInfo> {
    let body: Vec<_> = stmts.body().iter().collect();
    let node = body.get(index)?;
    let loc = node.location();
    let (line, col) = source.offset_to_line_col(loc.start_offset());
    let has_heredoc = contains_heredoc(node);
    let src = node_source(source, node);
    // Build AST-based fingerprint key for comparison — this matches RuboCop's
    // AST-level comparison: ignores optional parens, comments, string escape
    // representation, and distinguishes lvar from method calls.
    let mut key = Vec::new();
    node_fp(source, source.as_bytes(), node, &mut key);
    Some(StatementInfo {
        key,
        src,
        line,
        col,
        has_heredoc,
        index_assignment_receiver: index_assignment_receiver_source(source, node),
        assignment_child_source: assignment_child_source(source, node),
        is_if_or_unless: node.as_if_node().is_some() || node.as_unless_node().is_some(),
    })
}

/// A branch in a conditional: its statements node (if present) and number of
/// statements.
struct BranchInfo<'pr> {
    stmts: Option<ruby_prism::StatementsNode<'pr>>,
    count: usize,
}

impl<'pr> BranchInfo<'pr> {
    fn from_stmts(stmts: Option<ruby_prism::StatementsNode<'pr>>) -> Self {
        let count = stmts.as_ref().map(|s| s.body().iter().count()).unwrap_or(0);
        Self { stmts, count }
    }
}

impl IdenticalConditionalBranches {
    fn single_statement<'pr>(
        stmts: Option<ruby_prism::StatementsNode<'pr>>,
    ) -> Option<ruby_prism::Node<'pr>> {
        let stmts = stmts?;
        let mut body = stmts.body().iter();
        let stmt = body.next()?;
        if body.next().is_some() {
            return None;
        }
        Some(stmt)
    }

    fn collect_if_subsequent_branches<'pr>(
        mut subsequent: Option<ruby_prism::Node<'pr>>,
        branches: &mut Vec<BranchInfo<'pr>>,
    ) -> Option<()> {
        loop {
            match subsequent {
                None => return None,
                Some(node) => {
                    if let Some(elsif_node) = node.as_if_node() {
                        branches.push(BranchInfo::from_stmts(elsif_node.statements()));
                        subsequent = elsif_node.subsequent();
                        continue;
                    }

                    if let Some(else_node) = node.as_else_node() {
                        if let Some(stmt) = Self::single_statement(else_node.statements()) {
                            if let Some(nested_if) = stmt.as_if_node() {
                                branches.push(BranchInfo::from_stmts(nested_if.statements()));
                                subsequent = nested_if.subsequent();
                                continue;
                            }

                            if let Some(nested_unless) = stmt.as_unless_node() {
                                let else_clause = nested_unless.else_clause()?;
                                branches.push(BranchInfo::from_stmts(else_clause.statements()));
                                branches.push(BranchInfo::from_stmts(nested_unless.statements()));
                                return Some(());
                            }
                        }

                        branches.push(BranchInfo::from_stmts(else_node.statements()));
                        return Some(());
                    }

                    return None;
                }
            }
        }
    }

    /// Collect all branches from an if/elsif/else chain, expanding nested elsifs.
    fn collect_if_branches<'pr>(if_node: &ruby_prism::IfNode<'pr>) -> Option<Vec<BranchInfo<'pr>>> {
        let mut branches = Vec::new();
        branches.push(BranchInfo::from_stmts(if_node.statements()));
        Self::collect_if_subsequent_branches(if_node.subsequent(), &mut branches)?;
        Some(branches)
    }

    /// Collect all branches from a case/when/else node.
    fn collect_case_branches<'pr>(
        case_node: &ruby_prism::CaseNode<'pr>,
    ) -> Option<Vec<BranchInfo<'pr>>> {
        // Must have an else clause
        let else_clause = case_node.else_clause()?;

        let mut branches = Vec::new();
        for when in case_node.conditions().iter() {
            if let Some(when_node) = when.as_when_node() {
                branches.push(BranchInfo::from_stmts(when_node.statements()));
            }
        }
        branches.push(BranchInfo::from_stmts(else_clause.statements()));
        Some(branches)
    }

    /// Collect all branches from a case/in/else node (pattern matching).
    fn collect_case_match_branches<'pr>(
        case_node: &ruby_prism::CaseMatchNode<'pr>,
    ) -> Option<Vec<BranchInfo<'pr>>> {
        // Must have an else clause
        let else_clause = case_node.else_clause()?;

        let mut branches = Vec::new();
        for in_node in case_node.conditions().iter() {
            if let Some(in_node) = in_node.as_in_node() {
                branches.push(BranchInfo::from_stmts(in_node.statements()));
            }
        }
        branches.push(BranchInfo::from_stmts(else_clause.statements()));
        Some(branches)
    }

    /// Remove duplicate diagnostics added from `start_idx` onwards (same line+col).
    fn dedup_diagnostics(diagnostics: &mut Vec<Diagnostic>, start_idx: usize) {
        let mut seen: std::collections::HashSet<(String, String, usize, usize)> = diagnostics
            .iter()
            .take(start_idx)
            .map(|diag| {
                (
                    diag.cop_name.clone(),
                    diag.message.clone(),
                    diag.location.line,
                    diag.location.column,
                )
            })
            .collect();
        let mut i = start_idx;
        while i < diagnostics.len() {
            let key = (
                diagnostics[i].cop_name.clone(),
                diagnostics[i].message.clone(),
                diagnostics[i].location.line,
                diagnostics[i].location.column,
            );
            if seen.contains(&key) {
                diagnostics.remove(i);
            } else {
                seen.insert(key);
                i += 1;
            }
        }
    }

    /// Check identical tail (last statement) across all branches.
    fn check_tails(
        &self,
        source: &SourceFile,
        branches: &[BranchInfo<'_>],
        condition_node: Option<&ruby_prism::Node<'_>>,
        is_last_child_of_parent: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // All branches must have at least one statement
        if branches.iter().any(|b| b.count == 0) {
            return;
        }

        // Get tail (last statement) from each branch
        let mut tails: Vec<StatementInfo> = Vec::new();
        for branch in branches {
            let stmts = match &branch.stmts {
                Some(s) => s,
                None => return,
            };
            match stmt_info(source, stmts, branch.count - 1) {
                Some(info) => tails.push(info),
                None => return,
            }
        }

        // Skip if any tail contains a heredoc
        if tails.iter().any(|tail| tail.has_heredoc) {
            return;
        }

        // All tails must be identical
        let first_src = &tails[0].src;
        if first_src.is_empty() {
            return;
        }
        let first_key = &tails[0].key;
        if !tails.iter().all(|tail| tail.key == *first_key) {
            return;
        }

        if is_last_child_of_parent
            && branches.iter().all(|branch| branch.count == 1)
            && tails[0].is_if_or_unless
        {
            return;
        }

        if let Some(condition) = condition_node {
            if let Some(receiver) = tails[0].index_assignment_receiver.as_deref() {
                if condition_contains_variable_source(source, condition, receiver) {
                    return;
                }
            }

            // RuboCop's `duplicated_expressions?` suppression: if the tail is
            // an assignment and the value (or LHS for operator writes) matches
            // a variable in the condition, skip.
            if let Some(child_src) = &tails[0].assignment_child_source {
                if condition_contains_variable_source(source, condition, child_src) {
                    return;
                }
            }
        }

        // Report offense on every branch's tail (RuboCop flags all of them)
        let msg = format!("Move `{}` out of the conditional.", tails[0].src);
        for tail in &tails {
            diagnostics.push(self.diagnostic(source, tail.line, tail.col, msg.clone()));
        }
    }

    /// Check identical head (first statement) across all branches.
    fn check_heads(
        &self,
        source: &SourceFile,
        branches: &[BranchInfo<'_>],
        condition_node: Option<&ruby_prism::Node<'_>>,
        is_last_child_of_parent: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // All branches must have at least one statement
        if branches.iter().any(|b| b.count == 0) {
            return;
        }

        // Suppression: if this is the last child of the parent and any branch
        // has only a single statement, skip head check (can't extract without
        // changing return value semantics).
        if is_last_child_of_parent && branches.iter().any(|b| b.count == 1) {
            return;
        }

        // Get head (first statement) from each branch
        let mut heads: Vec<StatementInfo> = Vec::new();
        for branch in branches {
            let stmts = match &branch.stmts {
                Some(s) => s,
                None => return,
            };
            match stmt_info(source, stmts, 0) {
                Some(info) => heads.push(info),
                None => return,
            }
        }

        // Skip if any head contains a heredoc
        if heads.iter().any(|head| head.has_heredoc) {
            return;
        }

        // All heads must be identical
        let first_src = &heads[0].src;
        if first_src.is_empty() {
            return;
        }
        let first_key = &heads[0].key;
        if !heads.iter().all(|head| head.key == *first_key) {
            return;
        }

        // Suppression: if the head is an assignment and the LHS matches the
        // condition variable, skip (moving it before the conditional would
        // change semantics).
        if let Some(cond) = condition_node {
            if is_assignment_to_condition(source, first_src, cond) {
                return;
            }

            // RuboCop's `duplicated_expressions?` suppression: if the head is
            // an assignment and the value (or LHS for operator writes) matches
            // a variable in the condition, skip.
            if let Some(child_src) = &heads[0].assignment_child_source {
                if condition_contains_variable_source(source, cond, child_src) {
                    return;
                }
            }
        }

        // Report offense on every branch's head (RuboCop flags all of them)
        let msg = format!("Move `{}` out of the conditional.", heads[0].src);
        for head in &heads {
            diagnostics.push(self.diagnostic(source, head.line, head.col, msg.clone()));
        }
    }
}

fn index_assignment_receiver_source(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
) -> Option<String> {
    let call = node.as_call_node()?;
    if call.name().as_slice() != b"[]=" {
        return None;
    }

    let receiver = call.receiver()?;
    Some(node_source(source, &receiver))
}

fn condition_contains_variable_source(
    source: &SourceFile,
    condition: &ruby_prism::Node<'_>,
    needle: &str,
) -> bool {
    struct DirectVariableFinder<'a> {
        source: &'a SourceFile,
        needle: &'a str,
        branch_depth: usize,
        found: bool,
    }

    impl<'a, 'pr> Visit<'pr> for DirectVariableFinder<'a> {
        fn visit_branch_node_enter(&mut self, _node: ruby_prism::Node<'pr>) {
            self.branch_depth += 1;
        }

        fn visit_branch_node_leave(&mut self) {
            self.branch_depth -= 1;
        }

        fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
            if self.branch_depth != 1 {
                return;
            }

            let matches = node
                .as_local_variable_read_node()
                .map(|n| node_source(self.source, &n.as_node()) == self.needle)
                .or_else(|| {
                    node.as_instance_variable_read_node()
                        .map(|n| node_source(self.source, &n.as_node()) == self.needle)
                })
                .or_else(|| {
                    node.as_class_variable_read_node()
                        .map(|n| node_source(self.source, &n.as_node()) == self.needle)
                })
                .or_else(|| {
                    node.as_global_variable_read_node()
                        .map(|n| node_source(self.source, &n.as_node()) == self.needle)
                })
                .unwrap_or(false);

            if matches {
                self.found = true;
            }
        }
    }

    let mut finder = DirectVariableFinder {
        source,
        needle,
        branch_depth: 0,
        found: false,
    };
    finder.visit(condition);
    finder.found
}

/// Check if the head expression is an assignment whose LHS matches the
/// condition variable (or its receiver). RuboCop suppresses these to avoid
/// changing semantics.
fn is_assignment_to_condition(
    source: &SourceFile,
    head_src: &str,
    condition: &ruby_prism::Node<'_>,
) -> bool {
    // Check for `x = ...` style assignments
    // The head source might be `x = value`, `@x = value`, `x += 1`, etc.
    // Extract the LHS (before ` =`, ` +=`, ` ||=`, etc.)
    let lhs = if let Some(pos) = head_src.find(" =") {
        head_src[..pos].trim()
    } else if let Some(pos) = head_src.find(" +=") {
        head_src[..pos].trim()
    } else if let Some(pos) = head_src.find(" -=") {
        head_src[..pos].trim()
    } else if let Some(pos) = head_src.find(" ||=") {
        head_src[..pos].trim()
    } else if let Some(pos) = head_src.find(" &&=") {
        head_src[..pos].trim()
    } else {
        return false;
    };

    // Get condition source
    let cond_loc = condition.location();
    let cond_bytes = &source.as_bytes()[cond_loc.start_offset()..cond_loc.end_offset()];
    let cond_src = String::from_utf8_lossy(cond_bytes);
    let cond_src = cond_src.trim();

    // Direct match: `if x` and `x = ...`
    if lhs == cond_src {
        return true;
    }

    // Receiver match: `if x.something` or `if x&.something` and `x = ...`
    // Extract receiver from condition (before `.` or `&.`)
    if let Some(call_node) = condition.as_call_node() {
        if let Some(receiver) = call_node.receiver() {
            let recv_loc = receiver.location();
            let recv_bytes = &source.as_bytes()[recv_loc.start_offset()..recv_loc.end_offset()];
            let recv_src = String::from_utf8_lossy(recv_bytes);
            let recv_src = recv_src.trim();
            if lhs == recv_src {
                return true;
            }
        }
    }

    // Check for index-style `h[:key]` in condition and head
    // e.g., `if h[:key]` and `h[:key] = foo`
    if let Some(call_node) = condition.as_call_node() {
        if call_node.name().as_slice() == b"[]" {
            // The condition is an indexing operation like h[:key]
            if lhs == cond_src || head_src.starts_with(&format!("{cond_src} ")) {
                return true;
            }
        }
    }

    false
}

/// Check if the conditional `node` is the last expression in its parent scope
/// (method body, block, etc.). This is used to suppress head checks for
/// single-child branches, matching RuboCop's `last_child_of_parent?` behavior.
fn is_last_child_of_parent(
    node: &ruby_prism::Node<'_>,
    parse_result: &ruby_prism::ParseResult<'_>,
) -> bool {
    // Walk the AST to find the parent of our node.
    // We check if the node's start offset matches as the last statement in any
    // parent StatementsNode. This is a heuristic that works for method bodies,
    // blocks, etc.
    let target_offset = node.location().start_offset();

    struct ParentFinder {
        target_offset: usize,
        is_last: bool,
    }

    impl ParentFinder {
        /// Check if a value node matches the target (i.e. the conditional is
        /// the value of an assignment like `y = if ...`).
        fn check_value(&mut self, value: &ruby_prism::Node<'_>) {
            if value.location().start_offset() == self.target_offset {
                self.is_last = true;
            }
        }
    }

    impl<'pr> Visit<'pr> for ParentFinder {
        fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
            let body: Vec<_> = node.body().iter().collect();
            if let Some(last) = body.last() {
                if last.location().start_offset() == self.target_offset {
                    self.is_last = true;
                }
            }
            ruby_prism::visit_statements_node(self, node);
        }

        // Assignment nodes: the conditional is the "last child" when it's the
        // value of an assignment (e.g. `y = if ...`).
        fn visit_local_variable_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableWriteNode<'pr>,
        ) {
            self.check_value(&node.value());
            ruby_prism::visit_local_variable_write_node(self, node);
        }
        fn visit_instance_variable_write_node(
            &mut self,
            node: &ruby_prism::InstanceVariableWriteNode<'pr>,
        ) {
            self.check_value(&node.value());
            ruby_prism::visit_instance_variable_write_node(self, node);
        }
        fn visit_class_variable_write_node(
            &mut self,
            node: &ruby_prism::ClassVariableWriteNode<'pr>,
        ) {
            self.check_value(&node.value());
            ruby_prism::visit_class_variable_write_node(self, node);
        }
        fn visit_global_variable_write_node(
            &mut self,
            node: &ruby_prism::GlobalVariableWriteNode<'pr>,
        ) {
            self.check_value(&node.value());
            ruby_prism::visit_global_variable_write_node(self, node);
        }
        fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
            self.check_value(&node.value());
            ruby_prism::visit_constant_write_node(self, node);
        }
    }

    let mut finder = ParentFinder {
        target_offset,
        is_last: false,
    };
    finder.visit(&parse_result.node());
    finder.is_last
}

impl Cop for IdenticalConditionalBranches {
    fn name(&self) -> &'static str {
        "Style/IdenticalConditionalBranches"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, CASE_NODE, CASE_MATCH_NODE, UNLESS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if let Some(if_node) = node.as_if_node() {
            // Skip elsif nodes — we process the full chain from the top-level if
            if let Some(kw_loc) = if_node.if_keyword_loc() {
                if kw_loc.as_slice() == b"elsif" {
                    return;
                }
            } else {
                // No keyword loc — this is a ternary or modifier if
                // RuboCop still flags ternaries, but we handle them via the
                // same branch expansion
            }

            let branches = match Self::collect_if_branches(&if_node) {
                Some(b) => b,
                None => return, // no else clause
            };

            let pre_len = diagnostics.len();
            let condition = if_node.predicate();
            let last_child = is_last_child_of_parent(node, parse_result);

            // Check tails (last statement in each branch)
            self.check_tails(source, &branches, Some(&condition), last_child, diagnostics);

            // Check heads (first statement in each branch)
            self.check_heads(source, &branches, Some(&condition), last_child, diagnostics);

            // Deduplicate: when both head and tail fire on single-stmt branches
            Self::dedup_diagnostics(diagnostics, pre_len);
        } else if let Some(case_node) = node.as_case_node() {
            let branches = match Self::collect_case_branches(&case_node) {
                Some(b) => b,
                None => return,
            };

            let pre_len = diagnostics.len();
            let condition = case_node.predicate();
            let last_child = is_last_child_of_parent(node, parse_result);

            self.check_tails(
                source,
                &branches,
                condition.as_ref(),
                last_child,
                diagnostics,
            );

            self.check_heads(
                source,
                &branches,
                condition.as_ref(),
                last_child,
                diagnostics,
            );

            Self::dedup_diagnostics(diagnostics, pre_len);
        } else if let Some(case_match_node) = node.as_case_match_node() {
            let branches = match Self::collect_case_match_branches(&case_match_node) {
                Some(b) => b,
                None => return,
            };

            let pre_len = diagnostics.len();
            let condition = case_match_node.predicate();
            let last_child = is_last_child_of_parent(node, parse_result);

            self.check_tails(
                source,
                &branches,
                condition.as_ref(),
                last_child,
                diagnostics,
            );

            self.check_heads(
                source,
                &branches,
                condition.as_ref(),
                last_child,
                diagnostics,
            );

            Self::dedup_diagnostics(diagnostics, pre_len);
        } else if let Some(unless_node) = node.as_unless_node() {
            // unless/else — must have an else clause for comparison
            let else_clause = match unless_node.else_clause() {
                Some(e) => e,
                None => return,
            };

            let branches = vec![
                BranchInfo::from_stmts(unless_node.statements()),
                BranchInfo::from_stmts(else_clause.statements()),
            ];

            let pre_len = diagnostics.len();
            let condition = unless_node.predicate();
            let last_child = is_last_child_of_parent(node, parse_result);

            self.check_tails(source, &branches, Some(&condition), last_child, diagnostics);

            self.check_heads(source, &branches, Some(&condition), last_child, diagnostics);

            Self::dedup_diagnostics(diagnostics, pre_len);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        IdenticalConditionalBranches,
        "cops/style/identical_conditional_branches"
    );

    #[test]
    fn dedup_diagnostics_only_removes_exact_duplicates_from_new_slice() {
        let mut diagnostics = vec![
            Diagnostic {
                path: "test.rb".to_string(),
                location: crate::diagnostic::Location {
                    line: 10,
                    column: 4,
                },
                severity: crate::diagnostic::Severity::Convention,
                cop_name: "Lint/OtherCop".to_string(),
                message: "Move `x` out of the conditional.".to_string(),
                corrected: false,
            },
            Diagnostic {
                path: "test.rb".to_string(),
                location: crate::diagnostic::Location {
                    line: 10,
                    column: 4,
                },
                severity: crate::diagnostic::Severity::Convention,
                cop_name: "Style/IdenticalConditionalBranches".to_string(),
                message: "Move `x` out of the conditional.".to_string(),
                corrected: false,
            },
            Diagnostic {
                path: "test.rb".to_string(),
                location: crate::diagnostic::Location {
                    line: 10,
                    column: 4,
                },
                severity: crate::diagnostic::Severity::Convention,
                cop_name: "Style/IdenticalConditionalBranches".to_string(),
                message: "Move `x` out of the conditional.".to_string(),
                corrected: false,
            },
            Diagnostic {
                path: "test.rb".to_string(),
                location: crate::diagnostic::Location {
                    line: 10,
                    column: 4,
                },
                severity: crate::diagnostic::Severity::Convention,
                cop_name: "Style/IdenticalConditionalBranches".to_string(),
                message: "Move `y` out of the conditional.".to_string(),
                corrected: false,
            },
        ];

        IdenticalConditionalBranches::dedup_diagnostics(&mut diagnostics, 2);

        assert_eq!(diagnostics.len(), 3);
        assert_eq!(diagnostics[0].cop_name, "Lint/OtherCop");
        assert_eq!(
            diagnostics[1].cop_name,
            "Style/IdenticalConditionalBranches"
        );
        assert_eq!(diagnostics[1].message, "Move `x` out of the conditional.");
        assert_eq!(diagnostics[2].message, "Move `y` out of the conditional.");
    }
}
