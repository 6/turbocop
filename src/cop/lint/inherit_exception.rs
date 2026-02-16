use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct InheritException;

impl Cop for InheritException {
    fn name(&self) -> &'static str {
        "Lint/InheritException"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "standard_error");
        let _supported = config.get_string_array("SupportedStyles");

        let prefer = match style {
            "runtime_error" => "RuntimeError",
            _ => "StandardError",
        };

        // Check class Foo < Exception
        if let Some(class_node) = node.as_class_node() {
            let parent = match class_node.superclass() {
                Some(p) => p,
                None => return Vec::new(),
            };

            if is_exception(&parent) {
                let loc = parent.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Inherit from `{prefer}` instead of `Exception`."),
                )];
            }
            return Vec::new();
        }

        // Check Class.new(Exception)
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() != b"new" {
                return Vec::new();
            }

            let receiver = match call.receiver() {
                Some(r) => r,
                None => return Vec::new(),
            };

            let recv_name = match constant_name(&receiver) {
                Some(n) => n,
                None => return Vec::new(),
            };

            if recv_name != b"Class" {
                return Vec::new();
            }

            let arguments = match call.arguments() {
                Some(a) => a,
                None => return Vec::new(),
            };

            let args = arguments.arguments();
            if let Some(first_arg) = args.first() {
                if is_exception(&first_arg) {
                    let loc = first_arg.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Inherit from `{prefer}` instead of `Exception`."),
                    )];
                }
            }
        }

        Vec::new()
    }
}

fn is_exception(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(name) = constant_name(node) {
        return name == b"Exception";
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InheritException, "cops/lint/inherit_exception");
}
