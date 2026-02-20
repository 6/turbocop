pub mod bundler;
pub mod factory_bot;
pub mod gemspec;
pub mod layout;
pub mod migration;
pub mod lint;
pub mod metrics;
pub mod naming;
pub mod performance;
pub mod rails;
pub mod registry;
pub mod rspec;
pub mod rspec_rails;
pub mod security;
pub mod style;
pub mod node_type;
pub mod util;
pub mod walker;

use std::collections::HashMap;

use serde::Serialize;

use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Tri-state for cop Enabled field, matching RuboCop semantics.
///
/// - `True` / `False` — explicitly set in config
/// - `Pending` — set by plugin defaults (e.g. `rubocop-rails`); disabled
///   unless `AllCops.NewCops: enable`
/// - `Unset` — no explicit setting; inherits from defaults (enabled unless
///   `AllCops.DisabledByDefault: true`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum EnabledState {
    True,
    False,
    Pending,
    #[default]
    Unset,
}

/// Per-cop configuration extracted from .rubocop.yml.
#[derive(Debug, Clone, Serialize)]
pub struct CopConfig {
    pub enabled: EnabledState,
    pub severity: Option<Severity>,
    pub exclude: Vec<String>,
    pub include: Vec<String>,
    pub options: HashMap<String, serde_yml::Value>,
}

impl Default for CopConfig {
    fn default() -> Self {
        Self {
            enabled: EnabledState::Unset,
            severity: None,
            exclude: Vec::new(),
            include: Vec::new(),
            options: HashMap::new(),
        }
    }
}

impl CopConfig {
    /// Get a string option with a default value.
    pub fn get_str<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.options
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or(default)
    }

    /// Get an unsigned integer option with a default value.
    pub fn get_usize(&self, key: &str, default: usize) -> usize {
        self.options
            .get(key)
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(default)
    }

    /// Get a boolean option with a default value.
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.options
            .get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(default)
    }

    /// Get a string array option. Returns None if the key is absent.
    pub fn get_string_array(&self, key: &str) -> Option<Vec<String>> {
        self.options.get(key).and_then(|v| {
            v.as_sequence().map(|seq| {
                seq.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect()
            })
        })
    }

    /// Get a string→string hash option. Returns None if the key is absent.
    pub fn get_string_hash(&self, key: &str) -> Option<HashMap<String, String>> {
        self.options.get(key).and_then(|v| {
            v.as_mapping().map(|m| {
                m.iter()
                    .filter_map(|(k, v)| {
                        let ks = k.as_str()?;
                        let vs = v.as_str()?;
                        Some((ks.to_string(), vs.to_string()))
                    })
                    .collect()
            })
        })
    }

    /// Get all string values from a config key that is either:
    /// - A flat array of strings
    /// - A hash of group_name → array of strings (like DebuggerMethods)
    /// Returns the flattened list of all strings. None if the key is absent.
    pub fn get_flat_string_values(&self, key: &str) -> Option<Vec<String>> {
        let v = self.options.get(key)?;
        let mut result = Vec::new();
        if let Some(mapping) = v.as_mapping() {
            for (_, group_val) in mapping.iter() {
                if let Some(seq) = group_val.as_sequence() {
                    for item in seq {
                        if let Some(s) = item.as_str() {
                            result.push(s.to_string());
                        }
                    }
                }
                if let Some(s) = group_val.as_str() {
                    result.push(s.to_string());
                }
            }
        }
        if let Some(seq) = v.as_sequence() {
            for item in seq {
                if let Some(s) = item.as_str() {
                    result.push(s.to_string());
                }
            }
        }
        if result.is_empty() { None } else { Some(result) }
    }

    /// Whether the cop itself is considered safe (default: true).
    pub fn is_safe(&self) -> bool {
        self.get_bool("Safe", true)
    }

    /// Whether the cop's autocorrect is considered safe (default: true).
    pub fn is_safe_autocorrect(&self) -> bool {
        self.get_bool("SafeAutoCorrect", true)
    }

    /// Read the `AutoCorrect` config key. RuboCop supports both boolean and
    /// string values ("always", "contextual", "disabled"). Default: "always".
    pub fn autocorrect_setting(&self) -> &str {
        if let Some(v) = self.options.get("AutoCorrect") {
            // Boolean true -> "always", false -> "disabled"
            if let Some(b) = v.as_bool() {
                return if b { "always" } else { "disabled" };
            }
            if let Some(s) = v.as_str() {
                return s;
            }
        }
        "always"
    }

    /// Whether this cop should autocorrect given the current CLI mode.
    pub fn should_autocorrect(&self, mode: crate::cli::AutocorrectMode) -> bool {
        use crate::cli::AutocorrectMode;
        match mode {
            AutocorrectMode::Off => false,
            AutocorrectMode::Safe => {
                self.is_safe() && self.is_safe_autocorrect() && self.autocorrect_setting() != "disabled"
            }
            AutocorrectMode::All => {
                self.autocorrect_setting() != "disabled"
            }
        }
    }
}

/// A lint rule. Implementations must be Send + Sync so they can be shared
/// across rayon worker threads.
pub trait Cop: Send + Sync {
    /// The fully-qualified cop name, e.g. "Style/FrozenStringLiteralComment".
    fn name(&self) -> &'static str;

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    /// Default Include patterns for this cop. If non-empty, the cop only runs
    /// on files matching at least one pattern. User config overrides these.
    fn default_include(&self) -> &'static [&'static str] {
        &[]
    }

    /// Default Exclude patterns for this cop. If non-empty, the cop is skipped
    /// on files matching any pattern. User config overrides these.
    fn default_exclude(&self) -> &'static [&'static str] {
        &[]
    }

    /// Whether the cop is enabled by default.
    ///
    /// Matches the `Enabled` value from vendor `config/default.yml`.
    /// Cops that have `Enabled: false` in the vendor config should override
    /// this to return `false`. This ensures they stay disabled even when no
    /// `.rubocop.yml` is present (and vendor defaults are not loaded).
    fn default_enabled(&self) -> bool {
        true
    }

    /// Create a Diagnostic with standard fields filled in.
    fn diagnostic(
        &self,
        source: &SourceFile,
        line: usize,
        column: usize,
        message: String,
    ) -> Diagnostic {
        Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message,
            corrected: false,
        }
    }

    /// Whether this cop can produce autocorrections.
    fn supports_autocorrect(&self) -> bool {
        false
    }

    /// Whether this cop's autocorrections are safe (won't change semantics).
    fn safe_autocorrect(&self) -> bool {
        true
    }

    /// Line-based check — runs before AST traversal.
    #[allow(unused_variables)]
    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
    }

    /// Source-based check — runs once per file with full parse context and CodeMap.
    ///
    /// Use this for cops that scan raw source bytes while needing to skip
    /// non-code regions (strings, comments, regexps).
    #[allow(unused_variables)]
    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
    }

    /// Node types this cop handles in `check_node`.
    /// Return a non-empty slice to opt into selective dispatch (only called for
    /// matching node types). Return `&[]` to be called for every node (default).
    fn interested_node_types(&self) -> &'static [u8] {
        &[]
    }

    /// Node-based check — called for every AST node during traversal.
    #[allow(unused_variables)]
    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
    }
}

/// Generate standard offense/no_offense fixture tests for a cop.
///
/// Usage:
/// ```ignore
/// #[cfg(test)]
/// mod tests {
///     use super::*;
///     crate::cop_fixture_tests!(CopStruct, "cops/dept/cop_name");
///     // additional tests...
/// }
/// ```
#[macro_export]
macro_rules! cop_fixture_tests {
    ($cop:expr, $path:literal) => {
        #[test]
        fn offense_fixture() {
            $crate::testutil::assert_cop_offenses_full(
                &$cop,
                include_bytes!(concat!("../../../testdata/", $path, "/offense.rb")),
            );
        }

        #[test]
        fn no_offense_fixture() {
            $crate::testutil::assert_cop_no_offenses_full(
                &$cop,
                include_bytes!(concat!("../../../testdata/", $path, "/no_offense.rb")),
            );
        }
    };
}

/// Generate scenario-based fixture tests for cops that need multiple offense files.
///
/// Use when a cop fires at most once per file (e.g., InitialIndentation,
/// LeadingEmptyLines) or when offenses can't be annotated with `^` markers
/// (e.g., TrailingEmptyLines). Each scenario file is a separate `.rb` file
/// in an `offense/` directory.
///
/// Usage:
/// ```ignore
/// #[cfg(test)]
/// mod tests {
///     use super::*;
///     crate::cop_scenario_fixture_tests!(
///         CopStruct, "cops/dept/cop_name",
///         scenario_one = "scenario_one.rb",
///         scenario_two = "scenario_two.rb",
///     );
/// }
/// ```
#[macro_export]
macro_rules! cop_scenario_fixture_tests {
    ($cop:expr, $path:literal, $($name:ident = $file:literal),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                $crate::testutil::assert_cop_offenses_full(
                    &$cop,
                    include_bytes!(concat!("../../../testdata/", $path, "/offense/", $file)),
                );
            }
        )+

        #[test]
        fn no_offense_fixture() {
            $crate::testutil::assert_cop_no_offenses_full(
                &$cop,
                include_bytes!(concat!("../../../testdata/", $path, "/no_offense.rb")),
            );
        }
    };
}

/// Generate autocorrect fixture tests for a cop.
///
/// If `testdata/<path>/corrected.rb` exists, this generates a test that:
/// 1. Strips annotations from `offense.rb` to get the input source
/// 2. Runs the cop with corrections enabled
/// 3. Applies corrections to produce corrected source
/// 4. Asserts the output matches `corrected.rb` byte-for-byte
///
/// Usage:
/// ```ignore
/// #[cfg(test)]
/// mod tests {
///     use super::*;
///     crate::cop_fixture_tests!(CopStruct, "cops/dept/cop_name");
///     crate::cop_autocorrect_fixture_tests!(CopStruct, "cops/dept/cop_name");
/// }
/// ```
#[macro_export]
macro_rules! cop_autocorrect_fixture_tests {
    ($cop:expr, $path:literal) => {
        #[test]
        fn autocorrect_fixture() {
            $crate::testutil::assert_cop_autocorrect(
                &$cop,
                include_bytes!(concat!("../../../testdata/", $path, "/offense.rb")),
                include_bytes!(concat!("../../../testdata/", $path, "/corrected.rb")),
            );
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;
        use serde_yml::Value;

        fn config_with(options: HashMap<String, Value>) -> CopConfig {
            CopConfig {
                options,
                ..CopConfig::default()
            }
        }

        /// Strategy for serde_yml::Value that exercises the type branches.
        fn yaml_value_strategy() -> impl Strategy<Value = Value> {
            prop_oneof![
                any::<bool>().prop_map(Value::Bool),
                any::<u64>().prop_map(|n| Value::Number(serde_yml::Number::from(n))),
                "[a-z]{0,20}".prop_map(Value::String),
                Just(Value::Null),
                prop::collection::vec("[a-z]{1,10}".prop_map(Value::String), 0..5)
                    .prop_map(Value::Sequence),
            ]
        }

        proptest! {
            #[test]
            fn get_str_missing_key_returns_default(default in "[a-z]{1,10}") {
                let cfg = config_with(HashMap::new());
                prop_assert_eq!(cfg.get_str("NoSuchKey", &default), default.as_str());
            }

            #[test]
            fn get_str_present_string(key in "[A-Z][a-z]{1,8}", val in "[a-z]{1,20}") {
                let cfg = config_with(HashMap::from([
                    (key.clone(), Value::String(val.clone())),
                ]));
                prop_assert_eq!(cfg.get_str(&key, "fallback"), val.as_str());
            }

            #[test]
            fn get_str_wrong_type_returns_default(key in "[A-Z][a-z]{1,8}", n in any::<u64>()) {
                let cfg = config_with(HashMap::from([
                    (key.clone(), Value::Number(serde_yml::Number::from(n))),
                ]));
                prop_assert_eq!(cfg.get_str(&key, "default"), "default");
            }

            #[test]
            fn get_usize_missing_key_returns_default(default in any::<usize>()) {
                let cfg = config_with(HashMap::new());
                prop_assert_eq!(cfg.get_usize("NoSuchKey", default), default);
            }

            #[test]
            fn get_usize_present_number(key in "[A-Z][a-z]{1,8}", n in 0u64..1_000_000) {
                let cfg = config_with(HashMap::from([
                    (key.clone(), Value::Number(serde_yml::Number::from(n))),
                ]));
                prop_assert_eq!(cfg.get_usize(&key, 999), n as usize);
            }

            #[test]
            fn get_usize_wrong_type_returns_default(key in "[A-Z][a-z]{1,8}", s in "[a-z]{1,10}") {
                let cfg = config_with(HashMap::from([
                    (key.clone(), Value::String(s)),
                ]));
                prop_assert_eq!(cfg.get_usize(&key, 42), 42);
            }

            #[test]
            fn get_bool_missing_key_returns_default(default in any::<bool>()) {
                let cfg = config_with(HashMap::new());
                prop_assert_eq!(cfg.get_bool("NoSuchKey", default), default);
            }

            #[test]
            fn get_bool_present_bool(key in "[A-Z][a-z]{1,8}", val in any::<bool>()) {
                let cfg = config_with(HashMap::from([
                    (key.clone(), Value::Bool(val)),
                ]));
                prop_assert_eq!(cfg.get_bool(&key, !val), val);
            }

            #[test]
            fn get_bool_wrong_type_returns_default(key in "[A-Z][a-z]{1,8}", s in "[a-z]{1,10}") {
                let cfg = config_with(HashMap::from([
                    (key.clone(), Value::String(s)),
                ]));
                prop_assert_eq!(cfg.get_bool(&key, true), true);
            }

            #[test]
            fn get_string_array_missing_returns_none(key in "[A-Z][a-z]{1,8}") {
                let cfg = config_with(HashMap::new());
                prop_assert!(cfg.get_string_array(&key).is_none());
            }

            #[test]
            fn get_string_array_present(
                key in "[A-Z][a-z]{1,8}",
                items in prop::collection::vec("[a-z]{1,10}", 0..10),
            ) {
                let seq: Vec<Value> = items.iter().map(|s| Value::String(s.clone())).collect();
                let cfg = config_with(HashMap::from([
                    (key.clone(), Value::Sequence(seq)),
                ]));
                let result = cfg.get_string_array(&key).unwrap();
                prop_assert_eq!(result, items);
            }

            #[test]
            fn get_string_array_filters_non_strings(key in "[A-Z][a-z]{1,8}") {
                let seq = vec![
                    Value::String("keep".to_string()),
                    Value::Bool(true),
                    Value::Number(serde_yml::Number::from(42u64)),
                    Value::String("also_keep".to_string()),
                ];
                let cfg = config_with(HashMap::from([
                    (key.clone(), Value::Sequence(seq)),
                ]));
                let result = cfg.get_string_array(&key).unwrap();
                prop_assert_eq!(result, vec!["keep".to_string(), "also_keep".to_string()]);
            }

            #[test]
            fn no_panic_on_arbitrary_values(
                key in "[A-Z][a-z]{1,8}",
                val in yaml_value_strategy(),
            ) {
                let cfg = config_with(HashMap::from([(key.clone(), val)]));
                // None of these should panic regardless of value type
                let _ = cfg.get_str(&key, "default");
                let _ = cfg.get_usize(&key, 0);
                let _ = cfg.get_bool(&key, false);
                let _ = cfg.get_string_array(&key);
                let _ = cfg.get_string_hash(&key);
                let _ = cfg.get_flat_string_values(&key);
            }
        }
    }
}
