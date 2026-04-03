use ruby_prism::Visit;

use crate::cop::shared::node_type::{
    CLASS_NODE, CONSTANT_OR_WRITE_NODE, CONSTANT_PATH_OR_WRITE_NODE, CONSTANT_PATH_WRITE_NODE,
    CONSTANT_WRITE_NODE, MODULE_NODE, MULTI_WRITE_NODE, STATEMENTS_NODE,
};
use crate::cop::shared::util::{
    collect_foldable_ranges, count_body_lines_ex, count_body_lines_full, inner_classlike_ranges,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-04)
///
/// Artifact data for this cop remains FN-heavy with low FP. The dominant FN
/// examples are top-level `class << self` blocks that RuboCop checks in
/// `on_sclass` (except when nested under a real `class` ancestor).
///
/// Previous broad rewrite attempts regressed badly, so this implementation is
/// intentionally incremental and validated against per-repo reruns:
/// - `origin/main` implementation rerun: actual 14,382 vs expected 14,177.
/// - Singleton-class step rerun: actual 14,494 vs expected 14,177.
/// - Delta vs `origin/main`: +112 offenses, decomposed into:
///   - ~109 recovered missing offenses (mostly singleton-class FNs)
///   - ~3 additional non-noise offenses (fastlane, sentry-ruby, mongoid)
///   - large `jruby` file-drop noise dominates aggregate counts in both runs.
/// - Assignment-constructor step (`Class.new`/`Struct.new`) rerun:
///   - actual 14,498 vs expected 14,177
///   - net delta vs singleton-class step: +4 offenses, decomposed into:
///     - +3 recovered missing offenses
///     - +1 additional non-noise offense (community/community)
///
/// Follow-up work should verify edge-cases around all assignment forms from the
/// upstream spec matrix (e.g., constant-path writes and mixed multi-targets).
///
/// ## Corpus investigation (2026-03-10)
///
/// FP=1 in fastlane__fastlane (match/lib/match/nuke.rb:17). Root cause: the
/// source file contains two `# rubocop:disable Metrics/ClassLength` comments
/// (lines 16 and 400) with no intervening `# rubocop:enable`. The second
/// `disable` overwrote the first in `DisabledRanges::open_disables`, so only
/// lines 400+ were covered. Fix applied in `src/parse/directives.rs`: when a
/// new block disable is opened for a cop that already has one open, the
/// previous range is closed first before opening the new one.
///
/// ## Extended corpus investigation (2026-03-23)
///
/// Extended corpus (5592 repos) reported FP=6, FN=1. Standard corpus is 0/0.
///
/// FP=6: 4/6 from Tubalr (2) and stackneveroverflow (2) — same cross-cutting
/// vendored gem file issue. 1 from auth0, 1 from noosfero — config/exclusion.
///
/// FN=1 from brixen/poetics (bin/poetics) — extensionless file not discovered
/// by nitrocop. File discovery issue, not cop logic.
///
/// ## Corpus verification (2026-03-25)
///
/// verify_cop_locations.py: FP 0 fixed / 2 remain, FN 100 fixed / 0 remain.
/// All FN verified fixed. Remaining FP=2: auth0 (1, config resolution),
/// noosfero (1, vendored plugin). No cop-level fix needed.
///
/// ## Corpus re-verification (2026-03-27)
///
/// Rechecked the two remaining FP examples against upstream behavior before
/// changing this cop:
/// - `auth0/omniauth-auth0` `lib/omniauth/auth0/jwt_validator.rb` is covered by
///   `# rubocop:disable Metrics/`. RuboCop accepts that exact long class, while
///   still flagging the same long class without the directive. This cop already
///   matches in isolation, so the corpus mismatch is not a `Metrics/ClassLength`
///   detection bug.
/// - `noosfero/noosfero`
///   `vendor/plugins/xss_terminate/lib/html5lib_sanitize.rb` sits under
///   root `.rubocop.yml` `AllCops: Exclude: '**/vendor/**/*'`. RuboCop
///   suppresses the file through path config, not through class-length logic.
/// - `verify_cop_locations.py Metrics/ClassLength` now reports both original
///   CI false-positive locations fixed. The remaining sampled `check_cop.py`
///   delta is additional noosfero-only config/file-selection noise outside the
///   original CI locations, not a new `Metrics/ClassLength` counting change.
///
/// Added a fixture for the department-disable case to guard against future
/// regressions in directive handling. The remaining vendored-path mismatch
/// needs config/file-selection work outside this cop's route.
pub struct ClassLength;

struct LengthSettings<'a> {
    max: usize,
    count_comments: bool,
    count_as_one: Option<&'a Vec<String>>,
}

struct NodeSpan<'pr> {
    start_offset: usize,
    end_offset: usize,
    body: Option<ruby_prism::Node<'pr>>,
}

fn foldable_ranges_for(
    source: &SourceFile,
    body: Option<&ruby_prism::Node<'_>>,
    count_as_one: Option<&Vec<String>>,
) -> Vec<(usize, usize)> {
    let mut foldable_ranges = Vec::new();
    if let Some(cao) = count_as_one {
        if !cao.is_empty() {
            if let Some(body_node) = body {
                foldable_ranges.extend(collect_foldable_ranges(source, body_node, cao));
            }
        }
    }
    foldable_ranges
}

fn check_classlike_length(
    cop: &ClassLength,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    settings: &LengthSettings<'_>,
    span: NodeSpan<'_>,
) {
    let foldable_ranges = foldable_ranges_for(source, span.body.as_ref(), settings.count_as_one);

    let inner_ranges = span
        .body
        .as_ref()
        .map(|b| inner_classlike_ranges(source, b))
        .unwrap_or_default();

    let count = count_body_lines_full(
        source,
        span.start_offset,
        span.end_offset,
        settings.count_comments,
        &foldable_ranges,
        &inner_ranges,
    );

    if count > settings.max {
        let (line, column) = source.offset_to_line_col(span.start_offset);
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Class has too many lines. [{count}/{}]", settings.max),
        ));
    }
}

fn check_non_classlike_length(
    cop: &ClassLength,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    settings: &LengthSettings<'_>,
    span: NodeSpan<'_>,
) {
    let foldable_ranges = foldable_ranges_for(source, span.body.as_ref(), settings.count_as_one);

    // RuboCop handles `sclass` via generic code-length calculation (not the
    // class/module classlike path), so use non-classlike counting here.
    let count = count_body_lines_ex(
        source,
        span.start_offset,
        span.end_offset,
        settings.count_comments,
        &foldable_ranges,
    );

    if count > settings.max {
        let (line, column) = source.offset_to_line_col(span.start_offset);
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Class has too many lines. [{count}/{}]", settings.max),
        ));
    }
}

fn is_top_level_const_named(node: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    if let Some(read) = node.as_constant_read_node() {
        return read.name().as_slice() == name;
    }

    if let Some(path) = node.as_constant_path_node() {
        return path.parent().is_none()
            && path.name().map(|n| n.as_slice() == name).unwrap_or(false);
    }

    false
}

fn is_class_or_struct_constructor(call: &ruby_prism::CallNode<'_>) -> bool {
    if call.name().as_slice() != b"new" {
        return false;
    }

    let receiver = match call.receiver() {
        Some(r) => r,
        None => return false,
    };

    is_top_level_const_named(&receiver, b"Class") || is_top_level_const_named(&receiver, b"Struct")
}

fn assignment_class_constructor_call<'pr>(
    node: &ruby_prism::Node<'pr>,
) -> Option<ruby_prism::CallNode<'pr>> {
    let value = if let Some(n) = node.as_constant_write_node() {
        n.value()
    } else if let Some(n) = node.as_constant_path_write_node() {
        n.value()
    } else if let Some(n) = node.as_constant_or_write_node() {
        n.value()
    } else if let Some(n) = node.as_constant_path_or_write_node() {
        n.value()
    } else if let Some(n) = node.as_multi_write_node() {
        // `Foo, Bar = Struct.new(...) do ... end`
        let has_constant_target = n.lefts().iter().any(|t| {
            t.as_constant_target_node().is_some() || t.as_constant_path_target_node().is_some()
        });
        if !has_constant_target {
            return None;
        }
        n.value()
    } else {
        return None;
    };

    let call = value.as_call_node()?;
    if !is_class_or_struct_constructor(&call) {
        return None;
    }

    let has_block = call.block().and_then(|b| b.as_block_node()).is_some();
    if !has_block {
        return None;
    }

    Some(call)
}

struct SingletonClassLengthVisitor<'a> {
    cop: &'a ClassLength,
    source: &'a SourceFile,
    max: usize,
    count_comments: bool,
    count_as_one: Option<Vec<String>>,
    diagnostics: &'a mut Vec<Diagnostic>,
    class_depth: usize,
}

impl<'pr> Visit<'pr> for SingletonClassLengthVisitor<'_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        self.class_depth += 1;
        ruby_prism::visit_class_node(self, node);
        self.class_depth -= 1;
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        // Match RuboCop's on_sclass: skip singleton classes nested under class.
        if self.class_depth == 0 {
            let settings = LengthSettings {
                max: self.max,
                count_comments: self.count_comments,
                count_as_one: self.count_as_one.as_ref(),
            };
            check_non_classlike_length(
                self.cop,
                self.source,
                self.diagnostics,
                &settings,
                NodeSpan {
                    start_offset: node.class_keyword_loc().start_offset(),
                    end_offset: node.end_keyword_loc().start_offset(),
                    body: node.body(),
                },
            );
        }
        ruby_prism::visit_singleton_class_node(self, node);
    }
}

impl Cop for ClassLength {
    fn name(&self) -> &'static str {
        "Metrics/ClassLength"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CLASS_NODE,
            CONSTANT_OR_WRITE_NODE,
            CONSTANT_PATH_OR_WRITE_NODE,
            CONSTANT_PATH_WRITE_NODE,
            CONSTANT_WRITE_NODE,
            MODULE_NODE,
            MULTI_WRITE_NODE,
            STATEMENTS_NODE,
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
        let max = config.get_usize("Max", 100);
        let count_comments = config.get_bool("CountComments", false);
        let count_as_one = config.get_string_array("CountAsOne");
        let settings = LengthSettings {
            max,
            count_comments,
            count_as_one: count_as_one.as_ref(),
        };

        if let Some(class_node) = node.as_class_node() {
            check_classlike_length(
                self,
                source,
                diagnostics,
                &settings,
                NodeSpan {
                    start_offset: class_node.class_keyword_loc().start_offset(),
                    end_offset: class_node.end_keyword_loc().start_offset(),
                    body: class_node.body(),
                },
            );
            return;
        }

        let Some(call_node) = assignment_class_constructor_call(node) else {
            return;
        };
        let Some(block_node) = call_node.block().and_then(|b| b.as_block_node()) else {
            return;
        };

        check_non_classlike_length(
            self,
            source,
            diagnostics,
            &settings,
            NodeSpan {
                start_offset: call_node.location().start_offset(),
                end_offset: block_node.closing_loc().start_offset(),
                body: block_node.body(),
            },
        );
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = SingletonClassLengthVisitor {
            cop: self,
            source,
            max: config.get_usize("Max", 100),
            count_comments: config.get_bool("CountComments", false),
            count_as_one: config.get_string_array("CountAsOne"),
            diagnostics,
            class_depth: 0,
        };
        visitor.visit(&parse_result.node());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::collections::HashSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    crate::cop_fixture_tests!(ClassLength, "cops/metrics/class_length");

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nitrocop_class_length_{name}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    fn discovered(files: &[PathBuf]) -> crate::fs::DiscoveredFiles {
        crate::fs::DiscoveredFiles {
            files: files.to_vec(),
            explicit: HashSet::new(),
        }
    }

    fn class_length_args() -> crate::cli::Args {
        let mut args = crate::cli::Args::parse_from(["nitrocop"]);
        args.only = vec!["Metrics/ClassLength".to_string()];
        args.format = "text".to_string();
        args
    }

    fn long_class_source(prefix: &str) -> Vec<u8> {
        let mut source = String::from(prefix);
        for i in 1..=101 {
            source.push_str(&format!("  x = {i}\n"));
        }
        source.push_str("end\n");
        source.into_bytes()
    }

    #[test]
    fn config_custom_max() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(3.into()))]),
            ..CopConfig::default()
        };
        // 4 body lines exceeds Max:3
        let source = b"class Foo\n  a = 1\n  b = 2\n  c = 3\n  d = 4\nend\n";
        let diags = run_cop_full_with_config(&ClassLength, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:3 on 4-line class");
        assert!(diags[0].message.contains("[4/3]"));
    }

    #[test]
    fn config_count_as_one_hash() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // With CountAsOne: ["hash"], a multiline hash counts as 1 line
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                (
                    "CountAsOne".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("hash".into())]),
                ),
            ]),
            ..CopConfig::default()
        };
        // Body: a, b, { k: v, \n k2: v2 \n } = 2 + 1 folded = 3 lines
        let source = b"class Foo\n  a = 1\n  b = 2\n  HASH = {\n    k: 1,\n    k2: 2\n  }\nend\n";
        let diags = run_cop_full_with_config(&ClassLength, source, config);
        assert!(
            diags.is_empty(),
            "Should not fire when hash is folded (3/3)"
        );
    }

    #[test]
    fn singleton_class_nested_under_class_is_skipped() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };

        let source = b"class Outer\n  class << self\n    a = 1\n    b = 2\n  end\nend\n";
        let diags = run_cop_full_with_config(&ClassLength, source, config);

        assert_eq!(diags.len(), 1, "Nested singleton class should be skipped");
        assert_eq!(diags[0].location.line, 1);
    }

    #[test]
    fn department_disable_suppresses_long_class_in_full_pipeline() {
        let dir = temp_dir("directive_disable");
        let file = write_file(
            &dir,
            "lib/jwt_validator.rb",
            &long_class_source("# rubocop:disable Metrics/\nclass JWTValidator\n"),
        );

        let config = crate::config::load_config(None, Some(&dir), None).unwrap();
        let registry = crate::cop::registry::CopRegistry::default_registry();
        let tier_map = crate::cop::tiers::TierMap::load();
        let allowlist = crate::cop::autocorrect_allowlist::AutocorrectAllowlist::load();
        let args = class_length_args();
        let result = crate::linter::run_linter(
            &discovered(&[file]),
            &config,
            &registry,
            &args,
            &tier_map,
            &allowlist,
        );

        assert!(
            result.diagnostics.is_empty(),
            "Directive-disabled long class should be suppressed, got: {:?}",
            result
                .diagnostics
                .iter()
                .map(|d| format!("{d}"))
                .collect::<Vec<_>>()
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn vendor_exclude_suppresses_long_class_in_full_pipeline() {
        let dir = temp_dir("vendor_exclude");
        write_file(
            &dir,
            ".rubocop.yml",
            b"AllCops:\n  Exclude:\n    - '**/vendor/**/*'\n",
        );
        let file = write_file(
            &dir,
            "vendor/plugins/xss_terminate/lib/html5lib_sanitize.rb",
            &long_class_source("class String\n"),
        );

        let config = crate::config::load_config(None, Some(&dir), None).unwrap();
        let registry = crate::cop::registry::CopRegistry::default_registry();
        let tier_map = crate::cop::tiers::TierMap::load();
        let allowlist = crate::cop::autocorrect_allowlist::AutocorrectAllowlist::load();
        let args = class_length_args();
        let result = crate::linter::run_linter(
            &discovered(&[file]),
            &config,
            &registry,
            &args,
            &tier_map,
            &allowlist,
        );

        assert!(
            result.diagnostics.is_empty(),
            "Vendored path excluded by AllCops should be suppressed, got: {:?}",
            result
                .diagnostics
                .iter()
                .map(|d| format!("{d}"))
                .collect::<Vec<_>>()
        );

        fs::remove_dir_all(&dir).ok();
    }
}
