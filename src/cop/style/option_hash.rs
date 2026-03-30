use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/OptionHash detects methods and blocks with option hash parameters
/// (e.g., `opts = {}`) and suggests using keyword arguments instead.
///
/// Fixed: only bare `super` (forwarding super, without explicit arguments) exempts
/// a method from detection. `super(args)` with explicit arguments does NOT exempt,
/// matching RuboCop's `zsuper`-only check.
///
/// Also detects option hash params in block and lambda parameters (e.g.,
/// `lambda do |opts = {}|` or `define do |name, opts = {}|`).
pub struct OptionHash;

impl Cop for OptionHash {
    fn name(&self) -> &'static str {
        "Style/OptionHash"
    }

    fn default_enabled(&self) -> bool {
        false
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

/// Check if a node tree contains a bare `super` (forwarding super) call.
/// Only bare `super` without explicit arguments exempts a method;
/// `super(args)` does NOT.
fn has_forwarding_super(node: &ruby_prism::Node<'_>) -> bool {
    let mut visitor = HasForwardingSuperVisitor { found: false };
    visitor.visit(node);
    visitor.found
}

struct HasForwardingSuperVisitor {
    found: bool,
}

impl<'pr> Visit<'pr> for HasForwardingSuperVisitor {
    fn visit_forwarding_super_node(&mut self, _node: &ruby_prism::ForwardingSuperNode<'pr>) {
        self.found = true;
    }
}

impl OptionHashVisitor<'_> {
    /// Check a ParametersNode for a trailing option hash param and report a diagnostic.
    fn check_params(&mut self, params: &ruby_prism::ParametersNode<'_>) {
        // RuboCop's pattern: (args ... $(optarg [#suspicious_name? _] (hash)))
        // The optarg must be the LAST child of the args node.
        // In Prism terms: check only the last optional param, and only if
        // no rest, posts, keywords, keyword_rest, or block follow it.
        let has_rest = params.rest().is_some();
        let has_posts = !params.posts().is_empty();
        let has_keywords = !params.keywords().is_empty();
        let has_keyword_rest = params.keyword_rest().is_some();
        let has_block = params.block().is_some();

        if has_rest || has_posts || has_keywords || has_keyword_rest || has_block {
            return;
        }

        let optionals = params.optionals();
        if let Some(last_opt) = optionals.iter().last() {
            if let Some(opt_param) = last_opt.as_optional_parameter_node() {
                let name = opt_param.name();
                let name_str = std::str::from_utf8(name.as_slice()).unwrap_or("");
                if self.suspicious_names.iter().any(|s| s == name_str) {
                    // Check if default value is an empty hash.
                    // RuboCop only flags `(hash)` which is an empty hash literal.
                    let value = opt_param.value();
                    let is_empty_hash = value
                        .as_hash_node()
                        .is_some_and(|h| h.elements().is_empty())
                        || value
                            .as_keyword_hash_node()
                            .is_some_and(|h| h.elements().is_empty());
                    if is_empty_hash {
                        let loc = opt_param.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
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

        // Check if method body contains a bare super call (forwarding super only).
        // super(args) with explicit arguments does NOT exempt the method.
        if let Some(body) = node.body() {
            if has_forwarding_super(&body) {
                // Still visit nested defs
                self.visit(&body);
                return;
            }
        }

        if let Some(params) = node.parameters() {
            self.check_params(&params);
        }

        // Visit body for nested defs
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Check for forwarding super in the block body (e.g. `defined? super`).
        // RuboCop's super_used? searches the block body for zsuper too.
        let body_has_super = node.body().is_some_and(|body| has_forwarding_super(&body));
        if !body_has_super {
            if let Some(block_params) = node.parameters() {
                if let Some(bp) = block_params.as_block_parameters_node() {
                    if let Some(params) = bp.parameters() {
                        self.check_params(&params);
                    }
                }
            }
        }
        ruby_prism::visit_block_node(self, node);
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let body_has_super = node.body().is_some_and(|body| has_forwarding_super(&body));
        if !body_has_super {
            if let Some(block_params) = node.parameters() {
                if let Some(bp) = block_params.as_block_parameters_node() {
                    if let Some(params) = bp.parameters() {
                        self.check_params(&params);
                    }
                }
            }
        }
        ruby_prism::visit_lambda_node(self, node);
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
        assert!(diags.is_empty(), "Should skip methods that call bare super");
    }

    #[test]
    fn super_with_args_does_not_skip() {
        use crate::testutil::run_cop_full;
        let source = b"def process(opts = {})\n  super(opts)\nend\n";
        let diags = run_cop_full(&OptionHash, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag methods that call super(args) — only bare super exempts"
        );
    }
}
