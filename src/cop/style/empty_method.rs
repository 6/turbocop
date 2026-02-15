use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyMethod;

impl Cop for EmptyMethod {
    fn name(&self) -> &'static str {
        "Style/EmptyMethod"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        // Skip endless methods (no end keyword)
        let end_kw_loc = match def_node.end_keyword_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        // Skip if already on a single line (def and end on same line)
        let def_loc = def_node.def_keyword_loc();
        let (def_line, _) = source.offset_to_line_col(def_loc.start_offset());
        let (end_line, _) = source.offset_to_line_col(end_kw_loc.start_offset());
        if def_line == end_line {
            return Vec::new();
        }

        let is_empty = match def_node.body() {
            None => true,
            Some(body) => {
                if let Some(stmts) = body.as_statements_node() {
                    stmts.body().is_empty()
                } else {
                    false
                }
            }
        };

        if !is_empty {
            return Vec::new();
        }

        // Check for comment lines between def and end.
        // Prism treats comments as not part of the AST body, so a method with
        // only comments will have an empty/None body. RuboCop does not flag
        // methods that contain comments.
        for line_num in (def_line + 1)..end_line {
            if let Some(line) = source.lines().nth(line_num - 1) {
                let trimmed = line.iter().skip_while(|&&b| b == b' ' || b == b'\t');
                let trimmed_bytes: Vec<u8> = trimmed.copied().collect();
                if !trimmed_bytes.is_empty() && trimmed_bytes[0] == b'#' {
                    // Has a comment — don't flag
                    return Vec::new();
                }
            }
        }

        let loc = def_node.def_keyword_loc();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, "Put empty method definitions on a single line.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(EmptyMethod, "cops/style/empty_method");
}
