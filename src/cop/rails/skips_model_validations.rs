use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SkipsModelValidations;

const SKIP_METHODS: &[&[u8]] = &[
    b"update_attribute",
    b"touch",
    b"update_column",
    b"update_columns",
    b"update_all",
    b"toggle!",
    b"increment!",
    b"decrement!",
    b"insert",
    b"insert!",
    b"insert_all",
    b"insert_all!",
    b"upsert",
    b"upsert_all",
];

impl Cop for SkipsModelValidations {
    fn name(&self) -> &'static str {
        "Rails/SkipsModelValidations"
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
        let method_name = call.name().as_slice();
        if !SKIP_METHODS.contains(&method_name) {
            return Vec::new();
        }
        if call.receiver().is_none() {
            return Vec::new();
        }
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let msg = format!(
            "Avoid `{}` because it skips validations.",
            std::str::from_utf8(method_name).unwrap_or("?")
        );
        vec![self.diagnostic(source, line, column, msg)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SkipsModelValidations, "cops/rails/skips_model_validations");
}
