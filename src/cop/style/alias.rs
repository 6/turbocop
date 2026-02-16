use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Alias;

impl Cop for Alias {
    fn name(&self) -> &'static str {
        "Style/Alias"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "prefer_alias");

        // Check alias keyword usage
        if let Some(alias_node) = node.as_alias_method_node() {
            if enforced_style == "prefer_alias_method" {
                let loc = alias_node.location();
                let kw_slice = &source.content[loc.start_offset()..];
                // Only flag if it starts with `alias` keyword
                if kw_slice.starts_with(b"alias ") || kw_slice.starts_with(b"alias\t") {
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `alias_method` instead of `alias`.".to_string(),
                    )];
                }
            }
            return Vec::new();
        }

        // Check alias_method call
        if let Some(call_node) = node.as_call_node() {
            if enforced_style == "prefer_alias" {
                let name = call_node.name();
                if name.as_slice() == b"alias_method" && call_node.receiver().is_none() {
                    let msg_loc = call_node.message_loc().unwrap_or_else(|| call_node.location());
                    let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `alias` instead of `alias_method`.".to_string(),
                    )];
                }
            }
            return Vec::new();
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Alias, "cops/style/alias");
}
