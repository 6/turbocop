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
        // Detect patterns where the same variable x satisfies: low <= x && x <= high
        // This covers all equivalent forms:
        //   x >= min && x <= max   =>  x is l.left and r.left
        //   min <= x && x <= max   =>  x is l.right and r.left
        //   x <= max && x >= min   =>  x is l.left and r.left
        //   max >= x && min <= x   =>  x is l.right and r.right
        //
        // The key insight: each comparison must have one side as ">=" or "<="
        // and the shared variable must be on the "greater-or-equal" side of one
        // comparison and the "less-or-equal" side of the other.

        // Only consider >= and <= operators (not > or <)
        if !matches!(l.op.as_str(), ">=" | "<=") || !matches!(r.op.as_str(), ">=" | "<=") {
            return Vec::new();
        }

        // Determine which side of each comparison the variable is on
        // For `a >= b`, a is the "big" side, b is the "small" side
        // For `a <= b`, a is the "small" side, b is the "big" side
        let (l_small, l_big) = if l.op == ">=" {
            (&l.right, &l.left) // a >= b means b <= a
        } else {
            (&l.left, &l.right) // a <= b
        };

        let (r_small, r_big) = if r.op == ">=" {
            (&r.right, &r.left)
        } else {
            (&r.left, &r.right)
        };

        // The pattern is: x >= min && x <= max, which normalizes to:
        // l_big is the shared variable (x), r_small is the shared variable (x)
        // i.e. l_big == r_small
        // Also handle reversed form: x <= max && x >= min
        // where l_small == r_big
        if l_big == r_small || l_small == r_big {
            let and_node_loc = left.location();
            let (line, column) = source.offset_to_line_col(and_node_loc.start_offset());
            return vec![cop.diagnostic(
                source,
                line,
                column,
                "Prefer `between?` over logical comparison.".to_string(),
            )];
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
