use crate::cop::node_type::CALL_NODE;
use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ReflectionClassName;

const ASSOCIATION_METHODS: &[&[u8]] = &[
    b"has_many",
    b"has_one",
    b"belongs_to",
    b"has_and_belongs_to_many",
];

/// Check if a node is a constant (ConstantReadNode or ConstantPathNode).
fn is_constant(node: &ruby_prism::Node<'_>) -> bool {
    node.as_constant_read_node().is_some() || node.as_constant_path_node().is_some()
}

impl Cop for ReflectionClassName {
    fn name(&self) -> &'static str {
        "Rails/ReflectionClassName"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
            // RuboCop only flags constants and method calls on constants.
            // Bare constants: `class_name: Account`, `class_name: Foo::Bar`
            // Method calls on constants: `class_name: Account.name`, `Account.to_s`
            // Everything else (strings, symbols, method calls, self.xxx) is allowed.
            let should_flag = if is_constant(&value) {
                true
            } else if let Some(method_call) = value.as_call_node() {
                method_call
                    .receiver()
                    .is_some_and(|recv| is_constant(&recv))
            } else {
                false
            };

            if should_flag {
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
