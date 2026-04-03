use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use std::path::Path;

/// Detects `require_relative` calls that require the file itself.
///
/// ## Investigation (2026-03-28): RuboCop compares raw strings, not resolved paths
/// Corpus had 4 false positives (`./rspec`, `utils.rb`, `./create_fuzzer_dlg`,
/// `router.rb`) and 3 false negatives (`changelog.rake`, `mutex_m.gemspec`,
/// `rexml.gemspec`) after a previous heuristic tried to model how
/// `require_relative` resolves file extensions.
///
/// RuboCop's actual cop is much narrower: it flags only when the string argument
/// matches the processed file path string exactly, or matches the current file's
/// basename without extension. Mirroring that string comparison removes the FP
/// cases that only share a basename while restoring the `.rake` and `.gemspec`
/// offenses that RuboCop still reports.
pub struct RequireRelativeSelfPath;

impl Cop for RequireRelativeSelfPath {
    fn name(&self) -> &'static str {
        "Lint/RequireRelativeSelfPath"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        // Look for `require_relative 'self_filename'`
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"require_relative" {
            return;
        }

        // Must have no receiver
        if call.receiver().is_some() {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        if args.len() != 1 {
            return;
        }

        let first_arg = args.iter().next().unwrap();
        let string_node = match first_arg.as_string_node() {
            Some(s) => s,
            None => return,
        };

        let required_path = string_node.unescaped();
        let required_str = match std::str::from_utf8(required_path) {
            Ok(s) => s,
            Err(_) => return,
        };

        let file_path = Path::new(source.path_str());
        let file_stem = match file_path.file_stem().and_then(|s| s.to_str()) {
            Some(stem) => stem,
            None => return,
        };

        if source.path_str() == required_str || file_stem == required_str {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Remove the `require_relative` that requires itself.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        RequireRelativeSelfPath,
        "cops/lint/require_relative_self_path",
        same_basename = "same_basename.rb",
        same_basename_with_ext = "same_basename_with_ext.rb",
        rake_same_basename = "rake_same_basename.rb",
        mutex_m_gemspec_rescue = "mutex_m_gemspec_rescue.rb",
        rexml_gemspec_rescue = "rexml_gemspec_rescue.rb",
    );

    #[test]
    fn no_offense_nested_rb_file_same_basename_with_ext() {
        let source = b"require_relative 'utils.rb'\n";
        let diags = crate::testutil::run_cop_full_internal(
            &RequireRelativeSelfPath,
            source,
            crate::cop::CopConfig::default(),
            "lib/shodanz/apis/utils.rb",
        );
        assert!(
            diags.is_empty(),
            "Expected no offenses for nested .rb file but got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_nested_rb_file_same_basename_with_dot_prefix() {
        let source = b"require_relative './rspec'\n";
        let diags = crate::testutil::run_cop_full_internal(
            &RequireRelativeSelfPath,
            source,
            crate::cop::CopConfig::default(),
            "features/support/rspec.rb",
        );
        assert!(
            diags.is_empty(),
            "Expected no offenses for nested .rb file but got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_non_rb_file_same_basename_with_rb_ext() {
        let source = b"require_relative 'router.rb'\n";
        let diags = crate::testutil::run_cop_full_internal(
            &RequireRelativeSelfPath,
            source,
            crate::cop::CopConfig::default(),
            "rackup/router.ru",
        );
        assert!(
            diags.is_empty(),
            "Expected no offenses for non-.rb file but got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_non_rb_file_same_basename_with_dot_prefix() {
        let source = b"require_relative './persistent'\n";
        let diags = crate::testutil::run_cop_full_internal(
            &RequireRelativeSelfPath,
            source,
            crate::cop::CopConfig::default(),
            "test/apps/persistent.ru",
        );
        assert!(
            diags.is_empty(),
            "Expected no offenses for non-.rb file but got: {:?}",
            diags
        );
    }
}
