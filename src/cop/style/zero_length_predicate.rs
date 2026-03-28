use crate::cop::node_type::{CALL_NODE, INTEGER_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/ZeroLengthPredicate: Checks for `size == 0`, `length.zero?`, etc.
///
/// ## Investigation findings (2026-03-14)
/// FP=2, FN=0. Two false positive patterns found:
/// 1. Safe navigation chains (e.g., `values&.length&.> 0`) — `empty?` is not equivalent
///    because nil handling differs. Fixed by checking `call_operator()` for `&.` on any
///    call in the chain.
/// 2. Non-collection `.size`/`.length` (e.g., `File.stat(path).size.zero?`) — the receiver
///    returns an integer (file size), not a collection. Fixed by checking if the receiver
///    of `.size`/`.length` is a call on a constant (e.g., `File.stat`), which indicates
///    a non-collection context.
///
/// ## Investigation findings (2026-03-28)
/// FP=1, FN=87. Two code bugs found:
/// 1. `is_non_collection_receiver` was too broad — it excluded ANY `.size`/`.length` where
///    the receiver chain included a constant (e.g., `Post.find_all.length > 0`,
///    `ENV['X'].size > 0`, `Object.methods.length > 0`). RuboCop only excludes
///    `File.stat`, `File/Tempfile/StringIO.new/open`. Fixed by matching those specific
///    constants and methods only.
/// 2. Safe navigation was blocked for ALL comparisons. RuboCop allows safe nav on
///    `.length`/`.size` for zero-length checks (`x&.length == 0`, `x&.length < 1`) but
///    not for nonzero checks (`x&.length > 0`). Fixed by splitting the comparison logic:
///    zero checks allow safe nav on the inner call, nonzero checks require no safe nav.
/// 3. The single FP (octocatalog-diff multiline block `.size.zero?`) was context-dependent
///    and resolved itself with the non-collection receiver fix.
pub struct ZeroLengthPredicate;

impl ZeroLengthPredicate {
    /// Check if a CallNode uses safe navigation (`&.`)
    fn uses_safe_navigation(call: &ruby_prism::CallNode<'_>) -> bool {
        call.call_operator_loc()
            .is_some_and(|op: ruby_prism::Location<'_>| op.as_slice() == b"&.")
    }

    /// Check if the receiver of `.size`/`.length` is a non-polymorphic collection type
    /// that doesn't have `empty?` (File, Tempfile, StringIO).
    /// Matches: File.stat(x).size, File/Tempfile/StringIO.new/open(...).size
    fn is_non_collection_receiver(call: &ruby_prism::CallNode<'_>) -> bool {
        if let Some(receiver) = call.receiver() {
            if let Some(recv_call) = receiver.as_call_node() {
                if let Some(recv_recv) = recv_call.receiver() {
                    if let Some(const_name) = Self::bare_constant_name(&recv_recv) {
                        let method_bytes = recv_call.name().as_slice();
                        // File.stat(x).size/length
                        if const_name == b"File" && method_bytes == b"stat" {
                            return true;
                        }
                        // File/Tempfile/StringIO.new/open(...).size/length
                        if (const_name == b"File"
                            || const_name == b"Tempfile"
                            || const_name == b"StringIO")
                            && (method_bytes == b"new" || method_bytes == b"open")
                        {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Extract bare constant name from ConstantReadNode or top-level ConstantPathNode (::File).
    fn bare_constant_name<'a>(node: &ruby_prism::Node<'a>) -> Option<&'a [u8]> {
        if let Some(cr) = node.as_constant_read_node() {
            Some(cr.name().as_slice())
        } else if let Some(cp) = node.as_constant_path_node() {
            // Only match top-level (::File) — parent must be None
            if cp.parent().is_none() {
                cp.name().map(|n| n.as_slice())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Check if a call is `.length` or `.size` on a collection receiver
    /// (excludes non-collection receivers like File.stat, but allows safe navigation)
    fn is_length_or_size(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            let name = call.name();
            let name_bytes = name.as_slice();
            if (name_bytes == b"length" || name_bytes == b"size")
                && call.arguments().is_none()
                && call.receiver().is_some()
                && !Self::is_non_collection_receiver(&call)
            {
                return true;
            }
        }
        false
    }

    /// Check if a length/size call uses safe navigation (`&.`)
    fn length_or_size_has_safe_nav(node: &ruby_prism::Node<'_>) -> bool {
        node.as_call_node()
            .is_some_and(|call| Self::uses_safe_navigation(&call))
    }

    /// Get the integer value from a node
    fn int_value(node: &ruby_prism::Node<'_>) -> Option<i64> {
        if let Some(int_node) = node.as_integer_node() {
            // We need to extract the integer value from the source
            let src = int_node.location().as_slice();
            if let Ok(s) = std::str::from_utf8(src) {
                return s.parse::<i64>().ok();
            }
        }
        None
    }
}

impl Cop for ZeroLengthPredicate {
    fn name(&self) -> &'static str {
        "Style/ZeroLengthPredicate"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTEGER_NODE]
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

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        // Pattern: x.length.zero? or x.size.zero?
        if method_bytes == b"zero?"
            && call.arguments().is_none()
            && !Self::uses_safe_navigation(&call)
        {
            if let Some(receiver) = call.receiver() {
                if Self::is_length_or_size(&receiver) {
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let src = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Use `empty?` instead of `{}`.", src),
                    ));
                }
            }
        }

        // Pattern: x.length == 0, x.size == 0, 0 == x.length, x.length < 1, etc.
        if matches!(method_bytes, b"==" | b"!=" | b">" | b"<") && !Self::uses_safe_navigation(&call)
        {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() == 1 {
                    if let Some(receiver) = call.receiver() {
                        // x.length == 0 or x.length < 1
                        if Self::is_length_or_size(&receiver) {
                            let arg_val = Self::int_value(&arg_list[0]);
                            // Zero checks (empty?) — allowed with safe nav on length/size
                            let is_zero_check = (method_bytes == b"==" && arg_val == Some(0))
                                || (method_bytes == b"<" && arg_val == Some(1));
                            // Nonzero checks (!empty?) — NOT allowed with safe nav
                            let is_nonzero_check = (method_bytes == b"!=" || method_bytes == b">")
                                && arg_val == Some(0)
                                && !Self::length_or_size_has_safe_nav(&receiver);
                            if is_zero_check || is_nonzero_check {
                                let loc = node.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                let src = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                                let msg = if is_nonzero_check {
                                    format!("Use `!empty?` instead of `{}`.", src)
                                } else {
                                    format!("Use `empty?` instead of `{}`.", src)
                                };
                                diagnostics.push(self.diagnostic(source, line, column, msg));
                            }
                        }
                        // 0 == x.length, 1 > x.length
                        if let Some(recv_val) = Self::int_value(&receiver) {
                            if Self::is_length_or_size(&arg_list[0]) {
                                // Zero checks (empty?) — allowed with safe nav
                                let is_zero_check = (method_bytes == b"==" && recv_val == 0)
                                    || (method_bytes == b">" && recv_val == 1);
                                // Nonzero checks (!empty?) — NOT allowed with safe nav
                                let is_nonzero_check = (method_bytes == b"!="
                                    || method_bytes == b"<")
                                    && recv_val == 0
                                    && !Self::length_or_size_has_safe_nav(&arg_list[0]);
                                if is_zero_check || is_nonzero_check {
                                    let loc = node.location();
                                    let (line, column) =
                                        source.offset_to_line_col(loc.start_offset());
                                    let src = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                                    let msg = if is_nonzero_check {
                                        format!("Use `!empty?` instead of `{}`.", src)
                                    } else {
                                        format!("Use `empty?` instead of `{}`.", src)
                                    };
                                    diagnostics.push(self.diagnostic(source, line, column, msg));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ZeroLengthPredicate, "cops/style/zero_length_predicate");
}
