use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RegexpLiteral;

impl Cop for RegexpLiteral {
    fn name(&self) -> &'static str {
        "Style/RegexpLiteral"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "slashes");
        let allow_inner_slashes = config.get_bool("AllowInnerSlashes", false);

        let (open_bytes, content_bytes, node_start, node_end): (Vec<u8>, Vec<u8>, usize, usize) =
            if let Some(re) = node.as_regular_expression_node() {
                let opening = re.opening_loc();
                let content = re.content_loc().as_slice();
                let loc = re.location();
                (opening.as_slice().to_vec(), content.to_vec(), loc.start_offset(), loc.end_offset())
            } else if let Some(re) = node.as_interpolated_regular_expression_node() {
                let opening = re.opening_loc();
                let closing = re.closing_loc();
                let loc = re.location();
                let open = opening.as_slice();
                // Content is between opening and closing delimiters
                let content_start = opening.end_offset() - loc.start_offset();
                let content_end = closing.start_offset() - loc.start_offset();
                let full = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
                let content = if content_end > content_start {
                    full[content_start..content_end].to_vec()
                } else {
                    Vec::new()
                };
                (open.to_vec(), content, loc.start_offset(), loc.end_offset())
            } else {
                return Vec::new();
            };

        let is_slash = open_bytes == b"/";
        let is_percent_r = open_bytes.starts_with(b"%r");

        // Check if content contains unescaped forward slashes.
        // Need to properly handle consecutive backslashes: `\\/` has an unescaped slash
        // because `\\` is an escaped backslash, leaving `/` unescaped.
        let has_slash = {
            let mut found = false;
            let mut i = 0;
            while i < content_bytes.len() {
                if content_bytes[i] == b'\\' {
                    i += 2; // skip escaped character
                    continue;
                }
                if content_bytes[i] == b'/' {
                    found = true;
                    break;
                }
                i += 1;
            }
            found
        };

        let is_multiline = {
            let (start_line, _) = source.offset_to_line_col(node_start);
            let (end_line, _) = source.offset_to_line_col(node_end);
            end_line > start_line
        };

        // %r with content starting with space or = may be used to avoid syntax errors
        // when the regexp is a method argument without parentheses:
        //   do_something %r{ regexp}  # valid
        //   do_something / regexp/    # syntax error
        // Allow %r in these cases (matching RuboCop's behavior).
        let content_starts_with_space_or_eq = !content_bytes.is_empty()
            && (content_bytes[0] == b' ' || content_bytes[0] == b'=');

        match enforced_style {
            "slashes" => {
                if is_percent_r {
                    if has_slash && !allow_inner_slashes {
                        return Vec::new();
                    }
                    if content_starts_with_space_or_eq {
                        return Vec::new();
                    }
                    let (line, column) = source.offset_to_line_col(node_start);
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `//` around regular expression.".to_string(),
                    )];
                }
            }
            "percent_r" => {
                if is_slash {
                    let (line, column) = source.offset_to_line_col(node_start);
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `%r` around regular expression.".to_string(),
                    )];
                }
            }
            "mixed" => {
                if is_multiline {
                    if is_slash {
                        let (line, column) = source.offset_to_line_col(node_start);
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            "Use `%r` around regular expression.".to_string(),
                        )];
                    }
                } else if is_percent_r {
                    if has_slash && !allow_inner_slashes {
                        return Vec::new();
                    }
                    if content_starts_with_space_or_eq {
                        return Vec::new();
                    }
                    let (line, column) = source.offset_to_line_col(node_start);
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `//` around regular expression.".to_string(),
                    )];
                }
            }
            _ => {}
        }

        // For slashes style: check for inner slashes
        if enforced_style == "slashes" && is_slash && has_slash && !allow_inner_slashes {
            let (line, column) = source.offset_to_line_col(node_start);
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Use `%r` around regular expression.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RegexpLiteral, "cops/style/regexp_literal");
}
