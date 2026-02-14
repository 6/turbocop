use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ActiveSupportAliases;

impl Cop for ActiveSupportAliases {
    fn name(&self) -> &'static str {
        "Rails/ActiveSupportAliases"
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

        if call.receiver().is_none() {
            return Vec::new();
        }

        let name = call.name().as_slice();
        let replacement = if name == b"starts_with?" {
            "start_with?"
        } else if name == b"ends_with?" {
            "end_with?"
        } else {
            return Vec::new();
        };

        let original = std::str::from_utf8(name).unwrap_or("?");

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `{replacement}` instead of `{original}`."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ActiveSupportAliases, "cops/rails/active_support_aliases");
}
