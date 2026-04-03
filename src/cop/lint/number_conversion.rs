use crate::cop::shared::method_dispatch_predicates;
use crate::cop::shared::node_type::{
    CALL_NODE, CALL_OR_WRITE_NODE, FLOAT_NODE, IMAGINARY_NODE, INTEGER_NODE, RATIONAL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Warns about unsafe number conversion using `to_i`, `to_f`, `to_c`, `to_r`.
/// Prefers strict `Integer()`, `Float()`, etc. Disabled by default.
///
/// ## Corpus investigation findings (2026-03-10)
///
/// Root causes of 54 FP + 1,567 FN:
/// - FP: Safe navigation `&.to_i` was incorrectly skipped; RuboCop flags it.
/// - FN (bulk): Symbol form patterns not handled — `map(&:to_i)`, `try(:to_f)`,
///   `send(:to_c)` were entirely missing.
/// - FN: Kernel conversion methods (`Integer`, `Float`, `Complex`, `Rational`)
///   were not included in the receiver-skip list, potentially causing missed
///   skips (though this was a minor contributor).
///
/// Fixes applied:
/// 1. Removed the `&.` early return — safe navigation is still an offense.
/// 2. Added symbol form detection: block-pass `&:to_i` and symbol arg `:to_f`
///    patterns (for `try`/`send`/etc.) with single-argument guard.
/// 3. Added Kernel conversion methods to the receiver-call skip list.
///
/// ## Corpus investigation findings (2026-03-14)
///
/// Root causes of 16 FP + 40 FN (from 99.7% match rate):
/// - FP: Block-pass with additional arguments like `foo.map(x, &:to_i)` was
///   incorrectly flagged. In RuboCop, block_pass counts as an argument so
///   `arguments.one?` returns false for these; in Prism, block is separate.
/// - FN: Bare symbol form without explicit receiver (implicit self) like
///   `map(&:to_i)` or `try(:to_f)` was incorrectly skipped. RuboCop's
///   `handle_as_symbol` guard checks the method name capture (never nil),
///   not the AST receiver, so it flags these.
///
/// Fixes applied:
/// 1. Added `call.arguments().is_some()` guard in block_pass branch to skip
///    when regular arguments exist alongside the block_pass.
/// 2. Removed the `call.receiver().is_none()` early return from
///    `handle_symbol_form` so bare symbol forms are flagged.
///
/// ## Corpus investigation (2026-03-21)
///
/// Corpus oracle reported FP=0, FN=11.
///
/// FN=11: All from DataDog/dd-trace-rb using `Core::Utils::Time.now.to_i`.
/// `is_ignored_class` used `constant_short_name()` which returns just the last
/// segment "Time" from `Core::Utils::Time`, matching the default IgnoredClasses
/// ["Time", "DateTime"]. But RuboCop uses `const_name` which returns the full
/// qualified name "Core::Utils::Time" — this does NOT match "Time" so it's
/// flagged. Fix: use full source text of the root constant for comparison.
///
/// FP=10: All from rooted constant paths like `::Time.now.to_f`. The
/// `is_ignored_class` function compared the full source text `::Time` against
/// IgnoredClasses `["Time", "DateTime"]` — the `::` prefix prevented matching.
/// Fix: strip leading `::` before comparing against IgnoredClasses.
///
/// ## Corpus investigation (2026-03-28)
///
/// Corpus oracle reported FP=0, FN=6.
///
/// FN=4: `receiver.to_i ||= fallback` forms were missed because Prism collapses
/// the read side into `CallOrWriteNode` instead of exposing a nested `CallNode`
/// for `to_i`. RuboCop still sees the inner `send` and flags it, so nitrocop
/// must treat `CallOrWriteNode#read_name` like a direct conversion call.
///
/// Fix: subscribe to `CALL_OR_WRITE_NODE` and run the same receiver checks and
/// message construction against the node's receiver/read_name pair.
pub struct NumberConversion;

const CONVERSION_METHODS: &[(&[u8], &str)] = &[
    (b"to_i", "Integer(%s, 10)"),
    (b"to_f", "Float(%s)"),
    (b"to_c", "Complex(%s)"),
    (b"to_r", "Rational(%s)"),
];

/// Kernel conversion method names that should be treated as already-safe receivers.
const KERNEL_CONVERSION_METHODS: &[&[u8]] = &[b"Integer", b"Float", b"Complex", b"Rational"];

impl Cop for NumberConversion {
    fn name(&self) -> &'static str {
        "Lint/NumberConversion"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CALL_OR_WRITE_NODE,
            FLOAT_NODE,
            IMAGINARY_NODE,
            INTEGER_NODE,
            RATIONAL_NODE,
        ]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => {
                let Some(call) = node.as_call_or_write_node() else {
                    return;
                };

                let method_name = call.read_name().as_slice();
                if let Some(conversion) = CONVERSION_METHODS.iter().find(|(m, _)| *m == method_name)
                {
                    let Some(receiver) = call.receiver() else {
                        return;
                    };
                    self.handle_direct_conversion_like(
                        source,
                        node,
                        &receiver,
                        conversion,
                        config,
                        diagnostics,
                    );
                }
                return;
            }
        };

        let method_name = call.name().as_slice();

        // Try direct conversion method first (e.g., `x.to_i`)
        if let Some(conversion) = CONVERSION_METHODS.iter().find(|(m, _)| *m == method_name) {
            self.handle_direct_conversion(source, node, &call, conversion, config, diagnostics);
            return;
        }

        // Try symbol form (e.g., `map(&:to_i)`, `try(:to_f)`, `send(:to_c)`)
        self.handle_symbol_form(source, node, &call, config, diagnostics);
    }
}

impl NumberConversion {
    /// Handle direct conversion: `receiver.to_i`, `receiver.to_f`, etc.
    fn handle_direct_conversion(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        call: &ruby_prism::CallNode<'_>,
        conversion: &(&[u8], &str),
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Must have a receiver
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Must not have arguments
        if call.arguments().is_some() {
            return;
        }

        self.handle_direct_conversion_like(
            source,
            node,
            &receiver,
            conversion,
            config,
            diagnostics,
        );
    }

    fn handle_direct_conversion_like(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        receiver: &ruby_prism::Node<'_>,
        conversion: &(&[u8], &str),
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Skip if receiver is numeric (already a number)
        if receiver.as_integer_node().is_some()
            || receiver.as_float_node().is_some()
            || receiver.as_rational_node().is_some()
            || receiver.as_imaginary_node().is_some()
        {
            return;
        }

        // Skip if receiver itself is a conversion method or Kernel conversion
        if let Some(recv_call) = receiver.as_call_node() {
            let recv_method = recv_call.name().as_slice();
            // RuboCop's `receiver.send_type?` excludes `csend`, so safe-navigation
            // receivers like `foo&.second.to_f` must still be flagged even when
            // `second` appears in AllowedMethods (e.g. from rubocop-rails).
            if !method_dispatch_predicates::is_safe_navigation(&recv_call) {
                if CONVERSION_METHODS.iter().any(|(m, _)| *m == recv_method) {
                    return;
                }
                if KERNEL_CONVERSION_METHODS.contains(&recv_method) {
                    return;
                }
                // Skip allowed methods from config
                if self.is_allowed_method(recv_method, config) {
                    return;
                }
            }
        }

        // Skip ignored classes - check the receiver and walk one level deeper
        let ignored_classes = config
            .get_string_array("IgnoredClasses")
            .unwrap_or_else(|| vec!["Time".to_string(), "DateTime".to_string()]);
        if is_ignored_class(receiver, &ignored_classes) {
            return;
        }

        let recv_src = node_source(source, receiver);
        let method_str = std::str::from_utf8(conversion.0).unwrap_or("to_i");
        let corrected = conversion.1.replace("%s", recv_src);

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Replace unsafe number conversion with number class parsing, instead of using `{recv_src}.{method_str}`, use stricter `{corrected}`.",
            ),
        ));
    }

    /// Handle symbol form: `map(&:to_i)`, `try(:to_f)`, `send(:to_c)`, etc.
    /// RuboCop flags these even without an explicit receiver (implicit self).
    fn handle_symbol_form(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        call: &ruby_prism::CallNode<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Check block-pass form: map(&:to_i)
        // In RuboCop, block_pass counts as an argument, so `map(x, &:to_i)` has 2 args
        // and is skipped by `arguments.one?`. In Prism, block is separate from arguments,
        // so we must check that no regular arguments exist alongside the block_pass.
        // Only enter this branch for block arguments (&:to_i), NOT regular blocks
        // ({ ... }). Regular blocks like `receive(:to_i) { 1 }` should fall through
        // to the symbol argument check below.
        if let Some(block) = call.block() {
            if let Some(block_arg) = block.as_block_argument_node() {
                if call.arguments().is_some() {
                    return;
                }
                if let Some(expr) = block_arg.expression() {
                    if let Some(sym) = expr.as_symbol_node() {
                        let sym_value = sym.unescaped();
                        if let Some(conversion) =
                            CONVERSION_METHODS.iter().find(|(m, _)| *m == sym_value)
                        {
                            let sym_src = node_source(source, &block);
                            let corrected =
                                format!("{{ |i| {} }}", conversion.1.replace("%s", "i"));
                            let loc = node.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!(
                                    "Replace unsafe number conversion with number class parsing, instead of using `{sym_src}`, use stricter `{corrected}`.",
                                ),
                            ));
                        }
                    }
                }
                return;
            }
            // Regular block (not block argument) — fall through to symbol arg check
        }

        // Check symbol argument form: try(:to_f), send(:to_c)
        // Must have exactly one argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }
        let arg = &arg_list[0];
        if let Some(sym) = arg.as_symbol_node() {
            let sym_value = sym.unescaped();
            if let Some(conversion) = CONVERSION_METHODS.iter().find(|(m, _)| *m == sym_value) {
                let sym_src = node_source(source, arg);
                let corrected = format!("{{ |i| {} }}", conversion.1.replace("%s", "i"));
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Replace unsafe number conversion with number class parsing, instead of using `{sym_src}`, use stricter `{corrected}`.",
                    ),
                ));
            }
        }
    }

    fn is_allowed_method(&self, method_name: &[u8], config: &CopConfig) -> bool {
        let allowed = config
            .get_string_array("AllowedMethods")
            .unwrap_or_default();
        let allowed_patterns = config
            .get_string_array("AllowedPatterns")
            .unwrap_or_default();
        if let Ok(name) = std::str::from_utf8(method_name) {
            if allowed.iter().any(|a| a == name) {
                return true;
            }
            for pattern in &allowed_patterns {
                if let Ok(re) = regex::Regex::new(pattern) {
                    if re.is_match(name) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Check if the root receiver of the receiver chain is an ignored class constant.
/// RuboCop uses `top_receiver` to walk the receiver chain to the root constant,
/// then `const_name` to get the full qualified name (e.g., "Core::Utils::Time").
/// The IgnoredClasses check compares the FULL name, so "Core::Utils::Time" does
/// NOT match "Time" in the default list.
fn is_ignored_class(node: &ruby_prism::Node<'_>, ignored_classes: &[String]) -> bool {
    // If this is a constant, check it directly using full source text
    if node.as_constant_read_node().is_some() || node.as_constant_path_node().is_some() {
        let name_bytes = node.location().as_slice();
        if let Ok(name) = std::str::from_utf8(name_bytes) {
            let stripped = name.strip_prefix("::").unwrap_or(name);
            return ignored_classes.iter().any(|c| c == stripped);
        }
        return false;
    }
    // Walk receiver chain: check if it's a call whose receiver is an ignored class
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            return is_ignored_class(&recv, ignored_classes);
        }
    }
    false
}

fn node_source<'a>(source: &'a SourceFile, node: &ruby_prism::Node<'_>) -> &'a str {
    let loc = node.location();
    source.byte_slice(loc.start_offset(), loc.end_offset(), "...")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use serde_yml::Value;

    fn config_with_allowed_methods(methods: &[&str]) -> CopConfig {
        CopConfig {
            options: HashMap::from([(
                "AllowedMethods".to_string(),
                Value::Sequence(
                    methods
                        .iter()
                        .map(|method| Value::String((*method).to_string()))
                        .collect(),
                ),
            )]),
            ..CopConfig::default()
        }
    }

    #[test]
    fn allowed_method_receiver_skips_regular_send() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &NumberConversion,
            b"10.seconds.to_i\n",
            config_with_allowed_methods(&["second", "seconds"]),
        );
    }

    #[test]
    fn allowed_method_receiver_does_not_skip_safe_navigation_send() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &NumberConversion,
            br#"f = flow_data[i]&.second.to_f
    ^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/NumberConversion: Replace unsafe number conversion with number class parsing, instead of using `flow_data[i]&.second.to_f`, use stricter `Float(flow_data[i]&.second)`.
"#,
            config_with_allowed_methods(&["second", "seconds"]),
        );
    }

    crate::cop_fixture_tests!(NumberConversion, "cops/lint/number_conversion");
}
