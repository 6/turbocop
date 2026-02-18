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
                let loc = re.location();
                let full = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
                let open = opening.as_slice();
                let content_end = full.len().saturating_sub(1);
                let content_start = open.len();
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

        let has_slash = content_bytes.windows(1).enumerate().any(|(i, w)| {
            w[0] == b'/' && (i == 0 || content_bytes[i - 1] != b'\\')
        });

        let is_multiline = {
            let (start_line, _) = source.offset_to_line_col(node_start);
            let (end_line, _) = source.offset_to_line_col(node_end);
            end_line > start_line
        };

        match enforced_style {
            "slashes" => {
                if is_percent_r {
                    if has_slash && !allow_inner_slashes {
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
