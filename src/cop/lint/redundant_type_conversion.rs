use crate::cop::shared::node_type::{
    ARRAY_NODE, CALL_NODE, FLOAT_NODE, HASH_NODE, IMAGINARY_NODE, INTEGER_NODE,
    INTERPOLATED_STRING_NODE, INTERPOLATED_SYMBOL_NODE, KEYWORD_HASH_NODE, RATIONAL_NODE,
    STRING_NODE, SYMBOL_NODE,
};
use crate::cop::shared::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for redundant type conversions like `"text".to_s`, `:sym.to_sym`,
/// `42.to_i`, `[].to_a`, etc.
///
/// ## Investigation findings
///
/// FP fix: `is_constructor` and `is_kernel_method` used `constant_short_name()` which
/// returns the last segment for both `ConstantReadNode` (bare `Array`) and
/// `ConstantPathNode` (qualified `Native::Array`). This caused false positives
/// for qualified paths like `Native::Array.new(x).to_a` since `Native::Array`
/// is not Ruby's `Array`. Fixed by checking `as_constant_read_node()` directly
/// instead of using `constant_short_name()`, so only bare constants match.
///
/// FN fix: Missing `to_json` as a typed method that always returns a String.
/// RuboCop's `TYPED_METHODS` maps `to_s` to `[:inspect, :to_json]`, but we
/// only checked `inspect`. Added `to_json` to the chained typed method check.
///
/// ## Corpus investigation (2026-03-15)
///
/// Corpus oracle reported FP=0, FN=1.
///
/// FN fix:
/// - Prism wraps receivers like `("#{left}:#{right}")` in a `ParenthesesNode`.
///   Unwrap single-expression parentheses before checking whether the receiver
///   is already string/symbol/array/hash-like, so `(...).to_s` is treated the
///   same as the underlying expression.
///
/// ## Corpus investigation (2026-03-28)
///
/// Corpus oracle reported FP=0, FN=1 at `clio__ten_years_rails__a8389a2:
/// exe/deprecations:43` for `#{__FILE__.to_s}` inside a heredoc banner.
///
/// FN fix:
/// - Prism represents `__FILE__` as `SourceFileNode`, not `StringNode`.
///   RuboCop still treats `__FILE__.to_s` as redundant because `__FILE__`
///   always yields a String. Treat `SourceFileNode` as string-like for
///   `to_s` redundancy checks.
pub struct RedundantTypeConversion;

impl Cop for RedundantTypeConversion {
    fn name(&self) -> &'static str {
        "Lint/RedundantTypeConversion"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            CALL_NODE,
            FLOAT_NODE,
            HASH_NODE,
            IMAGINARY_NODE,
            INTEGER_NODE,
            INTERPOLATED_STRING_NODE,
            INTERPOLATED_SYMBOL_NODE,
            KEYWORD_HASH_NODE,
            RATIONAL_NODE,
            STRING_NODE,
            SYMBOL_NODE,
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Must have no arguments
        if call.arguments().is_some() {
            return;
        }

        // For to_h and to_set, skip if there's a block — the block transforms
        // the elements, so it's a different operation.
        if (method_name == b"to_h" || method_name == b"to_set") && call.block().is_some() {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        let receiver = util::unwrap_parentheses(receiver);

        let is_redundant = match method_name {
            b"to_s" => {
                receiver.as_string_node().is_some()
                    || receiver.as_interpolated_string_node().is_some()
                    || receiver.as_source_file_node().is_some()
                    || is_constructor(&receiver, b"String", b"new")
                    || is_chained_method(&receiver, b"to_s")
                    || is_chained_method(&receiver, b"inspect")
                    || is_chained_method(&receiver, b"to_json")
            }
            b"to_sym" => {
                receiver.as_symbol_node().is_some()
                    || receiver.as_interpolated_symbol_node().is_some()
                    || is_chained_method(&receiver, b"to_sym")
            }
            b"to_i" => {
                receiver.as_integer_node().is_some()
                    || is_chained_method(&receiver, b"to_i")
                    || is_kernel_method(&receiver, b"Integer")
            }
            b"to_f" => {
                receiver.as_float_node().is_some()
                    || is_chained_method(&receiver, b"to_f")
                    || is_kernel_method(&receiver, b"Float")
            }
            b"to_r" => {
                receiver.as_rational_node().is_some()
                    || is_chained_method(&receiver, b"to_r")
                    || is_kernel_method(&receiver, b"Rational")
            }
            b"to_c" => {
                receiver.as_imaginary_node().is_some()
                    || is_chained_method(&receiver, b"to_c")
                    || is_kernel_method(&receiver, b"Complex")
            }
            b"to_a" => {
                receiver.as_array_node().is_some()
                    || is_constructor(&receiver, b"Array", b"new")
                    || is_constructor(&receiver, b"Array", b"[]")
                    || is_chained_method(&receiver, b"to_a")
                    || is_kernel_method(&receiver, b"Array")
            }
            b"to_h" => {
                // Note: as_keyword_hash_node() is not checked here because keyword
                // hash nodes (keyword args like `foo(a: 1)`) cannot be receivers.
                receiver.as_hash_node().is_some()
                    || is_constructor(&receiver, b"Hash", b"new")
                    || is_constructor(&receiver, b"Hash", b"[]")
                    || is_chained_method(&receiver, b"to_h")
            }
            _ => false,
        };

        if !is_redundant {
            return;
        }

        let method_str = std::str::from_utf8(method_name).unwrap_or("to_s");
        let msg_loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Redundant `{}` detected.", method_str),
        ));
    }
}

fn is_constructor(node: &ruby_prism::Node<'_>, class_name: &[u8], method: &[u8]) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == method {
            if let Some(recv) = call.receiver() {
                // Only match bare constants (ConstantReadNode), not qualified paths
                // (ConstantPathNode). `Array.new` is redundant with `.to_a`, but
                // `Native::Array.new` is a different class entirely.
                // Also match cbase-prefixed constants like `::Array.new`.
                if let Some(cr) = recv.as_constant_read_node() {
                    return cr.name().as_slice() == class_name;
                }
                if let Some(cp) = recv.as_constant_path_node() {
                    // Match `::Array` (cbase prefix, no parent namespace)
                    if cp.parent().is_none() {
                        if let Some(name) = cp.name() {
                            return name.as_slice() == class_name;
                        }
                    }
                }
            }
        }
    }
    false
}

fn is_chained_method(node: &ruby_prism::Node<'_>, method: &[u8]) -> bool {
    if let Some(call) = node.as_call_node() {
        return call.name().as_slice() == method;
    }
    false
}

fn is_kernel_method(node: &ruby_prism::Node<'_>, method: &[u8]) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() != method {
            return false;
        }
        // Must be receiverless or Kernel.Method or ::Kernel.Method
        if call.receiver().is_none() {
            return true;
        }
        if let Some(recv) = call.receiver() {
            // Match bare `Kernel`
            if let Some(cr) = recv.as_constant_read_node() {
                return cr.name().as_slice() == b"Kernel";
            }
            // Match `::Kernel` (cbase prefix)
            if let Some(cp) = recv.as_constant_path_node() {
                if cp.parent().is_none() {
                    if let Some(name) = cp.name() {
                        return name.as_slice() == b"Kernel";
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantTypeConversion,
        "cops/lint/redundant_type_conversion"
    );
}
