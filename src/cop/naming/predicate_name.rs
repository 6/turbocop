use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct PredicateName;

impl Cop for PredicateName {
    fn name(&self) -> &'static str {
        "Naming/PredicateName"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let method_name = def_node.name().as_slice();
        let name_str = match std::str::from_utf8(method_name) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let (prefix, suggested) = if name_str.starts_with("has_") {
            ("has_", &name_str[4..])
        } else if name_str.starts_with("is_") {
            ("is_", &name_str[3..])
        } else {
            return Vec::new();
        };

        let _ = prefix; // used only for matching
        let loc = def_node.name_loc();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Rename `{name_str}` to `{suggested}`."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PredicateName, "cops/naming/predicate_name");
}
