use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Attr;

impl Cop for Attr {
    fn name(&self) -> &'static str {
        "Style/Attr"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be a bare `attr` call (no receiver)
        if call_node.name().as_slice() != b"attr" {
            return;
        }
        if call_node.receiver().is_some() {
            return;
        }

        // Must have arguments
        let args = match call_node.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();

        // Check if second argument is `true` → attr_accessor, otherwise attr_reader
        let has_true_arg = arg_list.get(1).is_some_and(|a| a.as_true_node().is_some());
        let has_false_arg = arg_list.get(1).is_some_and(|a| a.as_false_node().is_some());

        let replacement = if has_true_arg {
            "attr_accessor"
        } else {
            "attr_reader"
        };

        let msg_loc = call_node
            .message_loc()
            .unwrap_or_else(|| call_node.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            format!("Do not use `attr`. Use `{replacement}` instead."),
        );
        if let Some(ref mut corr) = corrections {
            if has_true_arg || has_false_arg {
                // Replace the entire call: `attr :name, true/false` → `attr_accessor/attr_reader :name`
                // We need to replace from `attr` through the boolean arg, keeping only the first arg
                let first_arg = &arg_list[0];
                let first_arg_str = source.byte_slice(
                    first_arg.location().start_offset(),
                    first_arg.location().end_offset(),
                    "",
                );
                corr.push(crate::correction::Correction {
                    start: msg_loc.start_offset(),
                    end: call_node.location().end_offset(),
                    replacement: format!("{replacement} {first_arg_str}"),
                    cop_name: self.name(),
                    cop_index: 0,
                });
            } else {
                // Simple replacement: `attr` → `attr_reader`
                corr.push(crate::correction::Correction {
                    start: msg_loc.start_offset(),
                    end: msg_loc.end_offset(),
                    replacement: replacement.to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
            }
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Attr, "cops/style/attr");
    crate::cop_autocorrect_fixture_tests!(Attr, "cops/style/attr");
}
