use std::collections::HashSet;

use ruby_prism::Visit;

use crate::cop::shared::node_type::{BEGIN_NODE, CASE_MATCH_NODE, CASE_NODE, IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks that there are no repeated bodies within `if/unless`, `case-when`,
/// `case-in` and `rescue` constructs.
///
/// ## Root cause analysis (528 FN, 0 FP at 79.1% match rate)
///
/// The original implementation was missing several branch types and config options:
///
/// 1. **rescue branches** - `begin/rescue` constructs were completely unhandled.
///    RuboCop's `on_rescue` checks rescue clause bodies and the else clause.
///    Fixed by handling `BEGIN_NODE` and walking `rescue_clause()` / `subsequent()` chain.
///
/// 2. **case-in (pattern matching)** - `CaseMatchNode` / `InNode` were not handled.
///    Fixed by adding `CASE_MATCH_NODE` to interested types and `check_case_match_branches`.
///
/// 3. **unless** - `UnlessNode` is separate from `IfNode` in Prism. Was not handled.
///    Fixed by adding `UNLESS_NODE` and treating it like a 2-branch if/else.
///
/// 4. **ternary** - Ternary operators parse as `IfNode` with `if_keyword_loc() == None`.
///    Were already handled by the `IfNode` path, but the offense location was wrong.
///    RuboCop reports on the false-branch expression for ternaries, and on the
///    `else` keyword for else-branch duplicates.
///
/// 5. **else branch in case/when** - `check_case_branches` didn't include the
///    `else_clause` body, so `case x; when a; foo; else; foo; end` was missed.
///    Fixed by including the else clause in the branch set.
///
/// 6. **Config options** - `IgnoreLiteralBranches`, `IgnoreConstantBranches`, and
///    `IgnoreDuplicateElseBranch` were read but never applied. All three are now wired up.
///
/// 7. **Offense location** - RuboCop reports on the `else` keyword for else-branch
///    duplicates, and on the parent clause node (elsif/when/rescue/in) for others.
///    The ternary case reports on the false-branch expression itself. Fixed to match.
///
/// ## Follow-up (2026-03-10)
///
/// Remaining FNs include branches whose source differs but whose literal values
/// are equivalent. One real corpus case used two string literals with different
/// escape spellings (`"\u2028"` vs `"\342\200\250"`) that both decode to the
/// same Ruby string, which RuboCop treats as duplicate branch bodies.
///
/// Fix: keep the existing source-based comparison for general branches, but
/// canonicalize single string-literal branches by their unescaped bytes so those
/// escape-equivalent forms compare equal. Also treat `KeywordHashNode` as a
/// literal branch for the `IgnoreLiteralBranches` config path to satisfy the
/// Prism hash/keyword-hash split.
///
/// ## Follow-up (2026-03-19) — backtick string FP
///
/// FP=2: `XStringNode` and `InterpolatedXStringNode` (backtick strings like
/// `` `cmd #{var}` ``) were not handled by `LiteralSpanFinder`. The `#` in
/// `#{...}` interpolation was not protected as a literal span, so
/// `strip_comments()` ate everything from `#` to end-of-line, making branches
/// with different interpolated variables appear identical. Fixed by adding
/// `visit_x_string_node` and `visit_interpolated_x_string_node` handlers.
///
/// ## Follow-up (2026-03-18) — whitespace normalization
///
/// Remaining 15 FN were caused by whitespace-only differences between branches.
/// Fix: normalize whitespace in source comparison keys.
///
/// ## Follow-up (2026-03-18) — FP from literal content + FN from comments
///
/// FP=29, FN=9 at 98.5% match rate.
///
/// **FP root cause:** whitespace normalization was applied to the entire source
/// range including string/regex/heredoc content. Branches with strings differing
/// only by whitespace inside the literal (e.g., `"foo => bar"` vs `"foo=>bar"`,
/// or heredocs with different indentation) were falsely flagged as duplicates.
///
/// **FN root cause:** Prism's `StatementsNode` source range includes comments
/// between statements. Branches with identical code but different comments
/// (e.g., different TODO comments between `render` and `return`) compared
/// differently because the raw source bytes included the comment text.
///
/// **Fix:** Two changes to `stmts_source`:
/// 1. Iterate individual body nodes (not the full StatementsNode range) to skip
///    inter-statement comments. Prism's `body()` returns only statement nodes.
/// 2. Use `LiteralSpanFinder` visitor to identify string/regex/symbol literal
///    spans within each node. Whitespace is normalized only OUTSIDE literal
///    spans; literal content is fingerprinted verbatim (strings use unescaped
///    bytes, regexes use unescaped pattern, interpolated strings use parts).
///
/// ## Follow-up (2026-03-18) — comments inside nodes, optional parens, -0.0
///
/// FP=0, FN=4 at 99.8% match rate. Three distinct root causes:
///
/// 1. **Comments inside a single node's source range** — previous comment
///    stripping only worked between statements (iterating body nodes). Comments
///    within a single node (e.g., trailing `# comment` on a hash argument, or
///    different `# comment` blocks inside an `if/else` sub-expression) were
///    included in the fingerprint. Fix: `strip_comments()` applied to all
///    non-literal source regions before whitespace normalization.
///
/// 2. **Optional method call parentheses** — `foo(x)` and `foo x` produce the
///    same AST but different source text. Fix: `call_node_fingerprint()` builds
///    a structural fingerprint from method name + receiver + arguments,
///    independent of opening/closing paren presence.
///
/// 3. **`-0.0` vs `0.0`** — In Ruby, `0.0 == -0.0` and they hash equally,
///    so RuboCop's Set-based comparison considers them duplicate branches.
///    Prism may parse `-0.0` as `FloatNode(-0.0)` or `CallNode(-@, FloatNode)`.
///    Fix: `is_negated_zero_float()` checks for `-@` on zero, and the FloatNode
///    path checks `val == 0.0` (which matches both `0.0` and `-0.0` per IEEE 754).
///
/// ## Follow-up (2026-03-28) — nested structural fingerprints
///
/// Remaining FN=4 at 99.9% were still caused by source-based fallback on nodes
/// that wrap or contain AST-equivalent subexpressions:
///
/// 1. `ReturnNode` wrappers around calls with and without optional parens
///    (`return user_input?(node.value)` vs `return user_input? node.value`).
/// 2. `BlockNode` delimiters under nested calls (`do .. end` vs `{ ... }`).
/// 3. Prism's `HashNode` vs `KeywordHashNode` split for equivalent call args
///    (`object.call(1, {a: 2})` vs `object.call(1, a: 2)`), while preserving
///    `**{a: 2}` as distinct via `AssocSplatNode`.
///
/// Fix: keep source-based comparison as the default, but add structural
/// fingerprints for `ReturnNode`, block nodes, local/instance variable writes,
/// and hash-like nodes so nested AST-equivalent bodies compare equal without
/// broad comment/whitespace suppression across unrelated node kinds.
///
/// ## Follow-up (2026-04-02) — `case in` pattern locals inside arrays
///
/// FP=2 remained in `case in` branches where array bodies had identical source
/// text, but Prism resolved a bare identifier as a zero-arg method call in one
/// branch and as a `LocalVariableReadNode` introduced by the pattern in another.
/// Example: `[status, headers, [body]]` under `in String => body` versus
/// `in [Integer => status, String => body]`.
///
/// Source-based fallback on `ArrayNode` collapsed those branches even though
/// their nested ASTs differ. Fix: fingerprint `ArrayNode` structurally by
/// element so `CallNode(status)` and `LocalVariableReadNode(status)` stay
/// distinct without suppressing legitimate duplicate array branches.
pub struct DuplicateBranch;

impl Cop for DuplicateBranch {
    fn name(&self) -> &'static str {
        "Lint/DuplicateBranch"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE, CASE_NODE, CASE_MATCH_NODE, BEGIN_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let ignore_literal = config.get_bool("IgnoreLiteralBranches", false);
        let ignore_constant = config.get_bool("IgnoreConstantBranches", false);
        let ignore_dup_else = config.get_bool("IgnoreDuplicateElseBranch", false);

        if let Some(if_node) = node.as_if_node() {
            check_if_branches(
                self,
                source,
                &if_node,
                ignore_literal,
                ignore_constant,
                ignore_dup_else,
                diagnostics,
            );
            return;
        }

        if let Some(unless_node) = node.as_unless_node() {
            check_unless_branches(
                self,
                source,
                &unless_node,
                ignore_literal,
                ignore_constant,
                diagnostics,
            );
            return;
        }

        if let Some(case_node) = node.as_case_node() {
            check_case_branches(
                self,
                source,
                &case_node,
                ignore_literal,
                ignore_constant,
                ignore_dup_else,
                diagnostics,
            );
            return;
        }

        if let Some(case_match_node) = node.as_case_match_node() {
            check_case_match_branches(
                self,
                source,
                &case_match_node,
                ignore_literal,
                ignore_constant,
                ignore_dup_else,
                diagnostics,
            );
            return;
        }

        if let Some(begin_node) = node.as_begin_node() {
            if begin_node.rescue_clause().is_some() {
                check_rescue_branches(
                    self,
                    source,
                    &begin_node,
                    ignore_literal,
                    ignore_constant,
                    ignore_dup_else,
                    diagnostics,
                );
            }
        }
    }
}

/// Extract a comparison key for branch body.
///
/// RuboCop compares branches by AST structure, not raw source text. Two branches
/// with different whitespace, comments, or string escape spellings are duplicates
/// if they produce the same AST.
///
/// We approximate this by building a fingerprint from individual statement nodes:
/// 1. Iterate body nodes individually (skips inter-statement comments, since
///    Prism's `body()` only returns statement nodes, not comment nodes).
/// 2. For single string-literal nodes, use unescaped byte content so
///    equivalent escape spellings compare equal.
/// 3. For other nodes, extend ranges for heredocs and normalize whitespace.
fn stmts_source(source: &SourceFile, stmts: &Option<ruby_prism::StatementsNode<'_>>) -> Vec<u8> {
    stmts_fingerprint(source.as_bytes(), stmts)
}

fn stmts_fingerprint(bytes: &[u8], stmts: &Option<ruby_prism::StatementsNode<'_>>) -> Vec<u8> {
    match stmts {
        Some(s) => {
            let body = s.body();
            if body.is_empty() {
                return Vec::new();
            }

            // Single string literal: use unescaped content for escape-equivalence
            if body.len() == 1 {
                if let Some(node) = body.iter().next() {
                    if let Some(string) = node.as_string_node() {
                        let mut fingerprint = b"string:".to_vec();
                        fingerprint.extend_from_slice(string.unescaped());
                        return fingerprint;
                    }
                }
            }

            // Build fingerprint by concatenating per-node fingerprints.
            // Iterating body nodes individually skips inter-statement comments.
            // node_fingerprint preserves string/regex content verbatim while
            // normalizing whitespace outside literals.
            let mut fingerprint = Vec::new();

            for (i, node) in body.iter().enumerate() {
                if i > 0 {
                    fingerprint.push(b'\x00'); // separator between statements
                }
                node_fingerprint(bytes, &node, &mut fingerprint);
            }

            fingerprint
        }
        None => Vec::new(),
    }
}

/// Strip single-line comments (`#` to end-of-line) from source bytes.
/// This is applied to non-literal source regions before whitespace normalization
/// so that branches differing only by comments are treated as duplicates,
/// matching RuboCop's AST-based comparison which ignores comments.
fn strip_comments(src: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() {
        if src[i] == b'#' {
            // Skip from '#' to end of line
            while i < src.len() && src[i] != b'\n' {
                i += 1;
            }
        } else {
            result.push(src[i]);
            i += 1;
        }
    }
    result
}

/// Strip all ASCII whitespace, inserting a single space only between two
/// adjacent word characters (alphanumeric or underscore) to prevent identifier
/// merging. This matches RuboCop's AST-based comparison which ignores all
/// formatting whitespace.
fn normalize_whitespace(src: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(src.len());
    let mut pending_ws = false;
    for &b in src {
        if b.is_ascii_whitespace() {
            pending_ws = true;
        } else {
            // Insert a space only if both the previous and current characters
            // are word characters (to avoid merging `return []` into `return[]`)
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

/// Strip comments, then normalize whitespace for non-literal source.
fn normalize_source(src: &[u8]) -> Vec<u8> {
    normalize_whitespace(&strip_comments(src))
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

struct MaxExtentFinder {
    max_end: usize,
}

impl<'pr> Visit<'pr> for MaxExtentFinder {
    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        if let Some(close) = node.closing_loc() {
            let end = close.end_offset();
            if end > self.max_end {
                self.max_end = end;
            }
        }
        ruby_prism::visit_interpolated_string_node(self, node);
    }

    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        if let Some(close) = node.closing_loc() {
            let end = close.end_offset();
            if end > self.max_end {
                self.max_end = end;
            }
        }
        ruby_prism::visit_string_node(self, node);
    }
}

/// Build a fingerprint for a single statement node.
///
/// Finds all string/regex/symbol literal spans within the node using a visitor,
/// then strips comments and normalizes whitespace only OUTSIDE those spans.
/// Literal content is preserved exactly, preventing false positives when
/// whitespace inside strings or regexes differs between branches.
///
/// Special cases:
/// - CallNode: structural fingerprint from method name + args, independent of
///   optional parentheses (`foo(x)` and `foo x` produce the same fingerprint)
/// - Unary minus on zero float: `-0.0` fingerprints the same as `0.0` because
///   Ruby's `0.0 == -0.0` and they hash equally in RuboCop's AST comparison
fn node_fingerprint(bytes: &[u8], node: &ruby_prism::Node<'_>, out: &mut Vec<u8>) {
    // Fast path: plain string literal — use unescaped content
    if let Some(string) = node.as_string_node() {
        out.extend_from_slice(b"S:");
        out.extend_from_slice(string.unescaped());
        return;
    }

    // Fast path: symbol literal — use unescaped name
    if let Some(sym) = node.as_symbol_node() {
        out.extend_from_slice(b"Y:");
        out.extend_from_slice(sym.unescaped());
        return;
    }

    // Unary minus on zero float: `-0.0` == `0.0` in Ruby (and hash equally),
    // so RuboCop considers them duplicate branch bodies. Must be checked before
    // the general CallNode path since `-0.0` parses as CallNode(-@, FloatNode).
    if is_negated_zero_float(node) {
        out.extend_from_slice(b"F:0.0");
        return;
    }

    // CallNode: structural fingerprint independent of optional parentheses.
    // `foo(x, y: z)` and `foo x, y: z` produce the same AST and should
    // fingerprint equally.
    if let Some(call) = node.as_call_node() {
        call_node_fingerprint(bytes, &call, out);
        return;
    }

    if let Some(ret) = node.as_return_node() {
        return_node_fingerprint(bytes, &ret, out);
        return;
    }

    if let Some(block) = node.as_block_node() {
        block_node_fingerprint(bytes, &block, out);
        return;
    }

    if let Some(write) = node.as_local_variable_write_node() {
        variable_write_fingerprint(bytes, b"LVW:", write.name().as_slice(), &write.value(), out);
        return;
    }

    if let Some(write) = node.as_instance_variable_write_node() {
        variable_write_fingerprint(bytes, b"IVW:", write.name().as_slice(), &write.value(), out);
        return;
    }

    if let Some(array) = node.as_array_node() {
        array_fingerprint(bytes, &array, out);
        return;
    }

    if let Some(hash) = node.as_hash_node() {
        hash_fingerprint(bytes, hash.elements().iter(), out);
        return;
    }

    if let Some(hash) = node.as_keyword_hash_node() {
        hash_fingerprint(bytes, hash.elements().iter(), out);
        return;
    }

    if let Some(assoc) = node.as_assoc_node() {
        assoc_fingerprint(bytes, &assoc, out);
        return;
    }

    if let Some(assoc_splat) = node.as_assoc_splat_node() {
        assoc_splat_fingerprint(bytes, &assoc_splat, out);
        return;
    }

    // Float literal: normalize so 0.0 fingerprints consistently
    if let Some(float_node) = node.as_float_node() {
        let val = float_node.value();
        if val == 0.0 {
            out.extend_from_slice(b"F:0.0");
            return;
        }
    }

    source_fingerprint(bytes, node, out);
}

fn return_node_fingerprint(bytes: &[u8], ret: &ruby_prism::ReturnNode<'_>, out: &mut Vec<u8>) {
    out.extend_from_slice(b"RET(");
    if let Some(args) = ret.arguments() {
        for (i, arg) in args.arguments().iter().enumerate() {
            if i > 0 {
                out.push(b',');
            }
            node_fingerprint(bytes, &arg, out);
        }
    }
    out.push(b')');
}

fn block_node_fingerprint(bytes: &[u8], block: &ruby_prism::BlockNode<'_>, out: &mut Vec<u8>) {
    out.extend_from_slice(b"BLK(");
    match block.parameters() {
        Some(params) => node_fingerprint(bytes, &params, out),
        None => out.push(b'-'),
    }
    out.push(b'|');
    if let Some(body) = block.body() {
        if let Some(stmts) = body.as_statements_node() {
            let mut body_fp = stmts_fingerprint(bytes, &Some(stmts));
            out.append(&mut body_fp);
        } else {
            node_fingerprint(bytes, &body, out);
        }
    }
    out.push(b')');
}

fn variable_write_fingerprint(
    bytes: &[u8],
    prefix: &[u8],
    name: &[u8],
    value: &ruby_prism::Node<'_>,
    out: &mut Vec<u8>,
) {
    out.extend_from_slice(prefix);
    out.extend_from_slice(name);
    out.push(b'=');
    node_fingerprint(bytes, value, out);
}

fn array_fingerprint(bytes: &[u8], array: &ruby_prism::ArrayNode<'_>, out: &mut Vec<u8>) {
    out.extend_from_slice(b"A[");
    for (i, element) in array.elements().iter().enumerate() {
        if i > 0 {
            out.push(b',');
        }
        node_fingerprint(bytes, &element, out);
    }
    out.push(b']');
}

fn hash_fingerprint<'pr, I>(bytes: &[u8], elements: I, out: &mut Vec<u8>)
where
    I: Iterator<Item = ruby_prism::Node<'pr>>,
{
    out.extend_from_slice(b"H{");
    for (i, element) in elements.enumerate() {
        if i > 0 {
            out.push(b',');
        }
        node_fingerprint(bytes, &element, out);
    }
    out.push(b'}');
}

fn assoc_fingerprint(bytes: &[u8], assoc: &ruby_prism::AssocNode<'_>, out: &mut Vec<u8>) {
    out.extend_from_slice(b"P(");
    let key = assoc.key();
    node_fingerprint(bytes, &key, out);
    out.extend_from_slice(b"=>");
    let value = assoc.value();
    node_fingerprint(bytes, &value, out);
    out.push(b')');
}

fn assoc_splat_fingerprint(
    bytes: &[u8],
    assoc_splat: &ruby_prism::AssocSplatNode<'_>,
    out: &mut Vec<u8>,
) {
    out.extend_from_slice(b"AS:");
    if let Some(value) = assoc_splat.value() {
        node_fingerprint(bytes, &value, out);
    } else {
        out.push(b'-');
    }
}

/// Build a source-based fingerprint for a node: strips comments and normalizes
/// whitespace outside literal spans.
fn source_fingerprint(bytes: &[u8], node: &ruby_prism::Node<'_>, out: &mut Vec<u8>) {
    let loc = node.location();
    let node_start = loc.start_offset();
    let mut node_end = loc.end_offset();

    // Extend for heredocs within this node
    let mut extent_finder = MaxExtentFinder { max_end: node_end };
    extent_finder.visit(node);
    node_end = extent_finder.max_end;

    let end = node_end.min(bytes.len());
    if node_start >= end {
        let mut ws_out = normalize_source(loc.as_slice());
        out.append(&mut ws_out);
        return;
    }

    // Collect literal spans (strings, regexes, symbols, heredocs) to preserve verbatim
    let mut lit_finder = LiteralSpanFinder { spans: Vec::new() };
    lit_finder.visit(node);
    lit_finder.spans.sort_by_key(|s| s.0);

    if lit_finder.spans.is_empty() {
        // No literals — just normalize the whole source
        let mut ws_out = normalize_source(&bytes[node_start..end]);
        out.append(&mut ws_out);
        return;
    }

    let raw = &bytes[node_start..end];
    let base = node_start;
    let mut cursor = 0usize; // offset relative to base

    for (span_start, span_end, content) in &lit_finder.spans {
        let rel_start = span_start.saturating_sub(base);
        let rel_end = span_end.saturating_sub(base).min(raw.len());

        // Strip comments + normalize whitespace in the gap before this literal
        if rel_start > cursor {
            let mut ws_out = normalize_source(&raw[cursor..rel_start]);
            out.append(&mut ws_out);
        }

        // Emit literal content verbatim
        out.extend_from_slice(content);

        if rel_end > cursor {
            cursor = rel_end;
        }
    }

    // Normalize any remaining source after the last literal
    if cursor < raw.len() {
        let mut ws_out = normalize_source(&raw[cursor..]);
        out.append(&mut ws_out);
    }
}

/// Build a structural fingerprint for a CallNode, independent of optional parens.
/// `foo(x, y: z)` and `foo x, y: z` produce the same fingerprint.
fn call_node_fingerprint(bytes: &[u8], call: &ruby_prism::CallNode<'_>, out: &mut Vec<u8>) {
    out.extend_from_slice(b"C:");
    // Receiver
    if let Some(recv) = call.receiver() {
        node_fingerprint(bytes, &recv, out);
        if let Some(op) = call.call_operator_loc() {
            out.extend_from_slice(op.as_slice());
        } else {
            out.push(b'.');
        }
    }
    // Method name
    out.extend_from_slice(call.name().as_slice());
    // Arguments
    out.push(b'(');
    if let Some(args) = call.arguments() {
        for (i, arg) in args.arguments().iter().enumerate() {
            if i > 0 {
                out.push(b',');
            }
            node_fingerprint(bytes, &arg, out);
        }
    }
    out.push(b')');
    // Block
    if let Some(block) = call.block() {
        out.push(b'{');
        node_fingerprint(bytes, &block, out);
        out.push(b'}');
    }
}

/// Check if a node is a unary minus applied to a zero float literal.
/// In Ruby's AST, `-0.0` is equivalent to `0.0` (they hash equally).
fn is_negated_zero_float(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"-@"
            && call.receiver().is_some()
            && call.arguments().is_none()
        {
            if let Some(recv) = call.receiver() {
                if let Some(float_node) = recv.as_float_node() {
                    return float_node.value() == 0.0;
                }
                if let Some(int_node) = recv.as_integer_node() {
                    let val = int_node.value();
                    let (negative, digits) = val.to_u32_digits();
                    return !negative && digits == [0];
                }
            }
        }
    }
    false
}

/// Collects byte spans of string/regex/symbol literals within a node.
/// Each span is (start_offset, end_offset, fingerprint_bytes).
struct LiteralSpanFinder {
    spans: Vec<(usize, usize, Vec<u8>)>,
}

impl<'pr> Visit<'pr> for LiteralSpanFinder {
    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        let start = node.location().start_offset();
        let mut end = node.location().end_offset();
        if let Some(close) = node.closing_loc() {
            let close_end = close.end_offset();
            if close_end > end {
                end = close_end;
            }
        }
        let mut fp = b"S:".to_vec();
        fp.extend_from_slice(node.unescaped());
        self.spans.push((start, end, fp));
    }

    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        let start = node.location().start_offset();
        let mut end = node.location().end_offset();
        if let Some(close) = node.closing_loc() {
            let close_end = close.end_offset();
            if close_end > end {
                end = close_end;
            }
        }
        // Use individual parts for content comparison (handles heredocs correctly)
        let mut fp = b"IS:".to_vec();
        for part in node.parts().iter() {
            fp.extend_from_slice(part.location().as_slice());
        }
        self.spans.push((start, end, fp));
        // Don't recurse — treat the whole interpolated string as one literal span
    }

    fn visit_regular_expression_node(&mut self, node: &ruby_prism::RegularExpressionNode<'pr>) {
        let start = node.location().start_offset();
        let end = node.location().end_offset();
        let mut fp = b"R:".to_vec();
        fp.extend_from_slice(node.unescaped());
        let flag_slice = node.closing_loc().as_slice();
        if flag_slice.len() > 1 {
            fp.extend_from_slice(&flag_slice[1..]);
        }
        self.spans.push((start, end, fp));
    }

    fn visit_interpolated_regular_expression_node(
        &mut self,
        node: &ruby_prism::InterpolatedRegularExpressionNode<'pr>,
    ) {
        let start = node.location().start_offset();
        let end = node.location().end_offset();
        let mut fp = b"IR:".to_vec();
        fp.extend_from_slice(node.location().as_slice());
        self.spans.push((start, end, fp));
    }

    fn visit_symbol_node(&mut self, node: &ruby_prism::SymbolNode<'pr>) {
        let start = node.location().start_offset();
        let end = node.location().end_offset();
        let mut fp = b"Y:".to_vec();
        fp.extend_from_slice(node.unescaped());
        self.spans.push((start, end, fp));
    }

    fn visit_interpolated_symbol_node(&mut self, node: &ruby_prism::InterpolatedSymbolNode<'pr>) {
        let start = node.location().start_offset();
        let end = node.location().end_offset();
        let mut fp = b"IY:".to_vec();
        fp.extend_from_slice(node.location().as_slice());
        self.spans.push((start, end, fp));
    }

    fn visit_x_string_node(&mut self, node: &ruby_prism::XStringNode<'pr>) {
        let start = node.location().start_offset();
        let close_end = node.closing_loc().end_offset();
        let end = close_end.max(node.location().end_offset());
        let mut fp = b"X:".to_vec();
        fp.extend_from_slice(node.unescaped());
        self.spans.push((start, end, fp));
    }

    fn visit_interpolated_x_string_node(
        &mut self,
        node: &ruby_prism::InterpolatedXStringNode<'pr>,
    ) {
        let start = node.location().start_offset();
        let close_end = node.closing_loc().end_offset();
        let end = close_end.max(node.location().end_offset());
        let mut fp = b"IX:".to_vec();
        for part in node.parts().iter() {
            fp.extend_from_slice(part.location().as_slice());
        }
        self.spans.push((start, end, fp));
        // Don't recurse — treat the whole interpolated xstring as one literal span
    }
}

/// Returns true if a branch body is a literal that should be ignored when
/// `IgnoreLiteralBranches` is true.
fn is_literal_branch(
    stmts: &Option<ruby_prism::StatementsNode<'_>>,
    ignore_constant: bool,
) -> bool {
    let stmts = match stmts {
        Some(s) => s,
        None => return false,
    };
    let body = stmts.body();
    if body.len() != 1 {
        return false;
    }
    let node = match body.iter().next() {
        Some(n) => n,
        None => return false,
    };
    is_literal_node(&node, ignore_constant)
}

fn is_literal_node(node: &ruby_prism::Node<'_>, ignore_constant: bool) -> bool {
    if node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
    {
        return true;
    }

    if ignore_constant
        && (node.as_constant_read_node().is_some() || node.as_constant_path_node().is_some())
    {
        return true;
    }

    if node.as_regular_expression_node().is_some() {
        return true;
    }

    if let Some(range) = node.as_range_node() {
        let left_ok = range
            .left()
            .is_none_or(|l| is_literal_node(&l, ignore_constant));
        let right_ok = range
            .right()
            .is_none_or(|r| is_literal_node(&r, ignore_constant));
        return left_ok && right_ok;
    }

    if let Some(arr) = node.as_array_node() {
        return arr
            .elements()
            .iter()
            .all(|e| is_literal_node(&e, ignore_constant));
    }

    if let Some(hash) = node.as_hash_node() {
        return hash.elements().iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_literal_node(&assoc.key(), ignore_constant)
                    && is_literal_node(&assoc.value(), ignore_constant)
            } else {
                false
            }
        });
    }

    if let Some(hash) = node.as_keyword_hash_node() {
        return hash.elements().iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_literal_node(&assoc.key(), ignore_constant)
                    && is_literal_node(&assoc.value(), ignore_constant)
            } else {
                false
            }
        });
    }

    false
}

fn is_constant_branch(stmts: &Option<ruby_prism::StatementsNode<'_>>) -> bool {
    let stmts = match stmts {
        Some(s) => s,
        None => return false,
    };
    let body = stmts.body();
    if body.len() != 1 {
        return false;
    }
    let node = match body.iter().next() {
        Some(n) => n,
        None => return false,
    };
    node.as_constant_read_node().is_some() || node.as_constant_path_node().is_some()
}

/// Check if a branch should be considered for duplicate detection based on config.
#[allow(clippy::too_many_arguments)]
fn should_consider(
    stmts: &Option<ruby_prism::StatementsNode<'_>>,
    body: &[u8],
    is_else: bool,
    is_last: bool,
    total_branches: usize,
    ignore_literal: bool,
    ignore_constant: bool,
    ignore_dup_else: bool,
) -> bool {
    if body.is_empty() {
        return false;
    }
    if ignore_literal && is_literal_branch(stmts, ignore_constant) {
        return false;
    }
    if ignore_constant && is_constant_branch(stmts) {
        return false;
    }
    if ignore_dup_else && is_else && is_last && total_branches > 2 {
        return false;
    }
    true
}

fn emit(
    cop: &DuplicateBranch,
    source: &SourceFile,
    offset: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (line, column) = source.offset_to_line_col(offset);
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        "Duplicate branch body detected.".to_string(),
    ));
}

/// A collected branch: body bytes, reporting offset, else flag, last flag, statements.
struct BranchInfo<'pr> {
    body: Vec<u8>,
    report_offset: usize,
    is_else: bool,
    is_last: bool,
    stmts: Option<ruby_prism::StatementsNode<'pr>>,
}

fn check_if_branches(
    cop: &DuplicateBranch,
    source: &SourceFile,
    if_node: &ruby_prism::IfNode<'_>,
    ignore_literal: bool,
    ignore_constant: bool,
    ignore_dup_else: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Skip elsif nodes - only process the outermost if.
    // In Prism, elsif is a nested IfNode whose if_keyword_loc() says "elsif".
    if let Some(kw_loc) = if_node.if_keyword_loc() {
        if kw_loc.as_slice() == b"elsif" {
            return;
        }
    }

    let is_ternary = if_node.if_keyword_loc().is_none();

    // Count total branches
    let mut total = 1usize;
    let mut sub = if_node.subsequent();
    while let Some(s) = sub {
        total += 1;
        if let Some(elsif) = s.as_if_node() {
            sub = elsif.subsequent();
        } else {
            break;
        }
    }

    let mut branches: Vec<BranchInfo<'_>> = Vec::new();

    // The if/ternary true branch
    let if_stmts = if_node.statements();
    let if_body = stmts_source(source, &if_stmts);
    branches.push(BranchInfo {
        body: if_body,
        report_offset: if_node.location().start_offset(),
        is_else: false,
        is_last: false,
        stmts: if_stmts,
    });

    // Walk elsif/else chain
    let mut idx = 1usize;
    let mut subsequent = if_node.subsequent();
    while let Some(sub) = subsequent {
        idx += 1;
        let is_last = idx == total;
        if let Some(elsif) = sub.as_if_node() {
            let stmts = elsif.statements();
            let body = stmts_source(source, &stmts);
            branches.push(BranchInfo {
                body,
                report_offset: elsif.location().start_offset(),
                is_else: false,
                is_last,
                stmts,
            });
            subsequent = elsif.subsequent();
        } else if let Some(else_node) = sub.as_else_node() {
            let stmts = else_node.statements();
            let body = stmts_source(source, &stmts);
            let report_offset = if is_ternary {
                // For ternary, report on the false-branch expression itself
                if let Some(ref s) = stmts {
                    let s_body = s.body();
                    if let Some(first) = s_body.first() {
                        first.location().start_offset()
                    } else {
                        else_node.else_keyword_loc().start_offset()
                    }
                } else {
                    else_node.else_keyword_loc().start_offset()
                }
            } else {
                else_node.else_keyword_loc().start_offset()
            };
            branches.push(BranchInfo {
                body,
                report_offset,
                is_else: true,
                is_last: true,
                stmts,
            });
            break;
        } else {
            break;
        }
    }

    if branches.len() < 2 {
        return;
    }

    let total_branches = branches.len();
    let mut seen = HashSet::new();

    for bi in &branches {
        if !should_consider(
            &bi.stmts,
            &bi.body,
            bi.is_else,
            bi.is_last,
            total_branches,
            ignore_literal,
            ignore_constant,
            ignore_dup_else,
        ) {
            continue;
        }
        if !seen.insert(bi.body.clone()) {
            emit(cop, source, bi.report_offset, diagnostics);
        }
    }
}

fn check_unless_branches(
    cop: &DuplicateBranch,
    source: &SourceFile,
    unless_node: &ruby_prism::UnlessNode<'_>,
    ignore_literal: bool,
    ignore_constant: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // unless only has 2 branches (body and else), so IgnoreDuplicateElseBranch
    // doesn't apply (requires > 2 branches).
    let else_clause = match unless_node.else_clause() {
        Some(e) => e,
        None => return,
    };

    let body_stmts = unless_node.statements();
    let body_src = stmts_source(source, &body_stmts);

    let else_stmts = else_clause.statements();
    let else_src = stmts_source(source, &else_stmts);

    if body_src.is_empty() || else_src.is_empty() {
        return;
    }

    if ignore_literal
        && is_literal_branch(&body_stmts, ignore_constant)
        && is_literal_branch(&else_stmts, ignore_constant)
    {
        return;
    }

    if body_src == else_src {
        emit(
            cop,
            source,
            else_clause.else_keyword_loc().start_offset(),
            diagnostics,
        );
    }
}

fn check_case_branches(
    cop: &DuplicateBranch,
    source: &SourceFile,
    case_node: &ruby_prism::CaseNode<'_>,
    ignore_literal: bool,
    ignore_constant: bool,
    ignore_dup_else: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let conditions = case_node.conditions();
    let has_else = case_node.else_clause().is_some();
    let total_branches = conditions.len() + if has_else { 1 } else { 0 };

    let mut seen = HashSet::new();

    for when_ref in conditions.iter() {
        if let Some(when_node) = when_ref.as_when_node() {
            let stmts = when_node.statements();
            let body = stmts_source(source, &stmts);
            if !should_consider(
                &stmts,
                &body,
                false,
                false,
                total_branches,
                ignore_literal,
                ignore_constant,
                ignore_dup_else,
            ) {
                continue;
            }
            if !seen.insert(body) {
                emit(
                    cop,
                    source,
                    when_node.keyword_loc().start_offset(),
                    diagnostics,
                );
            }
        }
    }

    if let Some(else_clause) = case_node.else_clause() {
        let stmts = else_clause.statements();
        let body = stmts_source(source, &stmts);
        if should_consider(
            &stmts,
            &body,
            true,
            true,
            total_branches,
            ignore_literal,
            ignore_constant,
            ignore_dup_else,
        ) && !seen.insert(body)
        {
            emit(
                cop,
                source,
                else_clause.else_keyword_loc().start_offset(),
                diagnostics,
            );
        }
    }
}

fn check_case_match_branches(
    cop: &DuplicateBranch,
    source: &SourceFile,
    case_match_node: &ruby_prism::CaseMatchNode<'_>,
    ignore_literal: bool,
    ignore_constant: bool,
    ignore_dup_else: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let conditions = case_match_node.conditions();
    let has_else = case_match_node.else_clause().is_some();
    let total_branches = conditions.len() + if has_else { 1 } else { 0 };

    let mut seen = HashSet::new();

    for in_ref in conditions.iter() {
        if let Some(in_node) = in_ref.as_in_node() {
            let stmts = in_node.statements();
            let body = stmts_source(source, &stmts);
            if !should_consider(
                &stmts,
                &body,
                false,
                false,
                total_branches,
                ignore_literal,
                ignore_constant,
                ignore_dup_else,
            ) {
                continue;
            }
            if !seen.insert(body) {
                emit(cop, source, in_node.in_loc().start_offset(), diagnostics);
            }
        }
    }

    if let Some(else_clause) = case_match_node.else_clause() {
        let stmts = else_clause.statements();
        let body = stmts_source(source, &stmts);
        if should_consider(
            &stmts,
            &body,
            true,
            true,
            total_branches,
            ignore_literal,
            ignore_constant,
            ignore_dup_else,
        ) && !seen.insert(body)
        {
            emit(
                cop,
                source,
                else_clause.else_keyword_loc().start_offset(),
                diagnostics,
            );
        }
    }
}

fn check_rescue_branches(
    cop: &DuplicateBranch,
    source: &SourceFile,
    begin_node: &ruby_prism::BeginNode<'_>,
    ignore_literal: bool,
    ignore_constant: bool,
    ignore_dup_else: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut rescue_data: Vec<(Vec<u8>, usize, Option<ruby_prism::StatementsNode<'_>>)> = Vec::new();

    let mut rescue_opt = begin_node.rescue_clause();
    while let Some(rescue_node) = rescue_opt {
        let stmts = rescue_node.statements();
        let body = stmts_source(source, &stmts);
        let offset = rescue_node.keyword_loc().start_offset();
        rescue_data.push((body, offset, stmts));
        rescue_opt = rescue_node.subsequent();
    }

    let has_else = begin_node.else_clause().is_some();
    let total_branches = rescue_data.len() + if has_else { 1 } else { 0 };

    let mut seen = HashSet::new();

    for (body, offset, stmts) in &rescue_data {
        if !should_consider(
            stmts,
            body,
            false,
            false,
            total_branches,
            ignore_literal,
            ignore_constant,
            ignore_dup_else,
        ) {
            continue;
        }
        if !seen.insert(body.clone()) {
            emit(cop, source, *offset, diagnostics);
        }
    }

    if let Some(else_clause) = begin_node.else_clause() {
        let stmts = else_clause.statements();
        let body = stmts_source(source, &stmts);
        if should_consider(
            &stmts,
            &body,
            true,
            true,
            total_branches,
            ignore_literal,
            ignore_constant,
            ignore_dup_else,
        ) && !seen.insert(body)
        {
            emit(
                cop,
                source,
                else_clause.else_keyword_loc().start_offset(),
                diagnostics,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateBranch, "cops/lint/duplicate_branch");

    #[test]
    fn keyword_hash_counts_as_literal_branch() {
        let result = ruby_prism::parse(b"call(foo: 1)\n");
        let root = result.node();
        let program = root.as_program_node().unwrap();
        let stmts = program.statements();
        let call = stmts.body().iter().next().unwrap().as_call_node().unwrap();
        let arg = call.arguments().unwrap().arguments().iter().next().unwrap();

        assert!(arg.as_keyword_hash_node().is_some());
        assert!(is_literal_node(&arg, false));
    }

    #[test]
    fn whitespace_normalization() {
        // Whitespace is stripped, with spaces preserved only between word chars
        assert_eq!(
            normalize_whitespace(b"foo(x)  .bar  {|y| baz(y)}"),
            b"foo(x).bar{|y|baz(y)}"
        );
        // Both should match after normalization when the only difference is
        // trailing space before closing brace
        assert_eq!(
            normalize_whitespace(b"each_child(node).all? {|child| check(child)}"),
            normalize_whitespace(b"each_child(node).all? {|child| check(child) }"),
        );
        // Blank lines between statements collapse
        assert_eq!(
            normalize_whitespace(b"report(e)\n  false"),
            normalize_whitespace(b"report(e)\n\n  false"),
        );
        // Space only preserved between two word chars
        assert_eq!(normalize_whitespace(b"return []"), b"return[]");
        // Space preserved between identifiers
        assert_eq!(normalize_whitespace(b"foo bar baz"), b"foo bar baz");
    }

    #[test]
    fn array_fingerprint_distinguishes_pattern_locals_from_method_calls() {
        let src = b"case caught\nin Enumerator => body\n  [status, headers, body]\nin [Integer => status, Enumerator => body]\n  [status, headers, body]\nend\n";
        let result = ruby_prism::parse(src);
        let root = result.node();
        let program = root.as_program_node().unwrap();
        let case_match = program
            .statements()
            .body()
            .iter()
            .next()
            .unwrap()
            .as_case_match_node()
            .unwrap();

        let mut branches = case_match.conditions().iter();
        let first = branches.next().unwrap().as_in_node().unwrap();
        let second = branches.next().unwrap().as_in_node().unwrap();

        assert_ne!(
            stmts_fingerprint(src, &first.statements()),
            stmts_fingerprint(src, &second.statements())
        );
    }
}
