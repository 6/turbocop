use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Performance/Size flags `.count` (no args, no block) on receivers that are
/// known to be Array or Hash values: literals, `.to_a`/`.to_h` conversions,
/// and `Array()`/`Array[]`/`Hash()`/`Hash[]` constructors.
///
/// Root cause of 36 FNs: the cop previously only matched literal array/hash
/// receivers, missing `.to_a`/`.to_h` chains and `Array()`/`Hash()` calls.
/// Fixed by checking the receiver for conversion methods and constructor
/// patterns in addition to literals.
///
/// FP fix: RuboCop skips `.count` when `node.parent&.block_type?` — i.e., when
/// the `.count` call is the direct body of a block (single-statement block body
/// where the return value is used as the block's value). In Parser AST, a
/// single-statement block has the statement as a direct child of the block node,
/// while multi-statement blocks wrap in `begin`. In Prism, the body is always a
/// `StatementsNode`, so we check statement count and compare the byte offset of
/// the sole statement against the `count` call's offset — only skipping if the
/// `count` call IS the direct sole statement (not deeply nested within it).
///
/// ## Extended corpus investigation (2026-03-23)
///
/// Extended corpus reported FP=0, FN=1. Root cause: the `parent_is_block` flag
/// was propagating through all children of a single-statement block body (array,
/// hash, etc.), causing deeply nested `.to_a.count` to be incorrectly skipped.
/// Fixed by switching from a boolean flag to an offset comparison — only the
/// call node at the exact byte offset of the sole block body statement is
/// suppressed.
pub struct Size;

impl Cop for Size {
    fn name(&self) -> &'static str {
        "Performance/Size"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let mut visitor = SizeVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            block_body_stmt_range: None,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct SizeVisitor<'a, 'src> {
    cop: &'a Size,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Byte range (start, end) of the sole statement in a block body, if any.
    /// Used to match RuboCop's `node.parent&.block_type?` — only the call node
    /// spanning this exact range is suppressed, not chained or nested children.
    /// Chained calls like `[].count.should` share the same start_offset but
    /// differ in end_offset, so comparing both is necessary.
    block_body_stmt_range: Option<(usize, usize)>,
}

impl SizeVisitor<'_, '_> {
    /// Visit a block-like node's body, recording the sole statement's offset
    /// so that only a `count` call at that exact position is suppressed.
    fn visit_block_body(&mut self, body: &ruby_prism::Node<'_>) {
        let prev = self.block_body_stmt_range;
        if let Some(stmts) = body.as_statements_node() {
            let mut iter = stmts.body().iter();
            if let Some(first) = iter.next() {
                if iter.next().is_none() {
                    // Single statement — record its full byte range
                    let loc = first.location();
                    self.block_body_stmt_range = Some((
                        loc.start_offset(),
                        loc.start_offset() + loc.as_slice().len(),
                    ));
                }
            }
        }
        self.visit(body);
        self.block_body_stmt_range = prev;
    }
}

impl<'pr> Visit<'pr> for SizeVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"count"
            && node.arguments().is_none()
            && node.block().is_none()
        {
            // RuboCop: skip when `node.parent&.block_type?` — the `count` call
            // is the direct sole statement of a block body. We check by comparing
            // both start and end byte offsets: only the call spanning the exact
            // range of the sole block body statement is suppressed. Chained calls
            // like `[].count.should` share the same start_offset but differ in
            // end_offset, so comparing both is necessary.
            let node_loc = node.location();
            let node_end = node_loc.start_offset() + node_loc.as_slice().len();
            let is_direct_block_body = self
                .block_body_stmt_range
                .is_some_and(|(start, end)| start == node_loc.start_offset() && end == node_end);
            if !is_direct_block_body {
                if let Some(recv) = node.receiver() {
                    if is_array_or_hash_receiver(&recv) {
                        let loc = node.message_loc().unwrap_or(node.location());
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Use `size` instead of `count`.".to_string(),
                        ));
                    }
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        if let Some(params) = node.parameters() {
            self.visit(&params);
        }
        if let Some(body) = node.body() {
            self.visit_block_body(&body);
        }
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        if let Some(params) = node.parameters() {
            self.visit(&params);
        }
        if let Some(body) = node.body() {
            self.visit_block_body(&body);
        }
    }
}

/// Returns true if the node is known to produce an Array or Hash:
/// - Array/Hash literals
/// - `.to_a` / `.to_h` calls (any receiver)
/// - `Array[...]` / `Array(...)` / `Hash[...]` / `Hash(...)`
fn is_array_or_hash_receiver(node: &ruby_prism::Node<'_>) -> bool {
    // Array or Hash literal (including keyword hash arguments)
    if node.as_array_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
    {
        return true;
    }

    // Check for call-based patterns: .to_a, .to_h, Array[], Array(), Hash[], Hash()
    if let Some(call) = node.as_call_node() {
        let name = call.name();
        let name_bytes = name.as_slice();

        // .to_a or .to_h on any receiver
        if name_bytes == b"to_a" || name_bytes == b"to_h" {
            return true;
        }

        // Array[...] or Hash[...] — `[]` method on constant `Array` or `Hash`
        if name_bytes == b"[]" {
            if let Some(recv) = call.receiver() {
                if is_array_or_hash_constant(&recv) {
                    return true;
                }
            }
        }

        // Array(...) or Hash(...) — Kernel method call with no explicit receiver
        if (name_bytes == b"Array" || name_bytes == b"Hash") && call.receiver().is_none() {
            return true;
        }
    }

    false
}

/// Checks if a node is a constant `Array` or `Hash` (simple or qualified like `::Array`).
fn is_array_or_hash_constant(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(c) = node.as_constant_read_node() {
        let name = c.name();
        let name_bytes = name.as_slice();
        return name_bytes == b"Array" || name_bytes == b"Hash";
    }
    if let Some(cp) = node.as_constant_path_node() {
        // ::Array or ::Hash (top-level constant path with no parent)
        if cp.parent().is_none() {
            let src = cp.location().as_slice();
            return src == b"::Array" || src == b"::Hash";
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Size, "cops/performance/size");
}
