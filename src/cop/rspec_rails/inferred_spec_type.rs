use crate::cop::rspec_rails::RSPEC_RAILS_DEFAULT_INCLUDE;
use crate::cop::shared::node_type::{
    ASSOC_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-24)
///
/// Unified corpus reported FP=1, FN=3.
///
/// FN=3: All from `RSpec.describe type: :model, extra_key: true do` (no described
/// class). RuboCop's `describe_with_type` NodePattern uses `...` which matches
/// zero-or-more args before the hash, so it flags these. Removed the incorrect
/// `has_positional_arg` early return that was skipping no-described-class patterns.
///
/// FP=1: Caused by `# rubocop:disable RSpec/Rails/InferredSpecType` not being
/// recognized as `RSpecRails/InferredSpecType`. Fixed in directive processing by
/// normalizing 3-part cop names (e.g. `RSpec/Rails/X` -> `RSpecRails/X`).
///
/// ## Corpus investigation (2026-03-25)
///
/// Corpus oracle reported FP=13, FN=0.
///
/// FP=13: All 13 are `RSpec.describe type: :model do` (no described class,
/// `type:` is the only pair). RuboCop crashes on these with NoMethodError in
/// `remove_range`: `node.left_sibling.source_range` fails because
/// `left_sibling` returns the method name Symbol (`:describe`) instead of an
/// AST node when the hash is the first argument. RuboCop swallows the error
/// and reports 0 offenses. Fixed by skipping the offense when `type:` is the
/// only pair and there are no positional arguments before the hash.
///
/// Also fixed `infer_type()` falling through to hardcoded `DEFAULT_INFERENCES`
/// when the config already has an `Inferences` key — RuboCop uses
/// `cop_config['Inferences'] || {}` without merging defaults.
pub struct InferredSpecType;

/// Default directory-to-type inferences (matching RuboCop's defaults).
const DEFAULT_INFERENCES: &[(&str, &str)] = &[
    ("channels", "channel"),
    ("controllers", "controller"),
    ("features", "feature"),
    ("generator", "generator"),
    ("helpers", "helper"),
    ("jobs", "job"),
    ("mailboxes", "mailbox"),
    ("mailers", "mailer"),
    ("models", "model"),
    ("requests", "request"),
    ("integration", "request"),
    ("api", "request"),
    ("routing", "routing"),
    ("system", "system"),
    ("views", "view"),
];

/// Example group methods that can have type metadata.
const EXAMPLE_GROUPS: &[&[u8]] = &[
    b"describe",
    b"context",
    b"feature",
    b"example_group",
    b"xdescribe",
    b"xcontext",
    b"xfeature",
    b"xexample_group",
];

impl Cop for InferredSpecType {
    fn name(&self) -> &'static str {
        "RSpecRails/InferredSpecType"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_RAILS_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ASSOC_NODE,
            CALL_NODE,
            HASH_NODE,
            KEYWORD_HASH_NODE,
            SYMBOL_NODE,
        ]
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

        // Check for RSpec.describe/context/feature/example_group or bare calls.
        let is_example_group = if let Some(recv) = call.receiver() {
            crate::cop::shared::util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                && EXAMPLE_GROUPS.contains(&method_name)
        } else {
            EXAMPLE_GROUPS.contains(&method_name)
        };

        if !is_example_group {
            return;
        }

        // Must have a block
        if call.block().is_none() {
            return;
        }

        // Look for `type:` keyword argument in the arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();

        // Find a hash argument containing `type: :something`
        for (i, arg) in arg_list.iter().enumerate() {
            let has_positional_before = i > 0;
            if let Some(diag) = self.check_hash_arg(source, arg, config, has_positional_before) {
                diagnostics.push(diag);
            }
        }
    }
}

impl InferredSpecType {
    fn check_hash_arg(
        &self,
        source: &SourceFile,
        arg: &ruby_prism::Node<'_>,
        config: &CopConfig,
        has_positional_before: bool,
    ) -> Option<Diagnostic> {
        if let Some(hash) = arg.as_hash_node() {
            return self.check_pairs(source, arg, &hash.elements(), config, has_positional_before);
        }
        if let Some(kw_hash) = arg.as_keyword_hash_node() {
            return self.check_pairs(
                source,
                arg,
                &kw_hash.elements(),
                config,
                has_positional_before,
            );
        }
        None
    }

    fn check_pairs(
        &self,
        source: &SourceFile,
        hash_arg: &ruby_prism::Node<'_>,
        pairs: &ruby_prism::NodeList<'_>,
        config: &CopConfig,
        has_positional_before: bool,
    ) -> Option<Diagnostic> {
        for element in pairs.iter() {
            let assoc = match element.as_assoc_node() {
                Some(a) => a,
                None => continue,
            };

            // Check if key is :type or `type:`
            let is_type_key = if let Some(sym) = assoc.key().as_symbol_node() {
                sym.unescaped() == b"type"
            } else {
                false
            };

            if !is_type_key {
                continue;
            }

            // Get the value as a symbol
            let type_sym = match assoc.value().as_symbol_node() {
                Some(s) => s,
                None => continue,
            };

            let type_name = type_sym.unescaped();
            let type_str = std::str::from_utf8(type_name).unwrap_or("");

            // Infer expected type from file path
            let file_path = source.path_str();
            let inferred = self.infer_type(file_path, config);

            if let Some(inferred_type) = inferred {
                if inferred_type == type_str {
                    let only_pair = self.is_only_pair(pairs);

                    // RuboCop bug: when `type:` is the only pair and the hash
                    // is the first argument (no described class), the autocorrect
                    // crashes with NoMethodError on `left_sibling.source_range`
                    // because `left_sibling` returns the method name Symbol
                    // instead of an AST node. RuboCop swallows the error and
                    // reports 0 offenses. We replicate this by skipping.
                    if only_pair && !has_positional_before {
                        return None;
                    }

                    let loc = if only_pair {
                        hash_arg.location()
                    } else {
                        assoc.location()
                    };
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return Some(self.diagnostic(
                        source,
                        line,
                        column,
                        "Remove redundant spec type.".to_string(),
                    ));
                }
            }
        }
        None
    }

    fn infer_type(&self, file_path: &str, config: &CopConfig) -> Option<String> {
        // Use config Inferences if present; only fall back to defaults when
        // the config key is entirely absent. RuboCop uses
        // `cop_config['Inferences'] || {}` — it never merges with defaults.
        if let Some(inferences) = config.get_string_hash("Inferences") {
            for (prefix, inferred_type) in &inferences {
                let pattern = format!("spec/{prefix}/");
                if file_path.contains(&pattern) {
                    return Some(inferred_type.clone());
                }
            }
            return None;
        }

        // Fall back to defaults only when config doesn't have Inferences key
        for (prefix, inferred_type) in DEFAULT_INFERENCES {
            let pattern = format!("spec/{prefix}/");
            if file_path.contains(&pattern) {
                return Some(inferred_type.to_string());
            }
        }

        None
    }

    fn is_only_pair(&self, pairs: &ruby_prism::NodeList<'_>) -> bool {
        let count = pairs.iter().filter(|e| e.as_assoc_node().is_some()).count();
        count == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InferredSpecType, "cops/rspecrails/inferred_spec_type");

    #[test]
    fn rspec_feature_with_redundant_type() {
        // RSpec.feature with type: :feature in spec/features/ should be flagged
        let source = b"RSpec.feature \"Dashboard\", type: :feature do\nend\n";
        let diags = crate::testutil::run_cop_full_internal(
            &InferredSpecType,
            source,
            crate::cop::CopConfig::default(),
            "spec/features/dashboard_spec.rb",
        );
        assert_eq!(diags.len(), 1, "Expected 1 offense, got {:?}", diags);
        assert_eq!(diags[0].message, "Remove redundant spec type.");
    }

    #[test]
    fn rspec_example_group_with_redundant_type() {
        // RSpec.example_group with type: :model in spec/models/ should be flagged
        let source = b"RSpec.example_group \"User\", type: :model do\nend\n";
        let diags = crate::testutil::run_cop_full_internal(
            &InferredSpecType,
            source,
            crate::cop::CopConfig::default(),
            "spec/models/user_spec.rb",
        );
        assert_eq!(diags.len(), 1, "Expected 1 offense, got {:?}", diags);
        assert_eq!(diags[0].message, "Remove redundant spec type.");
    }

    #[test]
    fn custom_inferences_does_not_fall_through_to_defaults() {
        // When config has a custom Inferences hash that does NOT include "models",
        // a file in spec/models/ with type: :model should NOT be flagged.
        // RuboCop uses cop_config['Inferences'] || {} without merging defaults.
        let source = b"RSpec.describe User, type: :model do\nend\n";
        let mut inferences = serde_yml::Mapping::new();
        inferences.insert(
            serde_yml::Value::String("services".into()),
            serde_yml::Value::String("service".into()),
        );
        let mut options = std::collections::HashMap::new();
        options.insert(
            "Inferences".to_string(),
            serde_yml::Value::Mapping(inferences),
        );
        let config = crate::cop::CopConfig {
            options,
            ..crate::cop::CopConfig::default()
        };
        let diags = crate::testutil::run_cop_full_internal(
            &InferredSpecType,
            source,
            config,
            "spec/models/user_spec.rb",
        );
        assert!(
            diags.is_empty(),
            "Should not flag when custom Inferences doesn't include 'models', got {:?}",
            diags
        );
    }
}
