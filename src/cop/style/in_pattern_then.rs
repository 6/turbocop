use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::IN_NODE;

pub struct InPatternThen;

impl Cop for InPatternThen {
    fn name(&self) -> &'static str {
        "Style/InPatternThen"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IN_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check for `in` pattern nodes in case-in expressions
        let in_node = match node.as_in_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Get the source between pattern and body
        let pattern = in_node.pattern();
        let pattern_end = pattern.location().end_offset();

        // Check if there's a semicolon between the pattern and body (then-like separator)
        // Look at the source bytes between pattern end and body/statements start
        let src = source.as_bytes();
        if let Some(stmts) = in_node.statements() {
            let stmts_start = stmts.location().start_offset();
            let between = &src[pattern_end..stmts_start];
            // Check if there's a semicolon (`;`) in the gap
            if between.contains(&b';') {
                // This is `in pattern; body` — should use `in pattern then body`
                let semi_offset = pattern_end + between.iter().position(|&b| b == b';').unwrap();
                let (line, column) = source.offset_to_line_col(semi_offset);
                let pattern_src = String::from_utf8_lossy(pattern.location().as_slice());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Do not use `in {}`. Use `in {} then` instead.", pattern_src, pattern_src),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InPatternThen, "cops/style/in_pattern_then");
}
