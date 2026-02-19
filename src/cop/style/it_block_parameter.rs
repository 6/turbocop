use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, REQUIRED_PARAMETER_NODE};

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
    ) {
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
    crate::cop_fixture_tests!(ItBlockParameter, "cops/style/it_block_parameter");
}
