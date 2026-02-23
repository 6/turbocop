use crate::cop::node_type::{CALL_NODE, ELSE_NODE, IF_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ComparableClamp;

impl Cop for ComparableClamp {
    fn name(&self) -> &'static str {
        "Style/ComparableClamp"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, ELSE_NODE, IF_NODE]
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
        // Pattern: if x < low then low elsif x > high then high else x end
        // (or with > / reversed operand positions)
        // Must match RuboCop's exact structural pattern:
        // - The if body must equal the bound from the condition
        // - The elsif body must equal the bound from the condition
        // - The else body must equal the clamped variable
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return,
        };

        // Skip elsif nodes — only check outermost if
        if if_node.if_keyword_loc().is_none() {
            return;
        }
        // Also skip if the keyword is not "if" (could be ternary or modifier)
        if if_node.if_keyword_loc().unwrap().as_slice() != b"if" {
            return;
        }

        // Must have exactly one elsif and an else
        let elsif = match if_node.subsequent() {
            Some(s) => s,
            None => return,
        };

        let elsif_node = match elsif.as_if_node() {
            Some(n) => n,
            None => return, // It's a plain else, not elsif
        };

        // The elsif must have an else (no more elsifs)
        let else_clause = match elsif_node.subsequent() {
            Some(s) => s,
            None => return,
        };

        // Should not have another elsif
        if else_clause.as_if_node().is_some() {
            return;
        }

        // Get the else body as source text
        let else_body = match else_clause.as_else_node() {
            Some(e) => e,
            None => return,
        };
        let else_body_src = get_single_stmt_src(else_body.statements(), source);
        let else_body_src = match else_body_src {
            Some(s) => s,
            None => return,
        };

        // Get the if body source
        let if_body_src = get_single_stmt_src(if_node.statements(), source);
        let if_body_src = match if_body_src {
            Some(s) => s,
            None => return,
        };

        // Get the elsif body source
        let elsif_body_src = get_single_stmt_src(elsif_node.statements(), source);
        let elsif_body_src = match elsif_body_src {
            Some(s) => s,
            None => return,
        };

        // Check conditions: both must be comparisons with < or >
        let first_cmp = get_comparison(&if_node.predicate());
        let second_cmp = get_comparison(&elsif_node.predicate());

        let (f_left, f_op, f_right) = match first_cmp {
            Some(c) => c,
            None => return,
        };
        let (s_left, s_op, s_right) = match second_cmp {
            Some(c) => c,
            None => return,
        };

        // Match one of the 8 patterns from RuboCop:
        // The else body must be the clamped variable x.
        // The if body must be one bound, the elsif body the other.
        // Each condition must compare x with the respective bound.
        let is_clamp = is_clamp_pattern(
            &f_left,
            f_op,
            &f_right,
            &if_body_src,
            &s_left,
            s_op,
            &s_right,
            &elsif_body_src,
            &else_body_src,
        );

        if is_clamp {
            let loc = if_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Use `clamp` instead of `if/elsif/else`.".to_string(),
            ));
        }
    }
}

/// Get source text of a single statement in a StatementsNode.
fn get_single_stmt_src(
    stmts: Option<ruby_prism::StatementsNode<'_>>,
    source: &SourceFile,
) -> Option<String> {
    let stmts = stmts?;
    let body: Vec<_> = stmts.body().iter().collect();
    if body.len() != 1 {
        return None;
    }
    let loc = body[0].location();
    let src = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
    Some(String::from_utf8_lossy(src).to_string())
}

struct Comparison {
    left: String,
    op: u8, // b'<' or b'>'
    right: String,
}

/// Extract comparison operands and operator from `x < y` or `x > y`.
fn get_comparison(node: &ruby_prism::Node<'_>) -> Option<(String, u8, String)> {
    let call = node.as_call_node()?;
    let method = call.name().as_slice();
    let op = match method {
        b"<" => b'<',
        b">" => b'>',
        _ => return None,
    };
    let receiver = call.receiver()?;
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() != 1 {
        return None;
    }
    let left_loc = receiver.location();
    let right_loc = arg_list[0].location();
    // Use location slice as source text
    let left = String::from_utf8_lossy(left_loc.as_slice()).to_string();
    let right = String::from_utf8_lossy(right_loc.as_slice()).to_string();
    Some((left, op, right))
}

/// Check if the if/elsif/else matches any of the 8 clamp patterns:
/// Pattern: if (x < min) then min elsif (x > max) then max else x end
/// (or with reversed operands / operators)
#[allow(clippy::too_many_arguments)] // pattern-matching all branch components
fn is_clamp_pattern(
    f_left: &str,
    f_op: u8,
    f_right: &str,
    if_body: &str,
    s_left: &str,
    s_op: u8,
    s_right: &str,
    elsif_body: &str,
    else_body: &str,
) -> bool {
    // Determine x and bound from first condition
    // Pattern 1: x < min → body is min, so x is the other operand
    // Pattern 2: min > x → body is min, so x is the other operand
    let (x_from_first, bound1) = if f_op == b'<' && f_right == if_body {
        // x < min → body is min → x = f_left, bound = f_right
        (f_left, f_right)
    } else if f_op == b'>' && f_left == if_body {
        // min > x → body is min → x = f_right, bound = f_left
        (f_right, f_left)
    } else if f_op == b'>' && f_right == if_body {
        // x > max → body is max → (this is the max-first variant)
        (f_left, f_right)
    } else if f_op == b'<' && f_left == if_body {
        // max < x → body is max → x = f_right, bound = f_left
        (f_right, f_left)
    } else {
        return false;
    };

    // The else body must be x
    if else_body != x_from_first {
        return false;
    }

    // Check second condition: x compared with bound2, and elsif body is bound2
    let (x_from_second, bound2) = if s_op == b'<' && s_right == elsif_body {
        (s_left, s_right)
    } else if s_op == b'>' && s_left == elsif_body {
        (s_right, s_left)
    } else if s_op == b'>' && s_right == elsif_body {
        (s_left, s_right)
    } else if s_op == b'<' && s_left == elsif_body {
        (s_right, s_left)
    } else {
        return false;
    };

    // x must be the same in both conditions
    if x_from_first != x_from_second {
        return false;
    }

    // bound1 and bound2 must be different
    if bound1 == bound2 {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ComparableClamp, "cops/style/comparable_clamp");
}
