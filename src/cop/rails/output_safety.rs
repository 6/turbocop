use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct OutputSafety;

const I18N_METHODS: &[&[u8]] = &[b"t", b"translate", b"l", b"localize"];

/// Check if the receiver is a non-interpolated string literal.
fn is_non_interpolated_string(receiver: &ruby_prism::Node<'_>) -> bool {
    if receiver.as_string_node().is_some() {
        return true;
    }
    // Interpolated string where all parts are string literals (adjacent string concatenation)
    if let Some(dstr) = receiver.as_interpolated_string_node() {
        return dstr.parts().iter().all(|part| part.as_string_node().is_some());
    }
    false
}

/// Recursively check if any node in the tree is an i18n method call.
/// Matches: t(), translate(), l(), localize(), I18n.t(), I18n.translate(), etc.
fn contains_i18n_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if I18N_METHODS.contains(&name) {
            // No receiver (bare t/translate/l/localize) or receiver is I18n constant
            if call.receiver().is_none() {
                return true;
            }
            if let Some(recv) = call.receiver() {
                if recv.as_constant_read_node().is_some_and(|c| c.name().as_slice() == b"I18n") {
                    return true;
                }
                if recv
                    .as_constant_path_node()
                    .is_some_and(|cp| {
                        cp.name()
                            .is_some_and(|n| n.as_slice() == b"I18n")
                    })
                {
                    return true;
                }
            }
        }
        // Check receiver chain
        if let Some(recv) = call.receiver() {
            if contains_i18n_call(&recv) {
                return true;
            }
        }
    }
    false
}

impl Cop for OutputSafety {
    fn name(&self) -> &'static str {
        "Rails/OutputSafety"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let name = call.name().as_slice();

        if name == b"html_safe" {
            let receiver = match call.receiver() {
                Some(r) => r,
                None => return Vec::new(),
            };

            // No arguments allowed for html_safe
            if call.arguments().is_some() {
                return Vec::new();
            }

            // Exempt non-interpolated string literals
            if is_non_interpolated_string(&receiver) {
                return Vec::new();
            }

            // Exempt i18n method calls in the receiver chain
            if contains_i18n_call(&receiver) {
                return Vec::new();
            }
        } else if name == b"raw" {
            // raw() must be called without a receiver (command style)
            if call.receiver().is_some() {
                return Vec::new();
            }
            // Must have exactly one argument
            let args = match call.arguments() {
                Some(a) => a,
                None => return Vec::new(),
            };
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 1 {
                return Vec::new();
            }
        } else if name == b"safe_concat" {
            if call.receiver().is_none() {
                return Vec::new();
            }
            // Must have exactly one argument
            let args = match call.arguments() {
                Some(a) => a,
                None => return Vec::new(),
            };
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 1 {
                return Vec::new();
            }
        } else {
            return Vec::new();
        }

        // Use message_loc to point to the method name (html_safe/raw/safe_concat)
        // instead of the entire call expression, matching RuboCop's `node.loc.selector`.
        let loc = call.message_loc().unwrap_or(node.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Tagging a string as html safe may be a security risk.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OutputSafety, "cops/rails/output_safety");
}
