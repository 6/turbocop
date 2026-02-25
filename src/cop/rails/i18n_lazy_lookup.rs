use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct I18nLazyLookup;

impl Cop for I18nLazyLookup {
    fn name(&self) -> &'static str {
        "Rails/I18nLazyLookup"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let path = source.path_str();
        if !is_controller_file(path) {
            return;
        }

        let style = config.get_str("EnforcedStyle", "lazy");
        let controller_prefix = controller_prefix_from_path(path);

        let mut visitor = I18nLazyLookupVisitor {
            cop: self,
            source,
            style,
            controller_prefix,
            current_method: None,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct I18nLazyLookupVisitor<'a> {
    cop: &'a I18nLazyLookup,
    source: &'a SourceFile,
    style: &'a str,
    controller_prefix: Option<String>,
    current_method: Option<Vec<u8>>,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for I18nLazyLookupVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let prev_method = self.current_method.take();
        self.current_method = Some(node.name().as_slice().to_vec());
        ruby_prism::visit_def_node(self, node);
        self.current_method = prev_method;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method = node.name().as_slice();

        // Only match bare t/translate calls (no receiver)
        let is_bare_t = (method == b"t" || method == b"translate") && node.receiver().is_none();

        if is_bare_t {
            if let Some(args) = node.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if !arg_list.is_empty() {
                    let key = if let Some(s) = arg_list[0].as_string_node() {
                        Some(s.unescaped().to_vec())
                    } else {
                        arg_list[0]
                            .as_symbol_node()
                            .map(|sym| sym.unescaped().to_vec())
                    };

                    if let Some(key) = key {
                        self.check_key(node, &key);
                    }
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }
}

impl I18nLazyLookupVisitor<'_> {
    fn check_key(&mut self, node: &ruby_prism::CallNode<'_>, key: &[u8]) {
        match self.style {
            "explicit" => {
                // Flag lazy lookups (keys starting with '.')
                if !key.starts_with(b".") {
                    return;
                }
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Use explicit lookup for i18n keys.".to_string(),
                ));
            }
            _ => {
                // "lazy" (default): flag explicit lookups that could use lazy lookup
                if key.starts_with(b".") {
                    return;
                }
                // Must have at least 3 segments (controller.action.key)
                let dot_count = key.iter().filter(|&&b| b == b'.').count();
                if dot_count < 2 {
                    return;
                }
                // Must be inside a method def
                let method_name = match &self.current_method {
                    Some(m) => m,
                    None => return,
                };

                // Match RuboCop's behavior: construct a scoped key from
                // controller_path + action_name + last segment of the given key.
                // Only flag if the full key matches this scoped key exactly.
                // This means keys with extra intermediate segments (e.g.,
                // "books.create.flash.success" vs scoped "books.create.success")
                // are NOT flagged.
                if let Some(ref prefix) = self.controller_prefix {
                    let method_str = std::str::from_utf8(method_name).unwrap_or("");
                    let key_str = std::str::from_utf8(key).unwrap_or("");
                    // Extract last segment of the key
                    let last_segment = match key_str.rsplit('.').next() {
                        Some(s) => s,
                        None => return,
                    };
                    let scoped_key = format!("{}.{}.{}", prefix, method_str, last_segment);
                    if key_str != scoped_key {
                        return;
                    }
                }

                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Use lazy lookup for i18n keys.".to_string(),
                ));
            }
        }
    }
}

/// Check if the file path looks like a Rails controller file.
fn is_controller_file(path: &str) -> bool {
    if path.contains("controllers/") || path.contains("controllers\\") {
        return true;
    }
    if path.ends_with("_controller.rb") {
        return true;
    }
    false
}

/// Derive a controller prefix from the file path for key matching.
/// E.g., "app/controllers/admin/accounts_controller.rb" => "admin.accounts"
fn controller_prefix_from_path(path: &str) -> Option<String> {
    let idx = path.find("controllers/")?;
    let rest = &path[idx + "controllers/".len()..];
    let rest = rest.strip_suffix(".rb")?;
    let rest = rest.strip_suffix("_controller").unwrap_or(rest);
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
        let source = b"# nitrocop-filename: app/controllers/books_controller.rb\nt('.success')\n^^^^^^^^^^^^^ Rails/I18nLazyLookup: Use explicit lookup for i18n keys.\n";
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
        let source = b"# nitrocop-filename: app/controllers/books_controller.rb\nt('books.create.success')\n";
        assert_cop_no_offenses_full_with_config(&I18nLazyLookup, source, config);
    }

    #[test]
    fn does_not_flag_outside_controller() {
        use crate::testutil::assert_cop_no_offenses;
        assert_cop_no_offenses(&I18nLazyLookup, b"t('admin.reports.processed_msg')\n");
    }

    #[test]
    fn does_not_flag_i18n_t() {
        use crate::testutil::assert_cop_no_offenses;
        assert_cop_no_offenses(&I18nLazyLookup, b"I18n.t('admin.reports.processed_msg')\n");
    }

    #[test]
    fn does_not_flag_mismatched_action() {
        use crate::testutil::assert_cop_no_offenses_full;
        // Key has action 'update' but we're in method 'validate_confirmation_token'
        let source = b"# nitrocop-filename: app/controllers/email_confirmations_controller.rb\ndef validate_confirmation_token\n  t(\"email_confirmations.update.token_failure\")\nend\n";
        assert_cop_no_offenses_full(&I18nLazyLookup, source);
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
        assert_eq!(controller_prefix_from_path("app/models/user.rb"), None);
    }
}
