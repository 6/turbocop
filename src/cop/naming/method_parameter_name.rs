use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{DEF_NODE, OPTIONAL_KEYWORD_PARAMETER_NODE, OPTIONAL_PARAMETER_NODE, REQUIRED_KEYWORD_PARAMETER_NODE, REQUIRED_PARAMETER_NODE};

pub struct MethodParameterName;

const DEFAULT_ALLOWED: &[&str] = &[
    "as", "at", "by", "cc", "db", "id", "if", "in", "io", "ip", "of", "on", "os", "pp", "to",
];

impl Cop for MethodParameterName {
    fn name(&self) -> &'static str {
        "Naming/MethodParameterName"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE, OPTIONAL_KEYWORD_PARAMETER_NODE, OPTIONAL_PARAMETER_NODE, REQUIRED_KEYWORD_PARAMETER_NODE, REQUIRED_PARAMETER_NODE]
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
        let min_length = config.get_usize("MinNameLength", 3);
        let _allow_numbers = config.get_bool("AllowNamesEndingInNumbers", true);
        let allowed_names = config.get_string_array("AllowedNames");
        let _forbidden_names = config.get_string_array("ForbiddenNames");

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let allowed: Vec<String> = allowed_names.unwrap_or_else(|| {
            DEFAULT_ALLOWED.iter().map(|s| s.to_string()).collect()
        });

        // Check required parameters
        for param in params.requireds().iter() {
            if let Some(req) = param.as_required_parameter_node() {
                let name = req.name().as_slice();
                check_param(self, source, name, &req.location(), min_length, &allowed, diagnostics);
            }
        }

        // Check optional parameters
        for param in params.optionals().iter() {
            if let Some(opt) = param.as_optional_parameter_node() {
                let name = opt.name().as_slice();
                check_param(self, source, name, &opt.name_loc(), min_length, &allowed, diagnostics);
            }
        }

        // Check keyword parameters
        for param in params.keywords().iter() {
            if let Some(kw) = param.as_required_keyword_parameter_node() {
                let name = kw.name().as_slice();
                // Strip trailing : from keyword name
                let clean_name = if name.ends_with(b":") {
                    &name[..name.len() - 1]
                } else {
                    name
                };
                check_param(self, source, clean_name, &kw.name_loc(), min_length, &allowed, diagnostics);
            }
            if let Some(kw) = param.as_optional_keyword_parameter_node() {
                let name = kw.name().as_slice();
                let clean_name = if name.ends_with(b":") {
                    &name[..name.len() - 1]
                } else {
                    name
                };
                check_param(self, source, clean_name, &kw.name_loc(), min_length, &allowed, diagnostics);
            }
        }

    }
}

fn check_param(
    cop: &MethodParameterName,
    source: &SourceFile,
    name: &[u8],
    loc: &ruby_prism::Location<'_>,
    min_length: usize,
    allowed: &[String],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let name_str = std::str::from_utf8(name).unwrap_or("");

    // Skip _-prefixed names (unused params)
    if name_str.starts_with('_') {
        return;
    }

    // Check allowed names
    if allowed.iter().any(|a| a == name_str) {
        return;
    }

    if name.len() < min_length {
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Method parameter must be at least {min_length} characters long."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(MethodParameterName, "cops/naming/method_parameter_name");
}
