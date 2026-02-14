use crate::cop::util::{is_rspec_example, is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ExcessiveDocstringSpacing;

impl Cop for ExcessiveDocstringSpacing {
    fn name(&self) -> &'static str {
        "RSpec/ExcessiveDocstringSpacing"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();

        // Must be an RSpec method (example group, example, skip, its, etc.)
        let is_rspec = is_rspec_example_group(method_name)
            || is_rspec_example(method_name)
            || method_name == b"its";

        if !is_rspec {
            return Vec::new();
        }

        // Must be receiverless or RSpec.describe
        if let Some(recv) = call.receiver() {
            if let Some(cr) = recv.as_constant_read_node() {
                if cr.name().as_slice() != b"RSpec" {
                    return Vec::new();
                }
            } else {
                return Vec::new();
            }
        }

        // Get first argument — must be a string
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let first_arg = &arg_list[0];

        // Get the string content
        let string_content = if let Some(s) = first_arg.as_string_node() {
            s.unescaped().to_vec()
        } else if let Some(s) = first_arg.as_interpolated_string_node() {
            // For interpolated strings, check the raw source between quotes
            let loc = s.location();
            let raw = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
            // Get content between quotes
            if raw.len() >= 2 {
                let inner = &raw[1..raw.len() - 1];
                inner.to_vec()
            } else {
                return Vec::new();
            }
        } else {
            return Vec::new();
        };

        // Check for excessive whitespace: leading, trailing, or multiple consecutive spaces
        let content_str = match std::str::from_utf8(&string_content) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let has_leading_space = content_str.starts_with(' ') || content_str.starts_with('\u{3000}') || content_str.starts_with('\u{00a0}');
        let has_trailing_space = content_str.ends_with(' ') || content_str.ends_with('\u{3000}') || content_str.ends_with('\u{00a0}');
        let has_multiple_spaces = content_str.contains("  ");

        if !has_leading_space && !has_trailing_space && !has_multiple_spaces {
            return Vec::new();
        }

        let loc = first_arg.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Excessive whitespace.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExcessiveDocstringSpacing, "cops/rspec/excessive_docstring_spacing");
}
