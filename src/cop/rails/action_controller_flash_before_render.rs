use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ActionControllerFlashBeforeRender;

impl Cop for ActionControllerFlashBeforeRender {
    fn name(&self) -> &'static str {
        "Rails/ActionControllerFlashBeforeRender"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["app/controllers/**/*.rb"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Look for `flash[...] = ...` which is a CallNode with method_name `[]=` and
        // receiver being a call to `flash`
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // We're looking for flash[:key] = value
        // This is parsed as a call to `[]=` on the receiver `flash`
        if call.name().as_slice() != b"[]=" {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Check if receiver is a call to `flash` (receiverless)
        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if recv_call.name().as_slice() != b"flash" || recv_call.receiver().is_some() {
            return Vec::new();
        }

        // Check it's not flash.now
        // flash.now[:key] = value would have receiver being a call to `now` on `flash`
        // Since we already checked receiver is `flash` directly, this is plain flash

        let loc = recv_call.message_loc().unwrap_or(recv_call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `flash.now` when using `flash` before `render`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ActionControllerFlashBeforeRender,
        "cops/rails/action_controller_flash_before_render"
    );
}
