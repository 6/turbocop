use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct StringMethods;

impl Cop for StringMethods {
    fn name(&self) -> &'static str {
        "Style/StringMethods"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let preferred_methods = config.get_string_hash("PreferredMethods");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        let name = call.name().as_slice();
        let name_str = match std::str::from_utf8(name) {
            Ok(s) => s,
            Err(_) => return,
        };

        // Check against preferred methods (default: intern -> to_sym)
        let preferred = if let Some(ref map) = preferred_methods {
            map.get(name_str).cloned()
        } else {
            // Default mapping
            if name_str == "intern" {
                Some("to_sym".to_string())
            } else {
                None
            }
        };

        if let Some(replacement) = preferred {
            let loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Prefer `{}` over `{}`.", replacement, name_str),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StringMethods, "cops/style/string_methods");
}
