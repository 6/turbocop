use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ActiveSupportOnLoad;

/// Map of Rails framework classes to their on_load hook names.
const FRAMEWORK_CLASSES: &[(&[u8], &[u8], &str)] = &[
    (b"ActiveRecord", b"Base", "active_record"),
    (b"ActionController", b"Base", "action_controller"),
    (b"ActionController", b"API", "action_controller"),
    (b"ActionController", b"TestCase", "action_controller_test_case"),
    (b"ActionView", b"Base", "action_view"),
    (b"ActionMailer", b"Base", "action_mailer"),
    (b"ActiveJob", b"Base", "active_job"),
    (b"ActionCable", b"Channel", "action_cable_channel"),
    (b"ActionCable", b"Connection", "action_cable_connection"),
];

const PATCH_METHODS: &[&[u8]] = &[b"include", b"prepend", b"extend"];

/// Try to match a constant path like `ActiveRecord::Base` or `::ActiveRecord::Base`.
/// Returns the hook name if matched.
fn match_framework_class(node: &ruby_prism::Node<'_>, source: &SourceFile) -> Option<&'static str> {
    // Get the full text of the receiver and match against known patterns
    let loc = node.location();
    let text = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
    // Strip leading ::
    let text = if text.starts_with(b"::") {
        &text[2..]
    } else {
        text
    };

    for &(module_name, class_name, hook) in FRAMEWORK_CLASSES {
        let mut expected = Vec::new();
        expected.extend_from_slice(module_name);
        expected.extend_from_slice(b"::");
        expected.extend_from_slice(class_name);
        if text == expected.as_slice() {
            return Some(hook);
        }
    }
    None
}

impl Cop for ActiveSupportOnLoad {
    fn name(&self) -> &'static str {
        "Rails/ActiveSupportOnLoad"
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
        if !PATCH_METHODS.contains(&method_name) {
            return Vec::new();
        }

        // Must have arguments
        if call.arguments().is_none() {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let hook = match match_framework_class(&receiver, source) {
            Some(h) => h,
            None => return Vec::new(),
        };

        let method_str = std::str::from_utf8(method_name).unwrap_or("include");
        let recv_loc = receiver.location();
        let recv_text = std::str::from_utf8(
            &source.as_bytes()[recv_loc.start_offset()..recv_loc.end_offset()],
        )
        .unwrap_or("FrameworkClass");

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `ActiveSupport.on_load(:{hook}) {{ {method_str} ... }}` instead of `{recv_text}.{method_str}(...)`."
            ),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ActiveSupportOnLoad, "cops/rails/active_support_on_load");
}
