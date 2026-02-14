use crate::cop::util::has_keyword_arg;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct NotNullColumn;

impl Cop for NotNullColumn {
    fn name(&self) -> &'static str {
        "Rails/NotNullColumn"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["db/migrate/**/*.rb"]
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

        if call.name().as_slice() != b"add_column" {
            return Vec::new();
        }

        // Check for null: false
        let null_val = match crate::cop::util::keyword_arg_value(&call, b"null") {
            Some(v) => v,
            None => return Vec::new(),
        };

        // Check if null: false
        if null_val.as_false_node().is_none() {
            return Vec::new();
        }

        // Check if default: is present
        if has_keyword_arg(&call, b"default") {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Do not add a NOT NULL column without a default value.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NotNullColumn, "cops/rails/not_null_column");
}
