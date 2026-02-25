use crate::cop::node_type::CLASS_NODE;
use crate::cop::util::{full_constant_path, parent_class_name};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ApplicationRecord;

impl Cop for ApplicationRecord {
    fn name(&self) -> &'static str {
        "Rails/ApplicationRecord"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_exclude(&self) -> &'static [&'static str] {
        &["db/**/*.rb"]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE]
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
        // minimum_target_rails_version 5.0
        if !config.rails_version_at_least(5.0) {
            return;
        }

        let class = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        // RuboCop's pattern: (const _ !:ApplicationRecord) checks the constant's
        // own name, not the full path. So Admin::ApplicationRecord has name
        // :ApplicationRecord and should NOT be flagged.
        let class_name = full_constant_path(source, &class.constant_path());
        if class_name == b"ApplicationRecord" || class_name.ends_with(b"::ApplicationRecord") {
            return;
        }

        let parent = match parent_class_name(source, &class) {
            Some(p) => p,
            None => return,
        };

        // Handle both ActiveRecord::Base and ::ActiveRecord::Base
        let parent_trimmed = if parent.starts_with(b"::") {
            &parent[2..]
        } else {
            parent
        };
        if parent_trimmed == b"ActiveRecord::Base" {
            let loc = class.class_keyword_loc();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Models should subclass `ApplicationRecord`.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config_with_rails(version: f64) -> CopConfig {
        let mut options = HashMap::new();
        options.insert(
            "TargetRailsVersion".to_string(),
            serde_yml::Value::Number(serde_yml::Number::from(version)),
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

    #[test]
    fn offense_fixture() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &ApplicationRecord,
            include_bytes!("../../../tests/fixtures/cops/rails/application_record/offense.rb"),
            config_with_rails(5.0),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &ApplicationRecord,
            include_bytes!("../../../tests/fixtures/cops/rails/application_record/no_offense.rb"),
            config_with_rails(5.0),
        );
    }

    #[test]
    fn skipped_when_no_target_rails_version() {
        let source = b"class User < ActiveRecord::Base\nend\n";
        let diagnostics = crate::testutil::run_cop_full_internal(
            &ApplicationRecord,
            source,
            CopConfig::default(),
            "test.rb",
        );
        assert!(
            diagnostics.is_empty(),
            "Should not fire when TargetRailsVersion is not set (non-Rails project)"
        );
    }
}
