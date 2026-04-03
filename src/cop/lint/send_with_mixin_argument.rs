use crate::cop::shared::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, STRING_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=5, FN=5.
///
/// FP=5 / FN=5: the reproduced multiline mismatches were all selector-range
/// issues when `.send` was wrapped after the receiver. RuboCop anchors the
/// offense at the selector range (`send(:include, Foo)`), not the whole call
/// expression starting at the receiver. While fixing the span, also build the
/// replacement/module list from every mixin argument to match upstream behavior.
///
/// Local rerun after the fix still undercounts 3 offenses in aggregate, but the
/// remaining gap is concentrated in vendor-path repos (`webistrano`,
/// `standalone-migrations`, `SquareSquash/web`) while `jruby` contributes
/// file-drop noise. `--list-target-files` under the corpus baseline config
/// skips those remaining vendor files locally, and the exact multiline/no-paren
/// shapes are covered by fixtures, so the remaining mismatch appears to be
/// corpus file-selection noise outside this cop's AST matching logic.
pub struct SendWithMixinArgument;

const SEND_METHODS: &[&[u8]] = &[b"send", b"public_send", b"__send__"];
const MIXIN_METHODS: &[&[u8]] = &[b"include", b"prepend", b"extend"];

impl Cop for SendWithMixinArgument {
    fn name(&self) -> &'static str {
        "Lint/SendWithMixinArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
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

        // Must be send/public_send/__send__
        if !SEND_METHODS.contains(&method_name) {
            return;
        }

        // Must have a receiver (constant)
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Receiver should be a constant
        if recv.as_constant_read_node().is_none() && recv.as_constant_path_node().is_none() {
            return;
        }

        // Must have at least 2 arguments: the mixin method name and the module
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() < 2 {
            return;
        }

        // First argument must be a symbol or string that's a mixin method
        let first_arg = &arg_list[0];
        let mixin_name = if let Some(sym) = first_arg.as_symbol_node() {
            sym.unescaped().to_vec()
        } else if let Some(s) = first_arg.as_string_node() {
            s.unescaped().to_vec()
        } else {
            return;
        };

        if !MIXIN_METHODS.iter().any(|m| **m == *mixin_name) {
            return;
        }

        let mut module_names = Vec::new();
        for arg in &arg_list[1..] {
            if arg.as_constant_read_node().is_none() && arg.as_constant_path_node().is_none() {
                return;
            }

            let start = arg.location().start_offset();
            let end = arg.location().end_offset();
            let module_name = source.byte_slice(start, end, "Module");
            module_names.push(module_name.to_string());
        }

        let mixin_str = std::str::from_utf8(&mixin_name).unwrap_or("include");
        let msg_loc = call.message_loc().unwrap_or(call.location());
        let bad_method = source.byte_slice(
            msg_loc.start_offset(),
            call.location().end_offset(),
            "send(...)",
        );
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `{mixin_str} {}` instead of `{bad_method}`.",
                module_names.join(", ")
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SendWithMixinArgument, "cops/lint/send_with_mixin_argument");
}
