use crate::cop::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, LAMBDA_NODE, OPTIONAL_KEYWORD_PARAMETER_NODE,
    OPTIONAL_PARAMETER_NODE, REQUIRED_KEYWORD_PARAMETER_NODE, REQUIRED_PARAMETER_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-20): 64 FN all from keyword block parameters
/// (RequiredKeywordParameterNode, OptionalKeywordParameterNode) which were not
/// iterated. Fixed by adding `params_node.keywords()` iteration matching the
/// pattern used in Naming/MethodParameterName.
///
/// ## Corpus investigation (2026-03-23) — extended corpus
///
/// Extended corpus reported FN=22 across 5 repos. All FNs from lambda parameters
/// (`->(locationID) { ... }`, `-> (Foo) { ... }`). In Prism, `->` creates a
/// `LambdaNode` (not `BlockNode`), so lambda parameters were not checked.
/// Fix: added `LAMBDA_NODE` to interested_node_types with matching handler.
pub struct BlockParameterName;

impl Cop for BlockParameterName {
    fn name(&self) -> &'static str {
        "Naming/BlockParameterName"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            LAMBDA_NODE,
            OPTIONAL_KEYWORD_PARAMETER_NODE,
            OPTIONAL_PARAMETER_NODE,
            REQUIRED_KEYWORD_PARAMETER_NODE,
            REQUIRED_PARAMETER_NODE,
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
        let min_length = config.get_usize("MinNameLength", 1);
        let _allow_numbers = config.get_bool("AllowNamesEndingInNumbers", true);
        let _allowed_names = config.get_string_array("AllowedNames");
        let _forbidden_names = config.get_string_array("ForbiddenNames");

        // Extract the parameters node from either BlockNode or LambdaNode.
        // LambdaNode (`-> (params) { }`) uses the same BlockParametersNode
        // structure as BlockNode (`foo { |params| }`).
        let params_node = if let Some(block) = node.as_block_node() {
            block
                .parameters()
                .and_then(|p| p.as_block_parameters_node())
                .and_then(|bp| bp.parameters())
        } else if let Some(lambda) = node.as_lambda_node() {
            lambda
                .parameters()
                .and_then(|p| p.as_block_parameters_node())
                .and_then(|bp| bp.parameters())
        } else {
            None
        };

        let params_node = match params_node {
            Some(p) => p,
            None => return,
        };

        for param in params_node.requireds().iter() {
            if let Some(req) = param.as_required_parameter_node() {
                let name = req.name().as_slice();
                check_param_name(
                    self,
                    source,
                    name,
                    &req.location(),
                    min_length,
                    config,
                    diagnostics,
                );
            }
        }

        for param in params_node.optionals().iter() {
            if let Some(opt) = param.as_optional_parameter_node() {
                let name = opt.name().as_slice();
                check_param_name(
                    self,
                    source,
                    name,
                    &opt.location(),
                    min_length,
                    config,
                    diagnostics,
                );
            }
        }

        for param in params_node.keywords().iter() {
            if let Some(kw) = param.as_required_keyword_parameter_node() {
                let name = kw.name().as_slice();
                let clean_name = if name.ends_with(b":") {
                    &name[..name.len() - 1]
                } else {
                    name
                };
                check_param_name(
                    self,
                    source,
                    clean_name,
                    &kw.name_loc(),
                    min_length,
                    config,
                    diagnostics,
                );
            }
            if let Some(kw) = param.as_optional_keyword_parameter_node() {
                let name = kw.name().as_slice();
                let clean_name = if name.ends_with(b":") {
                    &name[..name.len() - 1]
                } else {
                    name
                };
                check_param_name(
                    self,
                    source,
                    clean_name,
                    &kw.name_loc(),
                    min_length,
                    config,
                    diagnostics,
                );
            }
        }
    }
}

fn check_param_name(
    cop: &BlockParameterName,
    source: &SourceFile,
    name: &[u8],
    loc: &ruby_prism::Location<'_>,
    min_length: usize,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let name_str = std::str::from_utf8(name).unwrap_or("");

    // RuboCop skips plain `_` entirely
    if name_str == "_" {
        return;
    }

    // Strip leading underscores for all checks, matching RuboCop's UncommunicativeName mixin
    let stripped = name_str.trim_start_matches('_');

    // Check allowed names (against stripped name, matching RuboCop)
    if let Some(allowed) = config.get_string_array("AllowedNames") {
        if allowed.iter().any(|a| a == stripped) {
            return;
        }
    }

    // Check forbidden names (against stripped name, matching RuboCop)
    if let Some(forbidden) = config.get_string_array("ForbiddenNames") {
        if forbidden.iter().any(|f| f == stripped) {
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(cop.diagnostic(
                source,
                line,
                column,
                "Block parameter name is too short.".to_string(),
            ));
            return;
        }
    }

    // Check for capital letters (against stripped name, matching RuboCop)
    if stripped.bytes().any(|b| b.is_ascii_uppercase()) {
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            "Block parameter must not contain capital letters.".to_string(),
        ));
        return;
    }

    // Check minimum length (against stripped name, matching RuboCop)
    if stripped.len() < min_length {
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            "Block parameter name is too short.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(BlockParameterName, "cops/naming/block_parameter_name");
}
