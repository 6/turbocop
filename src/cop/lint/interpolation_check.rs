use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::STRING_NODE;

pub struct InterpolationCheck;

impl Cop for InterpolationCheck {
    fn name(&self) -> &'static str {
        "Lint/InterpolationCheck"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let string_node = match node.as_string_node() {
            Some(s) => s,
            None => return,
        };

        // Only check single-quoted strings.
        // opening_loc gives us the quote character (', ", %q{, etc.)
        let opening = match string_node.opening_loc() {
            Some(loc) => loc,
            None => return, // bare string (heredoc body etc.)
        };

        let open_slice = opening.as_slice();
        // Single-quoted: starts with ' or %q
        let is_single_quoted = open_slice == b"'"
            || open_slice.starts_with(b"%q");

        if !is_single_quoted {
            return;
        }

        // Check the raw source content between quotes for #{...}
        let content_loc = string_node.content_loc();
        let content_bytes = &source.as_bytes()[content_loc.start_offset()..content_loc.end_offset()];

        // Look for #{ in the content
        let mut i = 0;
        while i + 1 < content_bytes.len() {
            if content_bytes[i] == b'#' && content_bytes[i + 1] == b'{' {
                // Found interpolation-like pattern. Check it would be valid
                // if the string were double-quoted (the vendor does this check).
                // For simplicity, just check that there's a matching closing }.
                let mut depth = 0;
                let mut j = i + 1;
                let mut has_closing = false;
                while j < content_bytes.len() {
                    if content_bytes[j] == b'{' {
                        depth += 1;
                    } else if content_bytes[j] == b'}' {
                        depth -= 1;
                        if depth == 0 {
                            has_closing = true;
                            break;
                        }
                    }
                    j += 1;
                }

                if has_closing {
                    // Filter out Mustache/Liquid template syntax like `#{{ var }}`
                    // which has double opening braces, not valid Ruby interpolation.
                    if i + 2 < content_bytes.len() && content_bytes[i + 2] == b'{' {
                        break;
                    }

                    // Skip if the string contains double quotes — converting to
                    // double-quoted strings would break syntax (unescaped quotes).
                    // This matches RuboCop's valid_syntax? check.
                    if content_bytes.contains(&b'"') {
                        return;
                    }

                    // Check that the #{...} content looks like valid Ruby.
                    // Format directives like %<variable>s are NOT valid Ruby
                    // interpolation — skip them.
                    let interp_content = &content_bytes[i + 2..j];
                    if interp_content.contains(&b'%') && interp_content.contains(&b'<') {
                        return;
                    }

                    // Report at the string node's opening quote (matching RuboCop)
                    let open_offset = opening.start_offset();
                    let (line, column) = source.offset_to_line_col(open_offset);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.".to_string(),
                    ));
                    return;
                }
                break;
            }
            i += 1;
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InterpolationCheck, "cops/lint/interpolation_check");
}
