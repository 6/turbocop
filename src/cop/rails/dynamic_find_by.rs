use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DynamicFindBy;

impl Cop for DynamicFindBy {
    fn name(&self) -> &'static str {
        "Rails/DynamicFindBy"
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
        if !name.starts_with(b"find_by_") {
            return Vec::new();
        }
        if call.receiver().is_none() {
            return Vec::new();
        }
        let attr = &name[b"find_by_".len()..];
        let attr_str = std::str::from_utf8(attr).unwrap_or("...");
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let msg = format!(
            "Use `find_by({attr_str}: ...)` instead of `{}`.",
            std::str::from_utf8(name).unwrap_or("find_by_...")
        );
        vec![self.diagnostic(source, line, column, msg)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DynamicFindBy, "cops/rails/dynamic_find_by");
}
