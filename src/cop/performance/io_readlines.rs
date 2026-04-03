/// Performance/IoReadlines
///
/// Identifies places where inefficient `readlines` method can be replaced by
/// `each_line` to avoid fully loading file content into memory.
///
/// ## Investigation (2026-03-04)
///
/// Root cause of 0% match rate (208 FN): the original implementation listened
/// on CONSTANT_PATH_NODE/CONSTANT_READ_NODE and tried to look outward via
/// `as_method_chain`. This meant:
/// 1. Instance calls (`file.readlines.each`) were never matched (no constant receiver)
/// 2. Only `each` and `map` were checked, missing all other Enumerable methods
/// 3. Message format didn't match RuboCop ("IO.foreach" vs "each_line")
/// 4. Offense location covered the whole expression instead of readlines..method range
///
/// Fix: Rewrote to listen on CALL_NODE, check if method is an Enumerable method,
/// look at receiver for a `readlines` call, and match RuboCop's message format
/// and offense range exactly.
///
/// ## Investigation (2026-03-07) — FP=30 fix
///
/// Two bugs causing 30 false positives across 23 repos:
///
/// Bug 1: `is_io_or_file_const()` matched both ConstantReadNode and ConstantPathNode,
/// but RuboCop's class pattern `(const nil? {:IO :File})` only matches unqualified
/// constants. So `::File.readlines(...)` and `Foo::File.readlines(...)` were incorrectly
/// flagged. Fix: removed ConstantPathNode handling from `is_io_or_file_const`.
///
/// Bug 2: For the class pattern (IO/File receiver), we flagged all chained enumerable
/// calls regardless of args. But RuboCop's pattern `$(send ... _)` has just `_` (method
/// name only) — no extra children. So `File.readlines(x).map(&:chomp)` should NOT match
/// because `&:chomp` is a BlockArgumentNode child of the outer send. The instance pattern
/// uses `_ ...` which does allow args/block_pass. Fix: for class pattern, skip if outer
/// call has arguments or a BlockArgumentNode block.
use crate::cop::shared::method_identifier_predicates;
use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct IoReadlines;

/// Check if a node is an unqualified IO or File constant (ConstantReadNode only).
/// RuboCop's class pattern uses `(const nil? {:IO :File})` which only matches
/// simple unqualified constants, not `::File` or `Foo::File` (ConstantPathNode).
fn is_io_or_file_const(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(cr) = node.as_constant_read_node() {
        let name = cr.name().as_slice();
        return name == b"IO" || name == b"File";
    }
    false
}

impl Cop for IoReadlines {
    fn name(&self) -> &'static str {
        "Performance/IoReadlines"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let outer_method = outer_call.name().as_slice();
        if !method_identifier_predicates::is_enumerable_method(outer_method) {
            return;
        }

        // The receiver of the outer call must be a `readlines` call
        let receiver = match outer_call.receiver() {
            Some(r) => r,
            None => return,
        };
        let readlines_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if readlines_call.name().as_slice() != b"readlines" {
            return;
        }

        // RuboCop matches two patterns:
        // 1. Class call: (IO|File).readlines(...).method — receiver is IO/File constant
        //    Only matches unqualified constants. Outer call must have NO args and NO block_pass.
        // 2. Instance call: expr.readlines(...).method — receiver is non-constant or nil
        //    Allows args and block_pass on the outer call.
        let is_class_pattern = if let Some(recv) = readlines_call.receiver() {
            if is_io_or_file_const(&recv) {
                true
            } else if recv.as_constant_read_node().is_some()
                || recv.as_constant_path_node().is_some()
            {
                // Constant receiver but not IO/File — skip entirely
                return;
            } else {
                false // non-constant receiver → instance pattern
            }
        } else {
            false // nil receiver (bare `readlines`) → instance pattern
        };

        // For the class pattern, RuboCop's NodePattern is:
        //   $(send $(send (const nil? {:IO :File}) :readlines ...) _)
        // The outer send has only `_` (method name) with no extra children,
        // meaning no positional args and no block_pass.
        // A regular block `each { }` wraps the send (not a child), so it's fine.
        // A block_pass `map(&:chomp)` is a child of the send, so it must NOT match.
        if is_class_pattern {
            if outer_call.arguments().is_some() {
                return;
            }
            if let Some(block) = outer_call.block() {
                if block.as_block_argument_node().is_some() {
                    return;
                }
            }
        }

        // Build message matching RuboCop format
        let outer_name = std::str::from_utf8(outer_method).unwrap_or("?");
        let message = if outer_method == b"each" {
            "Use `each_line` instead of `readlines.each`.".to_string()
        } else {
            format!("Use `each_line.{outer_name}` instead of `readlines.{outer_name}`.")
        };

        // Offense location starts at `readlines` method name
        let readlines_loc = readlines_call
            .message_loc()
            .unwrap_or(readlines_call.location());
        let (line, column) = source.offset_to_line_col(readlines_loc.start_offset());

        diagnostics.push(self.diagnostic(source, line, column, message));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IoReadlines, "cops/performance/io_readlines");
}
