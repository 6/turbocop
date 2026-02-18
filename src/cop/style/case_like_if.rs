use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct CaseLikeIf;

impl Cop for CaseLikeIf {
    fn name(&self) -> &'static str {
        "Style/CaseLikeIf"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let min_branches = config.get_usize("MinBranchesCount", 3);

        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Count branches (if + elsif chain)
        let mut branch_count = 1;
        let mut current_else = if_node.subsequent();
        while let Some(else_clause) = current_else {
            if let Some(elsif) = else_clause.as_if_node() {
                branch_count += 1;
                current_else = elsif.subsequent();
            } else {
                // else clause
                break;
            }
        }

        if branch_count < min_branches {
            return Vec::new();
        }

        // Check that all conditions compare against the same variable
        // using ==, ===, is_a?, kind_of?, match?, etc.
        // Simplified: just check for if-elsif chains with enough branches
        let predicate = if_node.predicate();
        if is_comparison(&predicate) {
            let loc = if_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());

            // Get the line content to determine the range
            let end_offset = if let Some(end_kw) = if_node.end_keyword_loc() {
                end_kw.end_offset()
            } else {
                loc.end_offset()
            };
            let (end_line, _) = source.offset_to_line_col(end_offset);
            let _ = end_line;

            return vec![self.diagnostic(
                source,
                line,
                column,
                "Convert `if-elsif` to `case-when`.".to_string(),
            )];
        }

        Vec::new()
    }
}

fn is_comparison(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let method = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        return matches!(method, "==" | "===" | "is_a?" | "kind_of?" | "match?" | "=~");
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CaseLikeIf, "cops/style/case_like_if");
}
