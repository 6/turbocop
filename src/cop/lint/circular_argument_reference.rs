use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct CircularArgumentReference;

impl Cop for CircularArgumentReference {
    fn name(&self) -> &'static str {
        "Lint/CircularArgumentReference"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check optional keyword arguments: def foo(bar: bar)
        if let Some(kwopt) = node.as_optional_keyword_parameter_node() {
            let param_name = kwopt.name().as_slice();
            let value = kwopt.value();
            if is_circular_ref(param_name, &value) {
                let loc = value.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Circular argument reference - `{}`.",
                        std::str::from_utf8(param_name).unwrap_or("?")
                    ),
                )];
            }
            return Vec::new();
        }

        // Check optional positional arguments: def foo(bar = bar)
        if let Some(optarg) = node.as_optional_parameter_node() {
            let param_name = optarg.name().as_slice();
            let value = optarg.value();
            if is_circular_ref(param_name, &value) {
                let loc = value.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Circular argument reference - `{}`.",
                        std::str::from_utf8(param_name).unwrap_or("?")
                    ),
                )];
            }
            return Vec::new();
        }

        Vec::new()
    }
}

fn is_circular_ref(param_name: &[u8], value: &ruby_prism::Node<'_>) -> bool {
    // Direct reference: def foo(x = x) where value is a local variable read
    if let Some(lvar) = value.as_local_variable_read_node() {
        return lvar.name().as_slice() == param_name;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CircularArgumentReference, "cops/lint/circular_argument_reference");
}
