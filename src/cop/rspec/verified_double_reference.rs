use crate::cop::node_type::{CALL_NODE, STRING_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct VerifiedDoubleReference;

const VERIFIED_DOUBLES: &[&[u8]] = &[
    b"class_double",
    b"class_spy",
    b"instance_double",
    b"instance_spy",
    b"mock_model",
    b"object_double",
    b"object_spy",
    b"stub_model",
];

/// Default enforced style is constant — flags string references in verified doubles.
/// `instance_double('ClassName')` -> `instance_double(ClassName)`
impl Cop for VerifiedDoubleReference {
    fn name(&self) -> &'static str {
        "RSpec/VerifiedDoubleReference"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE]
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

        let method_name = call.name().as_slice();
        if !VERIFIED_DOUBLES.iter().any(|&d| d == method_name) {
            return;
        }

        // Must be receiverless
        if call.receiver().is_some() {
            return;
        }

        // Check the first argument — should be a string (we flag it)
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let first_arg = &arg_list[0];
        if let Some(str_node) = first_arg.as_string_node() {
            let content = str_node.unescaped();
            // Only flag if the content looks like a class name
            if !content.is_empty() && content[0].is_ascii_uppercase() || content.starts_with(b":") {
                let loc = first_arg.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use a constant class reference for verified doubles. String references are not verifying unless the class is loaded.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        VerifiedDoubleReference,
        "cops/rspec/verified_double_reference"
    );
}
