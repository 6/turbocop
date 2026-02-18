use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ComparableClamp;

impl Cop for ComparableClamp {
    fn name(&self) -> &'static str {
        "Style/ComparableClamp"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Pattern: if x < low then low elsif x > high then high else x end
        // This is complex to detect, so we check for if/elsif/else with comparison pattern
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Must have exactly one elsif and an else
        let elsif = match if_node.subsequent() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let elsif_node = match elsif.as_if_node() {
            Some(n) => n,
            None => return Vec::new(), // It's a plain else, not elsif
        };

        // The elsif must have an else (no more elsifs)
        let else_clause = match elsif_node.subsequent() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Should not have another elsif
        if else_clause.as_if_node().is_some() {
            return Vec::new();
        }

        // Check that conditions are comparisons with < or >
        let first_cond = if_node.predicate();
        let second_cond = elsif_node.predicate();

        let is_first_cmp = is_less_or_greater(&first_cond);
        let is_second_cmp = is_less_or_greater(&second_cond);

        if is_first_cmp && is_second_cmp {
            let loc = if_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Use `clamp` instead of `if/elsif/else`.".to_string(),
            )];
        }

        Vec::new()
    }
}

fn is_less_or_greater(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let method = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        return matches!(method, "<" | ">" | "<=" | ">=");
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ComparableClamp, "cops/style/comparable_clamp");
}
