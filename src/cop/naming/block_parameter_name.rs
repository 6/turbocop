use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct BlockParameterName;

impl Cop for BlockParameterName {
    fn name(&self) -> &'static str {
        "Naming/BlockParameterName"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let min_length = config.get_usize("MinNameLength", 1);
        let _allow_numbers = config.get_bool("AllowNamesEndingInNumbers", true);
        let _allowed_names = config.get_string_array("AllowedNames");
        let _forbidden_names = config.get_string_array("ForbiddenNames");

        let block = match node.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let params = match block.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return Vec::new(),
        };

        let params_node = match block_params.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();

        for param in params_node.requireds().iter() {
            if let Some(req) = param.as_required_parameter_node() {
                let name = req.name().as_slice();
                check_param_name(self, source, name, &req.location(), min_length, config, &mut diagnostics);
            }
        }

        for param in params_node.optionals().iter() {
            if let Some(opt) = param.as_optional_parameter_node() {
                let name = opt.name().as_slice();
                check_param_name(self, source, name, &opt.location(), min_length, config, &mut diagnostics);
            }
        }

        diagnostics
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

    // Check allowed names
    if let Some(allowed) = config.get_string_array("AllowedNames") {
        if allowed.iter().any(|a| a == name_str) {
            return;
        }
    }

    // Check forbidden names
    if let Some(forbidden) = config.get_string_array("ForbiddenNames") {
        if forbidden.iter().any(|f| f == name_str) {
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

    // Check for capital letters
    if name.iter().any(|&b| b.is_ascii_uppercase()) {
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            "Block parameter must not contain capital letters.".to_string(),
        ));
        return;
    }

    // Check minimum length
    if name.len() < min_length {
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
