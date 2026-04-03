use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for Rails framework classes that are patched directly instead of
/// using Active Support load hooks. Direct patching forcibly loads the
/// framework referenced; using hooks defers loading until it's actually needed.
///
/// ## Investigation findings (2026-03-16)
///
/// Root cause of 8 FPs: nitrocop was matching ALL LOAD_HOOKS entries regardless
/// of `TargetRailsVersion`, but RuboCop version-gates `RAILS_5_2_LOAD_HOOKS`
/// (requires >= 5.2) and `RAILS_7_1_LOAD_HOOKS` (requires >= 7.1). All 8 FPs
/// involved RAILS_7_1 hooks (`ActiveRecord::TestFixtures`, `ActiveModel::Model`,
/// `PostgreSQLAdapter`, `TrilogyAdapter`) in projects targeting Rails < 7.1.
///
/// Fix: split hooks into three tiers and check `config.rails_version_at_least()`
/// before matching 5.2 and 7.1 hooks, matching RuboCop's `hook_for_const`.
///
/// ## Investigation findings (2026-03-18)
///
/// Root cause of 5 FNs: all were `SQLite3Adapter.prepend(...)` in repos without
/// `railties` in their Gemfile.lock (fizzy, solid_queue, neighbor). The version
/// check used `config.rails_version_at_least(5.2)` which requires BOTH
/// `TargetRailsVersion >= 5.2` AND `railties` in lockfile. But RuboCop's
/// `ActiveSupportOnLoad` does NOT use `requires_gem 'railties'` — it only checks
/// `target_rails_version >= 5.2` in `hook_for_const`. Fix: use
/// `config.target_rails_version()` directly for the version-gated hook tiers,
/// bypassing the railties lockfile gate.
pub struct ActiveSupportOnLoad;

/// Base LOAD_HOOKS — available at all Rails versions.
const BASE_LOAD_HOOKS: &[(&str, &str)] = &[
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
];

/// RAILS_5_2_LOAD_HOOKS — only active when TargetRailsVersion >= 5.2.
const RAILS_5_2_LOAD_HOOKS: &[(&str, &str)] = &[(
    "ActiveRecord::ConnectionAdapters::SQLite3Adapter",
    "active_record_sqlite3adapter",
)];

/// RAILS_7_1_LOAD_HOOKS — only active when TargetRailsVersion >= 7.1.
const RAILS_7_1_LOAD_HOOKS: &[(&str, &str)] = &[
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
/// Returns the hook name if matched, respecting version-gated hook tiers.
fn match_framework_class(
    node: &ruby_prism::Node<'_>,
    source: &SourceFile,
    config: &CopConfig,
) -> Option<&'static str> {
    let loc = node.location();
    let text = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
    // Strip leading ::
    let text = if text.starts_with(b"::") {
        &text[2..]
    } else {
        text
    };

    // Base hooks — always active
    for &(constant_path, hook) in BASE_LOAD_HOOKS {
        if text == constant_path.as_bytes() {
            return Some(hook);
        }
    }

    // Rails 5.2+ hooks — use target_rails_version() directly instead of
    // rails_version_at_least() because RuboCop's ActiveSupportOnLoad does NOT
    // use `requires_gem 'railties'`. It only checks `target_rails_version >= 5.2`
    // in hook_for_const, so the railties lockfile gate must be skipped here.
    if config.target_rails_version().is_some_and(|v| v >= 5.2) {
        for &(constant_path, hook) in RAILS_5_2_LOAD_HOOKS {
            if text == constant_path.as_bytes() {
                return Some(hook);
            }
        }
    }

    // Rails 7.1+ hooks — same rationale as above
    if config.target_rails_version().is_some_and(|v| v >= 7.1) {
        for &(constant_path, hook) in RAILS_7_1_LOAD_HOOKS {
            if text == constant_path.as_bytes() {
                return Some(hook);
            }
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
        config: &CopConfig,
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

        let hook = match match_framework_class(&receiver, source, config) {
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

    fn rails_config(version: f64) -> CopConfig {
        let mut options = std::collections::HashMap::new();
        options.insert(
            "TargetRailsVersion".to_string(),
            serde_yml::Value::Number(serde_yml::value::Number::from(version)),
        );
        options.insert(
            "__RailtiesInLockfile".to_string(),
            serde_yml::Value::Bool(true),
        );
        CopConfig {
            options,
            ..CopConfig::default()
        }
    }

    /// RAILS_7_1_LOAD_HOOKS should fire when TargetRailsVersion >= 7.1.
    #[test]
    fn rails_7_1_hooks_with_version_set() {
        let source = b"ActiveModel::Model.include(MyModule)\n\
                        ActiveRecord::TestFixtures.prepend(TestFixtures)\n\
                        ActiveRecord::ConnectionAdapters::PostgreSQLAdapter.prepend(PgExt)\n";
        let diags = crate::testutil::run_cop_full_with_config(
            &ActiveSupportOnLoad,
            source,
            rails_config(7.1),
        );
        assert_eq!(
            diags.len(),
            3,
            "Expected 3 offenses for RAILS_7_1 hooks with version >= 7.1"
        );
    }

    /// RAILS_7_1_LOAD_HOOKS should NOT fire when TargetRailsVersion < 7.1.
    #[test]
    fn rails_7_1_hooks_not_flagged_below_7_1() {
        let source = b"ActiveModel::Model.include(MyModule)\n\
                        ActiveRecord::TestFixtures.prepend(TestFixtures)\n";
        let diags = crate::testutil::run_cop_full_with_config(
            &ActiveSupportOnLoad,
            source,
            rails_config(7.0),
        );
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses for RAILS_7_1 hooks with version < 7.1"
        );
    }

    /// RAILS_5_2_LOAD_HOOKS should fire when TargetRailsVersion >= 5.2.
    #[test]
    fn rails_5_2_hooks_with_version_set() {
        let source = b"ActiveRecord::ConnectionAdapters::SQLite3Adapter.include(MyClass)\n";
        let diags = crate::testutil::run_cop_full_with_config(
            &ActiveSupportOnLoad,
            source,
            rails_config(5.2),
        );
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for RAILS_5_2 hook with version >= 5.2"
        );
    }

    /// RAILS_5_2_LOAD_HOOKS should NOT fire when TargetRailsVersion < 5.2.
    #[test]
    fn rails_5_2_hooks_not_flagged_below_5_2() {
        let source = b"ActiveRecord::ConnectionAdapters::SQLite3Adapter.include(MyClass)\n";
        let diags = crate::testutil::run_cop_full_with_config(
            &ActiveSupportOnLoad,
            source,
            rails_config(5.0),
        );
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses for RAILS_5_2 hooks with version < 5.2"
        );
    }

    /// SQLite3Adapter.prepend inside an on_load block should still be flagged.
    /// This is the FN pattern reported in the corpus.
    #[test]
    fn sqlite3_inside_on_load_block_is_flagged() {
        let source = b"ActiveSupport.on_load(:active_record_sqlite3adapter) do\n  \
ActiveRecord::ConnectionAdapters::SQLite3Adapter.prepend(SqliteUuidAdapter)\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(
            &ActiveSupportOnLoad,
            source,
            rails_config(7.0),
        );
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for SQLite3Adapter.prepend inside on_load block"
        );
    }

    /// RAILS_5_2_LOAD_HOOKS should fire even without railties in lockfile.
    /// RuboCop's ActiveSupportOnLoad does NOT use `requires_gem 'railties'`,
    /// it only checks `target_rails_version >= 5.2` in `hook_for_const`.
    #[test]
    fn rails_5_2_hooks_fire_without_railties() {
        let mut options = std::collections::HashMap::new();
        options.insert(
            "TargetRailsVersion".to_string(),
            serde_yml::Value::Number(serde_yml::value::Number::from(7.0)),
        );
        // No __RailtiesInLockfile — simulates projects without railties in lockfile
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        let source =
            b"ActiveRecord::ConnectionAdapters::SQLite3Adapter.prepend(SqliteUuidAdapter)\n";
        let diags = crate::testutil::run_cop_full_with_config(&ActiveSupportOnLoad, source, config);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for SQLite3Adapter.prepend without railties in lockfile"
        );
    }
}
