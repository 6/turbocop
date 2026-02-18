use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ComparableBetween;

impl Cop for ComparableBetween {
    fn name(&self) -> &'static str {
        "Style/ComparableBetween"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check for `x >= min && x <= max` pattern
        if let Some(and_node) = node.as_and_node() {
            return check_between(self, source, &and_node.left(), &and_node.right());
        }

        Vec::new()
    }
}

fn check_between(
    cop: &ComparableBetween,
    source: &SourceFile,
    left: &ruby_prism::Node<'_>,
    right: &ruby_prism::Node<'_>,
) -> Vec<Diagnostic> {
    let left_cmp = parse_comparison(source, left);
    let right_cmp = parse_comparison(source, right);

    if let (Some(l), Some(r)) = (left_cmp, right_cmp) {
        // Check for patterns like: x >= min && x <= max
        // or: min <= x && x <= max
        let x_gte_min = (l.op == ">=" && r.op == "<=") || (l.op == "<=" && r.op == ">=");
        let x_lte_max = (l.op == "<=" && r.op == ">=") || (l.op == ">=" && r.op == "<=");

        if x_gte_min || x_lte_max {
            // Check that the same variable is used in both
            if l.left == r.left || l.left == r.right || l.right == r.left || l.right == r.right {
                let loc = left.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![cop.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer `between?` over logical comparison.".to_string(),
                )];
            }
        }
    }

    Vec::new()
}

struct Comparison {
    left: String,
    op: String,
    right: String,
}

fn parse_comparison(source: &SourceFile, node: &ruby_prism::Node<'_>) -> Option<Comparison> {
    let call = node.as_call_node()?;
    let method = std::str::from_utf8(call.name().as_slice()).ok()?;

    if !matches!(method, ">=" | "<=" | ">" | "<") {
        return None;
    }

    let receiver = call.receiver()?;
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() != 1 {
        return None;
    }

    let left_text = std::str::from_utf8(
        &source.as_bytes()[receiver.location().start_offset()..receiver.location().end_offset()],
    )
    .ok()?
    .to_string();

    let right_text = std::str::from_utf8(
        &source.as_bytes()[arg_list[0].location().start_offset()..arg_list[0].location().end_offset()],
    )
    .ok()?
    .to_string();

    Some(Comparison {
        left: left_text,
        op: method.to_string(),
        right: right_text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ComparableBetween, "cops/style/comparable_between");
}
