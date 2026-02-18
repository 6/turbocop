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

        // Both conditions must be comparisons with < or >
        let first_operands = get_comparison_operands(&first_cond);
        let second_operands = get_comparison_operands(&second_cond);

        // Both must be comparisons and must compare a common variable
        if let (Some((f_left, f_right)), Some((s_left, s_right))) = (first_operands, second_operands) {
            // The clamped variable appears in both conditions - check all combinations
            let has_common = f_left == s_left
                || f_left == s_right
                || f_right == s_left
                || f_right == s_right;
            if has_common {
                let loc = if_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `clamp` instead of `if/elsif/else`.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

/// Extract both operands from a comparison like `x < low` or `x > high`.
fn get_comparison_operands(node: &ruby_prism::Node<'_>) -> Option<(Vec<u8>, Vec<u8>)> {
    if let Some(call) = node.as_call_node() {
        let method = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if matches!(method, "<" | ">" | "<=" | ">=") {
            if let Some(receiver) = call.receiver() {
                let args = call.arguments()?;
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() == 1 {
                    let left = receiver.location().as_slice().to_vec();
                    let right = arg_list[0].location().as_slice().to_vec();
                    return Some((left, right));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ComparableClamp, "cops/style/comparable_clamp");
}
