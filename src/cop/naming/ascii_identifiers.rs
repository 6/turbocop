use crate::cop::node_type::{CONSTANT_WRITE_NODE, DEF_NODE, LOCAL_VARIABLE_WRITE_NODE};
use crate::cop::util::is_ascii_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AsciiIdentifiers;

impl Cop for AsciiIdentifiers {
    fn name(&self) -> &'static str {
        "Naming/AsciiIdentifiers"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CONSTANT_WRITE_NODE, DEF_NODE, LOCAL_VARIABLE_WRITE_NODE]
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
        // AsciiConstants: when true (default), also flag non-ASCII constants
        let ascii_constants = config.get_bool("AsciiConstants", true);

        if let Some(def_node) = node.as_def_node() {
            let method_name = def_node.name().as_slice();
            if !is_ascii_name(method_name) {
                let loc = def_node.name_loc();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use only ascii symbols in identifiers.".to_string(),
                ));
            }
        }

        if let Some(write_node) = node.as_local_variable_write_node() {
            let var_name = write_node.name().as_slice();
            if !is_ascii_name(var_name) {
                let loc = write_node.name_loc();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use only ascii symbols in identifiers.".to_string(),
                ));
            }
        }

        // Check constants only when AsciiConstants is true
        if ascii_constants {
            if let Some(const_write) = node.as_constant_write_node() {
                let const_name = const_write.name().as_slice();
                if !is_ascii_name(const_name) {
                    let loc = const_write.name_loc();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use only ascii symbols in constants.".to_string(),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AsciiIdentifiers, "cops/naming/ascii_identifiers");

    #[test]
    fn config_ascii_constants_true_flags_non_ascii_constant() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("AsciiConstants".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        let source = "Caf\u{00e9} = 1\n".as_bytes();
        let diags = run_cop_full_with_config(&AsciiIdentifiers, source, config);
        assert!(
            !diags.is_empty(),
            "Should flag non-ASCII constant when AsciiConstants:true"
        );
    }

    #[test]
    fn config_ascii_constants_false_allows_non_ascii_constant() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("AsciiConstants".into(), serde_yml::Value::Bool(false))]),
            ..CopConfig::default()
        };
        let source = "Caf\u{00e9} = 1\n".as_bytes();
        let diags = run_cop_full_with_config(&AsciiIdentifiers, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag non-ASCII constant when AsciiConstants:false"
        );
    }
}
