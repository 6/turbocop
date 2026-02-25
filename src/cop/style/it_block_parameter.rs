use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, REQUIRED_PARAMETER_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ItBlockParameter;

impl Cop for ItBlockParameter {
    fn name(&self) -> &'static str {
        "Style/ItBlockParameter"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, BLOCK_PARAMETERS_NODE, REQUIRED_PARAMETER_NODE]
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
        // RuboCop: minimum_target_ruby_version 3.4
        let ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(2.7);
        if ruby_version < 3.4 {
            return;
        }

        let _style = config.get_str("EnforcedStyle", "allow_single_line");

        // Detect block parameters named `it`: { |it| ... }
        let block = match node.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let params = match block.parameters() {
            Some(p) => p,
            None => return,
        };

        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };

        let parameters = match block_params.parameters() {
            Some(p) => p,
            None => return,
        };

        for req in parameters.requireds().iter() {
            if let Some(param) = req.as_required_parameter_node() {
                if param.name().as_slice() == b"it" {
                    let loc = param.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Avoid using `it` as a block parameter name, since `it` will be the default block parameter in Ruby 3.4+.".to_string(),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;

    fn ruby34_config() -> CopConfig {
        let mut config = CopConfig::default();
        config.options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::Value::Number(serde_yml::Number::from(3.4)),
        );
        config
    }

    #[test]
    fn offense_with_ruby34() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &ItBlockParameter,
            include_bytes!("../../../tests/fixtures/cops/style/it_block_parameter/offense.rb"),
            ruby34_config(),
        );
    }

    #[test]
    fn no_offense() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &ItBlockParameter,
            include_bytes!("../../../tests/fixtures/cops/style/it_block_parameter/no_offense.rb"),
            ruby34_config(),
        );
    }

    #[test]
    fn no_offense_below_ruby34() {
        // Default Ruby version (2.7) — cop should be completely silent
        crate::testutil::assert_cop_no_offenses_full(
            &ItBlockParameter,
            include_bytes!("../../../tests/fixtures/cops/style/it_block_parameter/offense.rb"),
        );
    }
}
