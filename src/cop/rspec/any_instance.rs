use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct AnyInstance;

impl Cop for AnyInstance {
    fn name(&self) -> &'static str {
        "RSpec/AnyInstance"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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

        let method_name = call.name().as_slice();

        // Check for `allow_any_instance_of(...)` and `expect_any_instance_of(...)`
        if (method_name == b"allow_any_instance_of" || method_name == b"expect_any_instance_of")
            && call.receiver().is_none()
        {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let label = std::str::from_utf8(method_name).unwrap_or("allow_any_instance_of");
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Avoid stubbing using `{label}`."),
            ));
        }

        // Check for old syntax: `Object.any_instance`
        if method_name == b"any_instance" && call.receiver().is_some() {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Avoid stubbing using `any_instance`.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AnyInstance, "cops/rspec/any_instance");
}
