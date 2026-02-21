use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, INTERPOLATED_STRING_NODE, STRING_NODE, SYMBOL_NODE};

pub struct ReflectionClassName;

const ASSOCIATION_METHODS: &[&[u8]] = &[
    b"has_many",
    b"has_one",
    b"belongs_to",
    b"has_and_belongs_to_many",
];

impl Cop for ReflectionClassName {
    fn name(&self) -> &'static str {
        "Rails/ReflectionClassName"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTERPOLATED_STRING_NODE, STRING_NODE, SYMBOL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if call.receiver().is_some() {
            return;
        }
        if !ASSOCIATION_METHODS.contains(&call.name().as_slice()) {
            return;
        }
        if let Some(value) = keyword_arg_value(&call, b"class_name") {
            // RuboCop flags non-string values (constants, method calls) for class_name.
            // ActiveRecord expects class_name to be a string.
            // Symbols are also acceptable (e.g., `class_name: :Article`).
            // Method calls ending in .to_s are also acceptable — they produce strings.
            let is_to_s = value.as_call_node().is_some_and(|c| c.name().as_slice() == b"to_s");
            if value.as_string_node().is_none()
                && value.as_symbol_node().is_none()
                && value.as_interpolated_string_node().is_none()
                && !is_to_s
            {
                let loc = value.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use a string value for `class_name`.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ReflectionClassName, "cops/rails/reflection_class_name");
}
