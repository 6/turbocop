use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, INTERPOLATED_STRING_NODE};

pub struct RedundantInterpolationUnfreeze;

impl Cop for RedundantInterpolationUnfreeze {
    fn name(&self) -> &'static str {
        "Style/RedundantInterpolationUnfreeze"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTERPOLATED_STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // minimum_target_ruby_version 3.0 — only applies for Ruby 3.0+
        let ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64())
            .unwrap_or(3.4);
        if ruby_version < 3.0 {
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Check for +@ (unary plus) or .dup on interpolated string
        let is_unfreeze = if name == b"+@" {
            // +@ can be prefix `+"#{foo}"` or method call `"#{foo}".+@`
            true
        } else if name == b"dup" {
            // "#{foo}".dup
            true
        } else {
            false
        };

        if !is_unfreeze {
            return;
        }

        // Receiver must be an interpolated string
        let is_interpolated = receiver.as_interpolated_string_node().is_some();
        if !is_interpolated {
            return;
        }

        // Report at the operator/method location
        if let Some(msg_loc) = call.message_loc() {
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Don't unfreeze interpolated strings as they are already unfrozen.".to_string(),
            ));
            return;
        }

        // For prefix +, the call_operator is None and message_loc might not exist
        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Don't unfreeze interpolated strings as they are already unfrozen.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantInterpolationUnfreeze, "cops/style/redundant_interpolation_unfreeze");
}
