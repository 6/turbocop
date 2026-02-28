use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ReverseFirst;

impl Cop for ReverseFirst {
    fn name(&self) -> &'static str {
        "Performance/ReverseFirst"
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
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        if chain.inner_method != b"reverse" || chain.outer_method != b"first" {
            return;
        }

        // Report at the inner call's selector (.reverse), matching RuboCop's
        // `receiver.loc.selector.begin_pos`
        let inner_msg_loc = chain
            .inner_call
            .message_loc()
            .unwrap_or(chain.inner_call.location());
        let (line, column) = source.offset_to_line_col(inner_msg_loc.start_offset());

        let outer_call = node.as_call_node().unwrap();
        let msg = if let Some(args) = outer_call.arguments() {
            if let Some(first_arg) = args.arguments().iter().next() {
                let arg_text = std::str::from_utf8(first_arg.location().as_slice()).unwrap_or("n");
                let dot = match outer_call.call_operator_loc() {
                    Some(loc) if loc.as_slice() == b"&." => "&.",
                    _ => ".",
                };
                format!(
                    "Use `last({arg_text}){dot}reverse` instead of `reverse{dot}first({arg_text})`."
                )
            } else {
                "Use `last` instead of `reverse.first`.".to_string()
            }
        } else {
            "Use `last` instead of `reverse.first`.".to_string()
        };

        diagnostics.push(self.diagnostic(source, line, column, msg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(ReverseFirst, "cops/performance/reverse_first");
}
