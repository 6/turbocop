use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct OptionHash;

impl Cop for OptionHash {
    fn name(&self) -> &'static str {
        "Style/OptionHash"
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
        let suspicious_names = config
            .get_string_array("SuspiciousParamNames")
            .unwrap_or_else(|| {
                vec![
                    "options".to_string(),
                    "opts".to_string(),
                    "args".to_string(),
                    "params".to_string(),
                    "parameters".to_string(),
                ]
            });
        let allowlist = config.get_string_array("Allowlist").unwrap_or_default();

        let mut visitor = OptionHashVisitor {
            cop: self,
            source,
            suspicious_names,
            allowlist,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct OptionHashVisitor<'a> {
    cop: &'a OptionHash,
    source: &'a SourceFile,
    suspicious_names: Vec<String>,
    allowlist: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}

/// Check if a node tree contains a `super` or `super(...)` call.
fn has_super(node: &ruby_prism::Node<'_>) -> bool {
    let mut visitor = HasSuperVisitor { found: false };
    visitor.visit(node);
    visitor.found
}

struct HasSuperVisitor {
    found: bool,
}

impl<'pr> Visit<'pr> for HasSuperVisitor {
    fn visit_forwarding_super_node(&mut self, _node: &ruby_prism::ForwardingSuperNode<'pr>) {
        self.found = true;
    }

    fn visit_super_node(&mut self, _node: &ruby_prism::SuperNode<'pr>) {
        self.found = true;
    }
}

impl<'pr> Visit<'pr> for OptionHashVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Check method name against allowlist
        let method_name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        if self.allowlist.iter().any(|s| s == method_name) {
            // Still visit nested defs
            if let Some(body) = node.body() {
                self.visit(&body);
            }
            return;
        }

        // Check if method body contains a super call
        if let Some(body) = node.body() {
            let body_node = body;
            if has_super(&body_node) {
                // Still visit nested defs
                self.visit(&body_node);
                return;
            }
        }

        if let Some(params) = node.parameters() {
            // RuboCop's pattern: (args ... $(optarg [#suspicious_name? _] (hash)))
            // The optarg must be the LAST child of the args node.
            // In Prism terms: check only the last optional param, and only if
            // no rest, posts, keywords, keyword_rest, or block follow it.
            let has_rest = params.rest().is_some();
            let has_posts = !params.posts().is_empty();
            let has_keywords = !params.keywords().is_empty();
            let has_keyword_rest = params.keyword_rest().is_some();
            let has_block = params.block().is_some();

            // The optarg can only be last if nothing follows optional params
            if !has_rest && !has_posts && !has_keywords && !has_keyword_rest && !has_block {
                // Check only the last optional parameter
                let optionals = params.optionals();
                if let Some(last_opt) = optionals.iter().last() {
                    if let Some(opt_param) = last_opt.as_optional_parameter_node() {
                        let name = opt_param.name();
                        let name_str = std::str::from_utf8(name.as_slice()).unwrap_or("");
                        if self.suspicious_names.iter().any(|s| s == name_str) {
                            // Check if default value is an empty hash
                            let value = opt_param.value();
                            if value.as_hash_node().is_some()
                                || value.as_keyword_hash_node().is_some()
                            {
                                let loc = opt_param.location();
                                let (line, column) =
                                    self.source.offset_to_line_col(loc.start_offset());
                                self.diagnostics.push(self.cop.diagnostic(
                                    self.source,
                                    line,
                                    column,
                                    format!("Use keyword arguments instead of an options hash argument `{name_str}`."),
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Visit body for nested defs
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OptionHash, "cops/style/option_hash");

    #[test]
    fn allowlist_skips_method() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "Allowlist".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("initialize".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"def initialize(options = {})\n  @options = options\nend\n";
        let diags = run_cop_full_with_config(&OptionHash, source, config);
        assert!(diags.is_empty(), "Should skip methods in Allowlist");
    }

    #[test]
    fn allowlist_does_not_skip_unlisted_method() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "Allowlist".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("initialize".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"def foo(options = {})\n  @options = options\nend\n";
        let diags = run_cop_full_with_config(&OptionHash, source, config);
        assert_eq!(diags.len(), 1, "Should still flag methods not in Allowlist");
    }

    #[test]
    fn super_skips_forwarding_super() {
        use crate::testutil::run_cop_full;
        let source = b"def update(options = {})\n  super\nend\n";
        let diags = run_cop_full(&OptionHash, source);
        assert!(diags.is_empty(), "Should skip methods that call super");
    }

    #[test]
    fn super_skips_explicit_super() {
        use crate::testutil::run_cop_full;
        let source = b"def process(opts = {})\n  super(opts)\nend\n";
        let diags = run_cop_full(&OptionHash, source);
        assert!(
            diags.is_empty(),
            "Should skip methods that call super(args)"
        );
    }
}
