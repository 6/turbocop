use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{DEF_NODE, FALSE_NODE, OPTIONAL_PARAMETER_NODE, TRUE_NODE};

pub struct OptionalBooleanParameter;

/// Default allowed methods that can have boolean optional parameters.
const DEFAULT_ALLOWED_METHODS: &[&str] = &["respond_to_missing?"];

impl Cop for OptionalBooleanParameter {
    fn name(&self) -> &'static str {
        "Style/OptionalBooleanParameter"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE, FALSE_NODE, OPTIONAL_PARAMETER_NODE, TRUE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let method_name = def_node.name();
        let method_name_str =
            std::str::from_utf8(method_name.as_slice()).unwrap_or("");

        // Check allowed methods
        let allowed_methods: Vec<String> = config
            .get_string_array("AllowedMethods")
            .unwrap_or_else(|| {
                DEFAULT_ALLOWED_METHODS
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            });

        if allowed_methods.iter().any(|m| m == method_name_str) {
            return Vec::new();
        }

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();

        for opt in params.optionals().iter() {
            if let Some(opt_param) = opt.as_optional_parameter_node() {
                let value = opt_param.value();
                let is_boolean = value.as_true_node().is_some() || value.as_false_node().is_some();

                if is_boolean {
                    let param_loc = opt_param.location();
                    let param_src = &source.as_bytes()
                        [param_loc.start_offset()..param_loc.end_offset()];
                    let param_src_str = String::from_utf8_lossy(param_src);

                    let param_name =
                        std::str::from_utf8(opt_param.name().as_slice()).unwrap_or("param");
                    let value_src = if value.as_true_node().is_some() {
                        "true"
                    } else {
                        "false"
                    };
                    let replacement = format!("{}: {}", param_name, value_src);

                    let (line, column) =
                        source.offset_to_line_col(param_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!(
                            "Prefer keyword arguments for arguments with a boolean default value; use `{}` instead of `{}`.",
                            replacement, param_src_str
                        ),
                    ));
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OptionalBooleanParameter, "cops/style/optional_boolean_parameter");
}
