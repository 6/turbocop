use crate::cop::util::{is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SpecFilePathFormat;

impl Cop for SpecFilePathFormat {
    fn name(&self) -> &'static str {
        "RSpec/SpecFilePathFormat"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Only check ProgramNode (root) so we examine top-level describes
        let program = match node.as_program_node() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let stmts = program.statements();
        let body = stmts.body();

        // Collect top-level describe calls
        let mut describes: Vec<ruby_prism::CallNode<'_>> = Vec::new();
        for stmt in body.iter() {
            if let Some(call) = stmt.as_call_node() {
                let name = call.name().as_slice();
                if !is_rspec_example_group(name) {
                    continue;
                }
                // Skip shared examples
                if name == b"shared_examples" || name == b"shared_examples_for" || name == b"shared_context" {
                    continue;
                }
                describes.push(call);
            }
        }

        // If multiple top-level describes, skip (ambiguous)
        if describes.len() != 1 {
            return Vec::new();
        }

        let call = &describes[0];
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // First arg must be a constant (class name)
        let first_arg = &arg_list[0];
        let class_name = if let Some(cr) = first_arg.as_constant_read_node() {
            std::str::from_utf8(cr.name().as_slice()).unwrap_or("").to_string()
        } else if let Some(cp) = first_arg.as_constant_path_node() {
            let loc = cp.location();
            let text = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
            let s = std::str::from_utf8(text).unwrap_or("");
            // Strip leading ::
            s.trim_start_matches("::").to_string()
        } else {
            return Vec::new();
        };

        // Convert class name to expected path: MyClass -> my_class
        let expected_path = class_to_snake(&class_name);

        // Get optional second string argument for method description
        let method_part = if arg_list.len() >= 2 {
            if let Some(s) = arg_list[1].as_string_node() {
                let val = std::str::from_utf8(s.unescaped()).unwrap_or("");
                // Convert to path-friendly form
                let cleaned: String = val.chars()
                    .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
                    .collect();
                let cleaned = cleaned.trim_matches('_').to_string();
                if cleaned.is_empty() { None } else { Some(cleaned) }
            } else {
                None
            }
        } else {
            None
        };

        let expected_suffix = match &method_part {
            Some(m) => format!("{expected_path}*{m}*_spec.rb"),
            None => format!("{expected_path}*_spec.rb"),
        };

        // Check if the file path matches
        let file_path = source.path_str();
        let normalized = file_path.replace('\\', "/");

        if !path_matches(&normalized, &expected_path, method_part.as_deref()) {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("Spec path should end with `{expected_suffix}`."),
            )];
        }

        Vec::new()
    }
}

fn class_to_snake(name: &str) -> String {
    // Convert CamelCase to snake_case, handle :: -> /
    let parts: Vec<&str> = name.split("::").collect();
    let snake_parts: Vec<String> = parts.iter().map(|p| camel_to_snake(p)).collect();
    snake_parts.join("/")
}

fn camel_to_snake(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            // Check if previous char is lowercase or next char is lowercase
            let prev = s.chars().nth(i - 1);
            if let Some(p) = prev {
                if p.is_lowercase() || p.is_ascii_digit() {
                    result.push('_');
                }
            }
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}

fn path_matches(path: &str, expected_class: &str, method: Option<&str>) -> bool {
    // Check that the path ends with the expected class path and _spec.rb
    let path_lower = path.to_lowercase();
    let class_lower = expected_class.to_lowercase();

    // Must contain the class path
    if !path_lower.contains(&class_lower) {
        return false;
    }

    // Must end with _spec.rb
    if !path_lower.ends_with("_spec.rb") {
        return false;
    }

    // If there's a method part, it should appear in the path too
    if let Some(m) = method {
        let m_lower = m.to_lowercase();
        if !path_lower.contains(&m_lower) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_scenario_fixture_tests!(
        SpecFilePathFormat, "cops/rspec/spec_file_path_format",
        scenario_wrong_class = "wrong_class.rb",
        scenario_wrong_method = "wrong_method.rb",
        scenario_wrong_path = "wrong_path.rb",
    );
}
