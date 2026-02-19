use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, OPTIONAL_KEYWORD_PARAMETER_NODE, OPTIONAL_PARAMETER_NODE, REQUIRED_KEYWORD_PARAMETER_NODE, REQUIRED_PARAMETER_NODE};

pub struct UnusedBlockArgument;

impl Cop for UnusedBlockArgument {
    fn name(&self) -> &'static str {
        "Lint/UnusedBlockArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, BLOCK_PARAMETERS_NODE, OPTIONAL_KEYWORD_PARAMETER_NODE, OPTIONAL_PARAMETER_NODE, REQUIRED_KEYWORD_PARAMETER_NODE, REQUIRED_PARAMETER_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let block_node = match node.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let ignore_empty = config.get_bool("IgnoreEmptyBlocks", true);
        let allow_unused_keyword = config.get_bool("AllowUnusedKeywordArguments", false);

        let body = match block_node.body() {
            Some(b) => b,
            None => {
                if ignore_empty {
                    return Vec::new();
                }
                return Vec::new();
            }
        };

        let block_params = match block_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let params_node = match block_params.as_block_parameters_node() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let params = match params_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        // Collect parameter info: (name_bytes, offset, is_keyword)
        let mut param_info: Vec<(Vec<u8>, usize, bool)> = Vec::new();

        for req in params.requireds().iter() {
            if let Some(rp) = req.as_required_parameter_node() {
                param_info.push((
                    rp.name().as_slice().to_vec(),
                    rp.location().start_offset(),
                    false,
                ));
            }
        }

        for opt in params.optionals().iter() {
            if let Some(op) = opt.as_optional_parameter_node() {
                param_info.push((
                    op.name().as_slice().to_vec(),
                    op.location().start_offset(),
                    false,
                ));
            }
        }

        if !allow_unused_keyword {
            for kw in params.keywords().iter() {
                if let Some(kp) = kw.as_required_keyword_parameter_node() {
                    param_info.push((
                        kp.name().as_slice().to_vec(),
                        kp.location().start_offset(),
                        true,
                    ));
                } else if let Some(kp) = kw.as_optional_keyword_parameter_node() {
                    param_info.push((
                        kp.name().as_slice().to_vec(),
                        kp.location().start_offset(),
                        true,
                    ));
                }
            }
        }

        if param_info.is_empty() {
            return Vec::new();
        }

        // Find all local variable reads in the body
        let mut finder = VarReadFinder {
            names: Vec::new(),
        };
        finder.visit(&body);

        let mut diagnostics = Vec::new();

        for (name, offset, _is_keyword) in &param_info {
            // Skip arguments prefixed with _
            if name.starts_with(b"_") {
                continue;
            }

            // Check if the variable is referenced in the body
            if !finder.names.iter().any(|n| n == name) {
                let (line, column) = source.offset_to_line_col(*offset);
                let display_name = if *_is_keyword {
                    let s = String::from_utf8_lossy(name);
                    s.trim_end_matches(':').to_string()
                } else {
                    String::from_utf8_lossy(name).to_string()
                };
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Unused block argument - `{display_name}`."),
                ));
            }
        }

        diagnostics
    }
}

struct VarReadFinder {
    names: Vec<Vec<u8>>,
}

impl<'pr> Visit<'pr> for VarReadFinder {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        self.names.push(node.name().as_slice().to_vec());
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        self.names.push(node.name().as_slice().to_vec());
    }

    // Don't recurse into nested def/class/module
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnusedBlockArgument, "cops/lint/unused_block_argument");
}
