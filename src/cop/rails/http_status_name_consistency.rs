use crate::cop::shared::node_type::{
    ASSOC_NODE, CALL_NODE, ELSE_NODE, HASH_NODE, IF_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE,
    UNLESS_NODE,
};
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
/// 580 FP (shared with RSpecRails variant), 0 FN. Root cause: config
/// loader resolved `rack_version` from `config_dir` (config file's parent)
/// instead of `base_dir` (CWD for non-dotfile configs). In corpus oracle
/// CI, `config_dir` = `bench/corpus/` whose `Gemfile.lock` has rack 3.2.5
/// as a transitive dep, causing the cop to fire on all repos. Fixed by
/// changing lockfile resolution to use `base_dir`, matching RuboCop's
/// `bundler_lock_file_path` / `base_dir_for_path_parameters`.
pub struct HttpStatusNameConsistency;

/// Deprecated HTTP status names and their preferred replacements (Rack >= 3.1).
const PREFERRED_STATUSES: &[(&[u8], &str)] = &[
    (b"unprocessable_entity", "unprocessable_content"),
    (b"payload_too_large", "content_too_large"),
];

impl Cop for HttpStatusNameConsistency {
    fn name(&self) -> &'static str {
        "Rails/HttpStatusNameConsistency"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/app/controllers/**/*.rb"]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ASSOC_NODE,
            CALL_NODE,
            ELSE_NODE,
            HASH_NODE,
            IF_NODE,
            KEYWORD_HASH_NODE,
            SYMBOL_NODE,
            UNLESS_NODE,
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
        // requires_gem 'rack', '>= 3.1.0' — only fire when the project has
        // Rack >= 3.1 in its lockfile (where status names were renamed).
        if !config.rack_version().is_some_and(|v| v >= 3.1) {
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = call.name().as_slice();
        // RESTRICT_ON_SEND = %i[render redirect_to head assert_response assert_redirected_to]
        if !matches!(
            method,
            b"render" | b"redirect_to" | b"head" | b"assert_response" | b"assert_redirected_to"
        ) {
            return;
        }

        // Must be receiverless
        if call.receiver().is_some() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        // Look for deprecated status symbols in arguments
        for arg in args.arguments().iter() {
            self.check_for_deprecated_status(source, &arg, diagnostics);
        }
    }
}

impl HttpStatusNameConsistency {
    fn check_for_deprecated_status(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Check symbol nodes
        if let Some(sym) = node.as_symbol_node() {
            let name = sym.unescaped();
            for &(deprecated, preferred) in PREFERRED_STATUSES {
                if AsRef::<[u8]>::as_ref(name) == deprecated {
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!(
                            "Prefer `:{preferred}` over `:{}`.",
                            String::from_utf8_lossy(deprecated)
                        ),
                    ));
                    return;
                }
            }
        }

        // Check hash nodes for `status: :deprecated_name`
        if let Some(hash) = node.as_hash_node() {
            for element in hash.elements().iter() {
                if let Some(pair) = element.as_assoc_node() {
                    if let Some(key_sym) = pair.key().as_symbol_node() {
                        if AsRef::<[u8]>::as_ref(key_sym.unescaped()) == b"status" {
                            self.check_for_deprecated_status(source, &pair.value(), diagnostics);
                        }
                    }
                }
            }
        }

        // Check keyword hash nodes (inline keyword args)
        if let Some(hash) = node.as_keyword_hash_node() {
            for element in hash.elements().iter() {
                if let Some(pair) = element.as_assoc_node() {
                    if let Some(key_sym) = pair.key().as_symbol_node() {
                        if AsRef::<[u8]>::as_ref(key_sym.unescaped()) == b"status" {
                            self.check_for_deprecated_status(source, &pair.value(), diagnostics);
                        }
                    }
                }
            }
        }

        // Check conditional expressions (ternary: condition ? val1 : val2)
        if let Some(if_node) = node.as_if_node() {
            if let Some(stmts) = if_node.statements() {
                for stmt in stmts.body().iter() {
                    self.check_for_deprecated_status(source, &stmt, diagnostics);
                }
            }
            if let Some(subsequent) = if_node.subsequent() {
                if let Some(else_node) = subsequent.as_else_node() {
                    if let Some(stmts) = else_node.statements() {
                        for stmt in stmts.body().iter() {
                            self.check_for_deprecated_status(source, &stmt, diagnostics);
                        }
                    }
                } else if let Some(elsif_node) = subsequent.as_if_node() {
                    self.check_for_deprecated_status(source, &elsif_node.as_node(), diagnostics);
                }
            }
        }

        // Check unless expressions
        if let Some(unless_node) = node.as_unless_node() {
            if let Some(stmts) = unless_node.statements() {
                for stmt in stmts.body().iter() {
                    self.check_for_deprecated_status(source, &stmt, diagnostics);
                }
            }
            if let Some(else_clause) = unless_node.else_clause() {
                if let Some(stmts) = else_clause.statements() {
                    for stmt in stmts.body().iter() {
                        self.check_for_deprecated_status(source, &stmt, diagnostics);
                    }
                }
            }
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
                "../../../tests/fixtures/cops/rails/http_status_name_consistency/offense.rb"
            ),
            rack31_config(),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &HttpStatusNameConsistency,
            include_bytes!(
                "../../../tests/fixtures/cops/rails/http_status_name_consistency/no_offense.rb"
            ),
            rack31_config(),
        );
    }

    #[test]
    fn skipped_when_no_rack_version() {
        // Projects without rack in lockfile should not trigger this cop.
        let source = include_bytes!(
            "../../../tests/fixtures/cops/rails/http_status_name_consistency/offense.rb"
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
            "../../../tests/fixtures/cops/rails/http_status_name_consistency/offense.rb"
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
