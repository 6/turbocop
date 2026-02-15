use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct I18nLazyLookup;

impl Cop for I18nLazyLookup {
    fn name(&self) -> &'static str {
        "Rails/I18nLazyLookup"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "lazy");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method = call.name().as_slice();

        // Match only bare t("key") or translate("key") — NOT I18n.t("key")
        // RuboCop's cop uses RESTRICT_ON_SEND = [:translate, :t] which only
        // matches receiverless calls (the controller's helper method).
        let is_bare_t = if method == b"t" || method == b"translate" {
            call.receiver().is_none()
        } else {
            false
        };

        if !is_bare_t {
            return Vec::new();
        }

        // RuboCop only fires inside controller classes.
        // Use file path heuristic: must be in a controllers/ directory
        // or the file must end with _controller.rb.
        let path = source.path_str();
        if !is_controller_file(path) {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // Accept both string and symbol keys
        let key = if let Some(s) = arg_list[0].as_string_node() {
            s.unescaped().to_vec()
        } else if let Some(sym) = arg_list[0].as_symbol_node() {
            sym.unescaped().to_vec()
        } else {
            return Vec::new();
        };

        match style {
            "explicit" => {
                // Flag lazy lookups (keys starting with '.')
                if !key.starts_with(b".") {
                    return Vec::new();
                }
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use explicit lookup for i18n keys.".to_string(),
                )]
            }
            _ => {
                // "lazy" (default): flag explicit lookups that could use lazy lookup
                // Key must not already be a lazy key (starting with '.')
                if key.starts_with(b".") {
                    return Vec::new();
                }
                // Must have at least 3 segments (controller.action.key)
                let dot_count = key.iter().filter(|&&b| b == b'.').count();
                if dot_count < 2 {
                    return Vec::new();
                }
                // Check that the key matches the controller path derived from the filename.
                // E.g., for app/controllers/admin/accounts_controller.rb,
                // the controller path is "admin.accounts".
                // A key like "admin.accounts.create.success" matches (prefix matches).
                if let Some(controller_prefix) = controller_prefix_from_path(path) {
                    if !key.starts_with(controller_prefix.as_bytes()) {
                        return Vec::new();
                    }
                }
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use lazy lookup for i18n keys.".to_string(),
                )]
            }
        }
    }
}

/// Check if the file path looks like a Rails controller file.
fn is_controller_file(path: &str) -> bool {
    // Check for controllers/ directory in path
    if path.contains("controllers/") || path.contains("controllers\\") {
        return true;
    }
    // Check for _controller.rb suffix
    if path.ends_with("_controller.rb") {
        return true;
    }
    false
}

/// Derive a controller prefix from the file path for key matching.
/// E.g., "app/controllers/admin/accounts_controller.rb" => "admin.accounts"
fn controller_prefix_from_path(path: &str) -> Option<String> {
    // Find the "controllers/" segment
    let idx = path.find("controllers/")?;
    let rest = &path[idx + "controllers/".len()..];
    // Strip .rb extension
    let rest = rest.strip_suffix(".rb")?;
    // Strip _controller suffix
    let rest = rest.strip_suffix("_controller").unwrap_or(rest);
    // Convert path separators to dots
    Some(rest.replace('/', "."))
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(I18nLazyLookup, "cops/rails/i18n_lazy_lookup");

    #[test]
    fn explicit_style_flags_lazy_key() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("explicit".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"# rblint-filename: app/controllers/books_controller.rb\nt('.success')\n^^^^^^^^^^^^^ Rails/I18nLazyLookup: Use explicit lookup for i18n keys.\n";
        assert_cop_offenses_full_with_config(&I18nLazyLookup, source, config);
    }

    #[test]
    fn explicit_style_allows_explicit_key() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("explicit".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"# rblint-filename: app/controllers/books_controller.rb\nt('books.create.success')\n";
        assert_cop_no_offenses_full_with_config(&I18nLazyLookup, source, config);
    }

    #[test]
    fn does_not_flag_outside_controller() {
        use crate::testutil::assert_cop_no_offenses;
        // Without a controller-like filename, no offenses
        assert_cop_no_offenses(&I18nLazyLookup, b"t('admin.reports.processed_msg')\n");
    }

    #[test]
    fn does_not_flag_i18n_t() {
        use crate::testutil::assert_cop_no_offenses;
        // I18n.t is not a lazy-lookup-eligible call
        assert_cop_no_offenses(&I18nLazyLookup, b"I18n.t('admin.reports.processed_msg')\n");
    }

    #[test]
    fn controller_prefix_extraction() {
        assert_eq!(
            controller_prefix_from_path("app/controllers/admin/accounts_controller.rb"),
            Some("admin.accounts".to_string())
        );
        assert_eq!(
            controller_prefix_from_path("app/controllers/books_controller.rb"),
            Some("books".to_string())
        );
        assert_eq!(
            controller_prefix_from_path("app/models/user.rb"),
            None
        );
    }
}
