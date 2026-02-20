use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, KEYWORD_HASH_NODE, SPLAT_NODE};

pub struct HashConversion;

impl Cop for HashConversion {
    fn name(&self) -> &'static str {
        "Style/HashConversion"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, KEYWORD_HASH_NODE, SPLAT_NODE]
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

        // Must be Hash[] call
        if call.name().as_slice() != b"[]" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Receiver must be Hash constant
        let is_hash = receiver.as_constant_read_node()
            .map_or(false, |c| c.name().as_slice() == b"Hash")
            || receiver.as_constant_path_node().map_or(false, |cp| {
                cp.parent().is_none() && cp.name().map_or(false, |n| n.as_slice() == b"Hash")
            });

        if !is_hash {
            return;
        }

        let _allow_splat = config.get_bool("AllowSplatArgument", true);

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        let args = call.arguments();

        if let Some(args) = args {
            let arg_list: Vec<_> = args.arguments().iter().collect();

            // Check for splat argument
            if _allow_splat && arg_list.iter().any(|a| a.as_splat_node().is_some()) {
                return;
            }

            // Check for keyword hash argument
            if arg_list.len() == 1 && arg_list[0].as_keyword_hash_node().is_some() {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer literal hash to `Hash[key: value, ...]`.".to_string(),
                ));
                return;
            }

            if arg_list.len() == 1 {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer `ary.to_h` to `Hash[ary]`.".to_string(),
                ));
                return;
            }

            // Multi-argument
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer literal hash to `Hash[arg1, arg2, ...]`.".to_string(),
            ));
            return;
        }

        // No arguments: Hash[]
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Prefer literal hash to `Hash[arg1, arg2, ...]`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashConversion, "cops/style/hash_conversion");
}
