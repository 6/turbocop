use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ActiveSupportOnLoad;

/// Complete map of Rails framework classes to their on_load hook names.
/// Includes all entries from RuboCop's LOAD_HOOKS, RAILS_5_2_LOAD_HOOKS,
/// and RAILS_7_1_LOAD_HOOKS maps.
const LOAD_HOOKS: &[(&str, &str)] = &[
    // LOAD_HOOKS (base)
    ("ActionCable", "action_cable"),
    ("ActionCable::Channel::Base", "action_cable_channel"),
    ("ActionCable::Connection::Base", "action_cable_connection"),
    (
        "ActionCable::Connection::TestCase",
        "action_cable_connection_test_case",
    ),
    ("ActionController::API", "action_controller"),
    ("ActionController::Base", "action_controller"),
    ("ActionController::TestCase", "action_controller_test_case"),
    (
        "ActionDispatch::IntegrationTest",
        "action_dispatch_integration_test",
    ),
    ("ActionDispatch::Request", "action_dispatch_request"),
    ("ActionDispatch::Response", "action_dispatch_response"),
    (
        "ActionDispatch::SystemTestCase",
        "action_dispatch_system_test_case",
    ),
    ("ActionMailbox::Base", "action_mailbox"),
    (
        "ActionMailbox::InboundEmail",
        "action_mailbox_inbound_email",
    ),
    ("ActionMailbox::Record", "action_mailbox_record"),
    ("ActionMailbox::TestCase", "action_mailbox_test_case"),
    ("ActionMailer::Base", "action_mailer"),
    ("ActionMailer::TestCase", "action_mailer_test_case"),
    ("ActionText::Content", "action_text_content"),
    ("ActionText::Record", "action_text_record"),
    ("ActionText::RichText", "action_text_rich_text"),
    ("ActionView::Base", "action_view"),
    ("ActionView::TestCase", "action_view_test_case"),
    ("ActiveJob::Base", "active_job"),
    ("ActiveJob::TestCase", "active_job_test_case"),
    ("ActiveRecord::Base", "active_record"),
    ("ActiveStorage::Attachment", "active_storage_attachment"),
    ("ActiveStorage::Blob", "active_storage_blob"),
    ("ActiveStorage::Record", "active_storage_record"),
    (
        "ActiveStorage::VariantRecord",
        "active_storage_variant_record",
    ),
    ("ActiveSupport::TestCase", "active_support_test_case"),
    // RAILS_5_2_LOAD_HOOKS
    (
        "ActiveRecord::ConnectionAdapters::SQLite3Adapter",
        "active_record_sqlite3adapter",
    ),
    // RAILS_7_1_LOAD_HOOKS
    ("ActiveRecord::TestFixtures", "active_record_fixtures"),
    ("ActiveModel::Model", "active_model"),
    (
        "ActionText::EncryptedRichText",
        "action_text_encrypted_rich_text",
    ),
    (
        "ActiveRecord::ConnectionAdapters::PostgreSQLAdapter",
        "active_record_postgresqladapter",
    ),
    (
        "ActiveRecord::ConnectionAdapters::Mysql2Adapter",
        "active_record_mysql2adapter",
    ),
    (
        "ActiveRecord::ConnectionAdapters::TrilogyAdapter",
        "active_record_trilogyadapter",
    ),
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

    for &(constant_path, hook) in LOAD_HOOKS {
        if text == constant_path.as_bytes() {
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
        if !PATCH_METHODS.contains(&method_name) {
            return;
        }

        // Must have arguments
        if call.arguments().is_none() {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let hook = match match_framework_class(&receiver, source) {
            Some(h) => h,
            None => return,
        };

        let method_str = std::str::from_utf8(method_name).unwrap_or("include");
        let recv_loc = receiver.location();
        let recv_text = source.byte_slice(
            recv_loc.start_offset(),
            recv_loc.end_offset(),
            "FrameworkClass",
        );

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `ActiveSupport.on_load(:{hook}) {{ {method_str} ... }}` instead of `{recv_text}.{method_str}(...)`."
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ActiveSupportOnLoad, "cops/rails/active_support_on_load");
}
