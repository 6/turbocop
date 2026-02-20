use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct InstanceSpy;

impl Cop for InstanceSpy {
    fn name(&self) -> &'static str {
        "RSpec/InstanceSpy"
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
        // Detect `instance_double(Foo).as_null_object` and suggest `instance_spy`
        // BUT only when `have_received` is used in the same file, because
        // instance_spy is only meaningful when message expectations are verified.
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"as_null_object" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if recv_call.name().as_slice() != b"instance_double" || recv_call.receiver().is_some() {
            return;
        }

        // Only flag if `have_received` appears somewhere in the source file.
        // Without `have_received`, the null object is used purely as a stub,
        // and `instance_spy` would not be appropriate.
        let bytes = source.as_bytes();
        if !has_pattern_in_bytes(bytes, b"have_received") {
            return;
        }

        let loc = recv_call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `instance_spy` when you check your double with `have_received`.".to_string(),
        ));
    }
}

fn has_pattern_in_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InstanceSpy, "cops/rspec/instance_spy");
}
