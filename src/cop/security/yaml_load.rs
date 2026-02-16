use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct YamlLoad;

impl Cop for YamlLoad {
    fn name(&self) -> &'static str {
        "Security/YAMLLoad"
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
        // RuboCop: maximum_target_ruby_version 3.0
        // In Ruby 3.1+ (Psych 4), YAML.load uses safe_load by default.
        let ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(2.7);
        if ruby_version > 3.0 {
            return Vec::new();
        }
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"load" {
            return Vec::new();
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let is_yaml = is_yaml_or_psych(source, &recv);
        if !is_yaml {
            return Vec::new();
        }

        let msg_loc = call.message_loc().unwrap();
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Prefer `YAML.safe_load` over `YAML.load`.".to_string(),
        )]
    }
}

fn is_yaml_or_psych(_source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    if let Some(cr) = node.as_constant_read_node() {
        let name = cr.name().as_slice();
        return name == b"YAML" || name == b"Psych";
    }
    if let Some(cp) = node.as_constant_path_node() {
        if let Some(child) = cp.name() {
            let name = child.as_slice();
            if (name == b"YAML" || name == b"Psych") && cp.parent().is_none() {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(YamlLoad, "cops/security/yaml_load");
}
