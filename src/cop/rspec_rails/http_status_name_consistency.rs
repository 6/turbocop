use crate::cop::rspec_rails::RSPEC_RAILS_DEFAULT_INCLUDE;
use crate::cop::shared::node_type::{CALL_NODE, SYMBOL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-09)
///
/// Synthetic benchmark reported FN=2 (corpus has zero activity for this cop).
///
/// FN=2: Fixed by replacing `has_target_rails_version()` (requires railties in
/// lockfile) with `target_rails_version().is_none()`. The RuboCop cop uses
/// `requires_gem 'rack', '>= 3.1.0'`, not `requires_gem 'railties'`. The
/// railties check was too strict for projects without a Gemfile.lock (like
/// the synthetic benchmark project).
///
/// ## Corpus investigation (2026-03-10)
///
/// 580 FP, 0 FN. Root cause: nitrocop resolved `rack_version` from
/// `config_dir` (the config file's parent), but RuboCop uses
/// `base_dir_for_path_parameters` (CWD for non-dotfile configs like
/// `--config baseline_rubocop.yml`). In corpus oracle CI, `config_dir` =
/// `bench/corpus/` whose `Gemfile.lock` has `rack (3.2.5)` as a transitive
/// rubocop dependency, causing the cop to fire on ALL repos regardless of
/// their actual Rack version. RuboCop searches upward from CWD (checkout
/// root) which has no `Gemfile.lock`, so it skips the cop entirely.
///
/// Fixed by changing lockfile resolution in config loader to use `base_dir`
/// (CWD for non-dotfile configs, config_dir for `.rubocop*` dotfiles),
/// matching RuboCop's `bundler_lock_file_path` behavior.
///
/// ## Synthetic FN=2 fix (2026-03-10)
///
/// Persistent synthetic FN=2 was NOT a cop logic bug — it was a benchmark
/// setup issue. The synthetic project lives at `bench/synthetic/project/`
/// but the `Gemfile.lock` (with rack 3.2.5) was at `bench/synthetic/Gemfile.lock`
/// (one directory up). Nitrocop looks for `Gemfile.lock` in `base_dir`
/// (= config file's parent for `.rubocop*` dotfiles = `project/`), so it
/// never found the lockfile → `rack_version` was `None` → cop skipped.
/// RuboCop found it because `run_synthetic.py` sets `BUNDLE_GEMFILE` for
/// RuboCop (Bundler follows the env var), but not for nitrocop.
///
/// Fixed by adding a `Gemfile.lock` symlink in `bench/synthetic/project/`
/// pointing to `../Gemfile.lock`, plus `.gitignore` negations to track both
/// the symlink and its target.
pub struct HttpStatusNameConsistency;

/// Mapping of deprecated status names to their preferred replacements.
fn preferred_status(sym: &[u8]) -> Option<&'static str> {
    match sym {
        b"unprocessable_entity" => Some("unprocessable_content"),
        b"payload_too_large" => Some("content_too_large"),
        _ => None,
    }
}

impl Cop for HttpStatusNameConsistency {
    fn name(&self) -> &'static str {
        "RSpecRails/HttpStatusNameConsistency"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_RAILS_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, SYMBOL_NODE]
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
        // requires_gem 'rack', '>= 3.1.0' — only fire when the project has
        // Rack >= 3.1 in its lockfile (where status names were renamed).
        if !config.rack_version().is_some_and(|v| v >= 3.1) {
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"have_http_status" {
            return;
        }

        if call.receiver().is_some() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let arg = &arg_list[0];
        let sym = match arg.as_symbol_node() {
            Some(s) => s,
            None => return,
        };

        let sym_name = sym.unescaped();
        let current = std::str::from_utf8(sym_name).unwrap_or("");

        if let Some(preferred) = preferred_status(sym_name) {
            let loc = arg.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Prefer `:{preferred}` over `:{current}`."),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rack31_config() -> CopConfig {
        let mut options = std::collections::HashMap::new();
        options.insert(
            "TargetRailsVersion".to_string(),
            serde_yml::Value::Number(serde_yml::value::Number::from(7.0_f64)),
        );
        options.insert(
            "__RailtiesInLockfile".to_string(),
            serde_yml::Value::Bool(true),
        );
        options.insert(
            "__RackVersion".to_string(),
            serde_yml::Value::Number(serde_yml::value::Number::from(3.1_f64)),
        );
        CopConfig {
            options,
            ..CopConfig::default()
        }
    }

    #[test]
    fn offense_fixture() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &HttpStatusNameConsistency,
            include_bytes!(
                "../../../tests/fixtures/cops/rspecrails/http_status_name_consistency/offense.rb"
            ),
            rack31_config(),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &HttpStatusNameConsistency,
            include_bytes!(
                "../../../tests/fixtures/cops/rspecrails/http_status_name_consistency/no_offense.rb"
            ),
            rack31_config(),
        );
    }

    #[test]
    fn skipped_when_no_rack_version() {
        // Projects without rack in lockfile should not trigger this cop.
        let source = include_bytes!(
            "../../../tests/fixtures/cops/rspecrails/http_status_name_consistency/offense.rb"
        );
        let parsed = crate::testutil::parse_fixture(source);
        let diagnostics = crate::testutil::run_cop_full_internal(
            &HttpStatusNameConsistency,
            &parsed.source,
            CopConfig::default(),
            "test.rb",
        );
        assert!(
            diagnostics.is_empty(),
            "Should not fire when rack version is not set, but got {} offenses",
            diagnostics.len()
        );
    }

    #[test]
    fn skipped_when_rack_below_31() {
        // Projects with rack < 3.1 should not trigger this cop.
        let source = include_bytes!(
            "../../../tests/fixtures/cops/rspecrails/http_status_name_consistency/offense.rb"
        );
        let parsed = crate::testutil::parse_fixture(source);
        let mut options = std::collections::HashMap::new();
        options.insert(
            "__RackVersion".to_string(),
            serde_yml::Value::Number(serde_yml::value::Number::from(2.2_f64)),
        );
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        let diagnostics = crate::testutil::run_cop_full_internal(
            &HttpStatusNameConsistency,
            &parsed.source,
            config,
            "test.rb",
        );
        assert!(
            diagnostics.is_empty(),
            "Should not fire when rack version is 2.2, but got {} offenses",
            diagnostics.len()
        );
    }
}
