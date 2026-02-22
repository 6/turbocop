use crate::cop::node_type::{
    ASSOC_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, HASH_NODE, KEYWORD_HASH_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct PluckInWhere;

impl Cop for PluckInWhere {
    fn name(&self) -> &'static str {
        "Rails/PluckInWhere"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ASSOC_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            HASH_NODE,
            KEYWORD_HASH_NODE,
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
        let style = config.get_str("EnforcedStyle", "conservative");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"where" {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        // Look for pluck inside argument values (keyword hash args)
        for arg in args.arguments().iter() {
            if self.has_pluck_call(&arg, style) {
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use a subquery instead of `pluck` inside `where`.".to_string(),
                ));
            }
        }
    }
}

impl PluckInWhere {
    /// Find the root receiver of a chained call (e.g., `User.active` -> `User`).
    fn root_receiver<'a>(&self, node: &ruby_prism::Node<'a>) -> Option<ruby_prism::Node<'a>> {
        if let Some(call) = node.as_call_node() {
            if let Some(recv) = call.receiver() {
                if recv.as_call_node().is_some() {
                    return self.root_receiver(&recv);
                }
                return Some(recv);
            }
        }
        None
    }

    fn is_const_rooted(&self, node: &ruby_prism::Node<'_>) -> bool {
        if let Some(root) = self.root_receiver(node) {
            return root.as_constant_read_node().is_some()
                || root.as_constant_path_node().is_some();
        }
        false
    }

    fn check_pluck_node(&self, node: &ruby_prism::Node<'_>, style: &str) -> bool {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"pluck" {
                if style == "conservative" {
                    // Only flag if receiver chain is rooted in a constant (model)
                    return self.is_const_rooted(node);
                }
                return true;
            }
        }
        false
    }

    fn has_pluck_call(&self, node: &ruby_prism::Node<'_>, style: &str) -> bool {
        // Direct pluck call
        if self.check_pluck_node(node, style) {
            return true;
        }
        // Check keyword hash values
        if let Some(kw) = node.as_keyword_hash_node() {
            for elem in kw.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    let val = assoc.value();
                    if self.check_pluck_node(&val, style) {
                        return true;
                    }
                }
            }
        }
        // Check hash literal values
        if let Some(hash) = node.as_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    let val = assoc.value();
                    if self.check_pluck_node(&val, style) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PluckInWhere, "cops/rails/pluck_in_where");

    #[test]
    fn conservative_style_skips_non_constant_receiver() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;

        let config = CopConfig::default();
        let source = b"Post.where(user_id: active_users.pluck(:id))\n";
        assert_cop_no_offenses_full_with_config(&PluckInWhere, source, config);
    }

    #[test]
    fn aggressive_style_flags_non_constant_receiver() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("aggressive".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"Post.where(user_id: active_users.pluck(:id))\n";
        let diags = run_cop_full_with_config(&PluckInWhere, source, config);
        assert!(
            !diags.is_empty(),
            "aggressive style should flag non-constant receiver pluck"
        );
    }
}
