use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ContextWording;

const DEFAULT_PREFIXES: &[&str] = &["when", "with", "without"];

impl Cop for ContextWording {
    fn name(&self) -> &'static str {
        "RSpec/ContextWording"
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
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method = call.name().as_slice();
        if method != b"context" && method != b"shared_context" {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let string_node = match arg_list[0].as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let content = string_node.unescaped();
        let content_str = match std::str::from_utf8(&content) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        // Read Prefixes from config, fall back to defaults
        let config_prefixes = config.get_string_array("Prefixes");
        let prefixes: Vec<&str> = if let Some(ref arr) = config_prefixes {
            arr.iter().map(|s| s.as_str()).collect()
        } else {
            DEFAULT_PREFIXES.to_vec()
        };

        // Check if description starts with any allowed prefix followed by a word boundary
        for prefix in &prefixes {
            if content_str.starts_with(prefix) {
                let after = &content_str[prefix.len()..];
                if after.is_empty()
                    || after.starts_with(' ')
                    || after.starts_with(',')
                    || after.starts_with('\n')
                {
                    return Vec::new();
                }
            }
        }

        let prefix_display: Vec<String> =
            prefixes.iter().map(|p| format!("/^{p}\\b/")).collect();
        let loc = arg_list[0].location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!(
                "Context description should match {}.",
                prefix_display.join(", ")
            ),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ContextWording, "cops/rspec/context_wording");
}
