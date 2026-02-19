use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE};

pub struct SymbolProc;

impl Cop for SymbolProc {
    fn name(&self) -> &'static str {
        "Style/SymbolProc"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allow_methods_with_arguments = config.get_bool("AllowMethodsWithArguments", false);
        let allowed_methods = config.get_string_array("AllowedMethods");
        let _allowed_patterns = config.get_string_array("AllowedPatterns");
        let _allow_comments = config.get_bool("AllowComments", false);

        // Look for blocks like { |x| x.foo } that can be replaced with (&:foo)
        // We match on CallNode (the method receiving the block) so we can
        // check AllowedMethods against the outer method name.
        let call_with_block = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let block = match call_with_block.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        // Check outer method name against AllowedMethods
        let outer_method = call_with_block.name().as_slice();
        if let Some(ref allowed) = allowed_methods {
            if let Ok(name_str) = std::str::from_utf8(outer_method) {
                if allowed.iter().any(|m| m == name_str) {
                    return Vec::new();
                }
            }
        }

        // Must have exactly one parameter
        let params = match block.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return Vec::new(),
        };

        let param_node = match block_params.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let requireds: Vec<_> = param_node.requireds().iter().collect();
        if requireds.len() != 1 {
            return Vec::new();
        }

        let param_name = if let Some(rp) = requireds[0].as_required_parameter_node() {
            rp.name().as_slice().to_vec()
        } else {
            return Vec::new();
        };

        // Must have no rest, keyword, optional, or block params
        if param_node.optionals().iter().count() > 0
            || param_node.rest().is_some()
            || param_node.keywords().iter().count() > 0
            || param_node.keyword_rest().is_some()
            || param_node.block().is_some()
        {
            return Vec::new();
        }

        // Body must be a single method call on the parameter
        let body = match block.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return Vec::new();
        }

        let call = match body_nodes[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must not use safe navigation (&.) - can't convert to &:method
        if let Some(op) = call.call_operator_loc() {
            if op.as_slice() == b"&." {
                return Vec::new();
            }
        }

        // The receiver must be the block parameter
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let is_param = if let Some(lv) = receiver.as_local_variable_read_node() {
            lv.name().as_slice() == param_name
        } else {
            false
        };

        if !is_param {
            return Vec::new();
        }

        // Method must not have arguments (unless AllowMethodsWithArguments)
        if !allow_methods_with_arguments && call.arguments().is_some() {
            return Vec::new();
        }

        // Must not have a block
        if call.block().is_some() {
            return Vec::new();
        }

        let method_name = call.name().as_slice();

        let loc = block.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!(
                "Pass `&:{}` as an argument to the method instead of a block.",
                String::from_utf8_lossy(method_name),
            ),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full_with_config, run_cop_full_with_config};

    crate::cop_fixture_tests!(SymbolProc, "cops/style/symbol_proc");

    fn config_with_allowed(methods: &[&str]) -> CopConfig {
        let mut config = CopConfig::default();
        let allowed: Vec<serde_yml::Value> = methods
            .iter()
            .map(|m| serde_yml::Value::String(m.to_string()))
            .collect();
        config
            .options
            .insert("AllowedMethods".to_string(), serde_yml::Value::Sequence(allowed));
        config
    }

    #[test]
    fn allowed_methods_skips_outer_method() {
        let config = config_with_allowed(&["respond_to"]);
        let source = b"respond_to do |format|\n  format.html\nend\n";
        assert_cop_no_offenses_full_with_config(&SymbolProc, source, config);
    }

    #[test]
    fn non_allowed_method_still_fires() {
        let config = config_with_allowed(&["respond_to"]);
        let source = b"items.map { |x| x.to_s }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, config);
        assert_eq!(diags.len(), 1);
    }
}
