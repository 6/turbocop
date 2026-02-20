use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, DEF_NODE, OPTIONAL_KEYWORD_PARAMETER_NODE, REQUIRED_KEYWORD_PARAMETER_NODE};

pub struct KeywordParametersOrder;

impl Cop for KeywordParametersOrder {
    fn name(&self) -> &'static str {
        "Style/KeywordParametersOrder"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, BLOCK_PARAMETERS_NODE, DEF_NODE, OPTIONAL_KEYWORD_PARAMETER_NODE, REQUIRED_KEYWORD_PARAMETER_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Check both def and block parameters
        let parameters = if let Some(def_node) = node.as_def_node() {
            def_node.parameters()
        } else if let Some(block_node) = node.as_block_node() {
            if let Some(params) = block_node.parameters() {
                if let Some(bp) = params.as_block_parameters_node() {
                    bp.parameters()
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let parameters = match parameters {
            Some(p) => p,
            None => return,
        };


        // Check keyword parameters order: required keywords should come before optional keywords
        let keywords: Vec<_> = parameters.keywords().iter().collect();
        let mut seen_required = false;
        let mut have_optional_before_required = false;

        // First pass: check if there are any required keywords after optional ones
        for kw in keywords.iter().rev() {
            if kw.as_required_keyword_parameter_node().is_some() {
                seen_required = true;
            } else if kw.as_optional_keyword_parameter_node().is_some() {
                if seen_required {
                    have_optional_before_required = true;
                    break;
                }
            }
        }

        if !have_optional_before_required {
            return;
        }

        // Second pass: report each optional keyword that appears before a required keyword
        seen_required = false;
        for kw in keywords.iter().rev() {
            if kw.as_required_keyword_parameter_node().is_some() {
                seen_required = true;
            } else if kw.as_optional_keyword_parameter_node().is_some() {
                if seen_required {
                    let loc = kw.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Place optional keyword parameters at the end of the parameters list.".to_string(),
                    ));
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(KeywordParametersOrder, "cops/style/keyword_parameters_order");
}
