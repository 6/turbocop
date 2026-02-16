use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ShadowedArgument;

impl Cop for ShadowedArgument {
    fn name(&self) -> &'static str {
        "Lint/ShadowedArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _ignore_implicit = config.get_bool("IgnoreImplicitReferences", false);

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.is_empty() {
            return Vec::new();
        }

        // Collect parameter names
        let mut param_names = Vec::new();
        for req in params.requireds().iter() {
            if let Some(rp) = req.as_required_parameter_node() {
                param_names.push(rp.name().as_slice().to_vec());
            }
        }
        for opt in params.optionals().iter() {
            if let Some(op) = opt.as_optional_parameter_node() {
                param_names.push(op.name().as_slice().to_vec());
            }
        }

        let mut diagnostics = Vec::new();

        // Check if the first statement reassigns any parameter
        let first = &body_nodes[0];
        if let Some(write) = first.as_local_variable_write_node() {
            let write_name = write.name().as_slice();
            if param_names.iter().any(|p| p.as_slice() == write_name) {
                let loc = write.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Argument `{}` was shadowed by a local variable before it was used.",
                        String::from_utf8_lossy(write_name)
                    ),
                ));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ShadowedArgument, "cops/lint/shadowed_argument");
}
