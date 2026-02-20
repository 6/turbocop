use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct FindEach;

const AR_SCOPE_METHODS: &[&[u8]] = &[b"all", b"where", b"order", b"select"];

impl Cop for FindEach {
    fn name(&self) -> &'static str {
        "Rails/FindEach"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");

        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        if chain.outer_method != b"each" {
            return;
        }

        if !AR_SCOPE_METHODS.contains(&chain.inner_method) {
            return;
        }

        let inner_str = std::str::from_utf8(chain.inner_method).unwrap_or("");

        // Skip if inner method is in AllowedMethods
        if let Some(ref list) = allowed_methods {
            if list.iter().any(|m| m == inner_str) {
                return;
            }
        }

        // Skip if inner method matches any AllowedPatterns (substring match)
        if let Some(ref patterns) = allowed_patterns {
            if patterns.iter().any(|p| inner_str.contains(p.as_str())) {
                return;
            }
        }

        // The outer call (each) should have a block
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if outer_call.block().is_none() {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `find_each` instead of `each` for batch processing.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FindEach, "cops/rails/find_each");

    #[test]
    fn allowed_patterns_suppresses_offense() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedPatterns".to_string(),
                serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("order".to_string()),
                ]),
            )]),
            ..CopConfig::default()
        };
        let source = b"User.order(:name).each { |u| puts u }\n";
        let diags = run_cop_full_with_config(&FindEach, source, config);
        assert!(diags.is_empty(), "AllowedPatterns should suppress offense for matching method");
    }
}
