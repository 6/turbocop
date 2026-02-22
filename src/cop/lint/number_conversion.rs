use crate::cop::node_type::{CALL_NODE, FLOAT_NODE, IMAGINARY_NODE, INTEGER_NODE, RATIONAL_NODE};
use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Warns about unsafe number conversion using `to_i`, `to_f`, `to_c`, `to_r`.
/// Prefers strict `Integer()`, `Float()`, etc. Disabled by default.
pub struct NumberConversion;

const CONVERSION_METHODS: &[(&[u8], &str)] = &[
    (b"to_i", "Integer(%s, 10)"),
    (b"to_f", "Float(%s)"),
    (b"to_c", "Complex(%s)"),
    (b"to_r", "Rational(%s)"),
];

impl Cop for NumberConversion {
    fn name(&self) -> &'static str {
        "Lint/NumberConversion"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
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
            None => return,
        };

        let method_name = call.name().as_slice();
        let conversion = match CONVERSION_METHODS.iter().find(|(m, _)| *m == method_name) {
            Some(c) => c,
            None => return,
        };

        // Must have a receiver
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Must not have arguments
        if call.arguments().is_some() {
            return;
        }

        // Skip if receiver is numeric (already a number)
        if receiver.as_integer_node().is_some()
            || receiver.as_float_node().is_some()
            || receiver.as_rational_node().is_some()
            || receiver.as_imaginary_node().is_some()
        {
            return;
        }

        // Skip if receiver itself is a conversion method
        if let Some(recv_call) = receiver.as_call_node() {
            let recv_method = recv_call.name().as_slice();
            if CONVERSION_METHODS.iter().any(|(m, _)| *m == recv_method) {
                return;
            }
            // Skip allowed methods from config
            let allowed = config
                .get_string_array("AllowedMethods")
                .unwrap_or_default();
            let allowed_patterns = config
                .get_string_array("AllowedPatterns")
                .unwrap_or_default();
            if let Ok(name) = std::str::from_utf8(recv_method) {
                if allowed.iter().any(|a| a == name) {
                    return;
                }
                // Skip if receiver method matches any AllowedPatterns (regex)
                for pattern in &allowed_patterns {
                    if let Ok(re) = regex::Regex::new(pattern) {
                        if re.is_match(name) {
                            return;
                        }
                    }
                }
            }
        }

        // Skip ignored classes - check the receiver and walk one level deeper
        let ignored_classes = config
            .get_string_array("IgnoredClasses")
            .unwrap_or_else(|| vec!["Time".to_string(), "DateTime".to_string()]);
        if is_ignored_class(&receiver, &ignored_classes) {
            return;
        }

        // Safe navigation check: &.to_i is fine
        if let Some(op) = call.call_operator_loc() {
            if op.as_slice() == b"&." {
                return;
            }
        }

        let recv_src = node_source(source, &receiver);
        let method_str = std::str::from_utf8(method_name).unwrap_or("to_i");
        let corrected = conversion.1.replace("%s", recv_src);

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Replace unsafe number conversion with number class parsing, instead of using `{}.{}`, use stricter `{}`.",
                recv_src, method_str, corrected
            ),
        ));
    }
}

/// Check if node (or its receiver chain root) is an ignored class constant.
fn is_ignored_class(node: &ruby_prism::Node<'_>, ignored_classes: &[String]) -> bool {
    // Direct constant check
    if let Some(name_bytes) = constant_name(node) {
        if let Ok(name) = std::str::from_utf8(name_bytes) {
            if ignored_classes.iter().any(|c| c == name) {
                return true;
            }
        }
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
    std::str::from_utf8(&source.as_bytes()[loc.start_offset()..loc.end_offset()]).unwrap_or("...")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NumberConversion, "cops/lint/number_conversion");
}
