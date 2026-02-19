use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, ELSE_NODE, IF_NODE};

pub struct MinMaxComparison;

impl Cop for MinMaxComparison {
    fn name(&self) -> &'static str {
        "Style/MinMaxComparison"
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
    ) -> Vec<Diagnostic> {
        // Looking for: a > b ? a : b  (max)  or  a < b ? a : b  (min), etc.
        let ternary = match node.as_if_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Must be a ternary (has if_keyword "?" syntax) -- check for consequent and alternative
        let consequent = match ternary.statements() {
            Some(s) => s,
            None => return Vec::new(),
        };
        let alternative = match ternary.subsequent() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Must be a ternary expression.
        // In Prism, ternary (a ? b : c) has if_keyword_loc() == None.
        // Regular if/unless has if_keyword_loc() == Some("if"/"unless").
        if ternary.if_keyword_loc().is_some() {
            return Vec::new();
        }

        // The condition must be a comparison: a > b, a >= b, a < b, a <= b
        let condition = match ternary.predicate() {
            c => c,
        };

        let cmp_call = match condition.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let op = cmp_call.name();
        let op_bytes = op.as_slice();
        if op_bytes != b">" && op_bytes != b">=" && op_bytes != b"<" && op_bytes != b"<=" {
            return Vec::new();
        }

        let cmp_lhs = match cmp_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let cmp_args = match cmp_call.arguments() {
            Some(args) => args,
            None => return Vec::new(),
        };
        let cmp_arg_list: Vec<_> = cmp_args.arguments().iter().collect();
        if cmp_arg_list.len() != 1 {
            return Vec::new();
        }
        let cmp_rhs = &cmp_arg_list[0];

        // Get consequent and alternative expressions
        let cons_stmts: Vec<_> = consequent.body().iter().collect();
        if cons_stmts.len() != 1 {
            return Vec::new();
        }
        let cons_expr = &cons_stmts[0];

        // alternative is an ElseNode
        let else_node = match alternative.as_else_node() {
            Some(e) => e,
            None => return Vec::new(),
        };
        let alt_stmts = match else_node.statements() {
            Some(s) => s,
            None => return Vec::new(),
        };
        let alt_body: Vec<_> = alt_stmts.body().iter().collect();
        if alt_body.len() != 1 {
            return Vec::new();
        }
        let alt_expr = &alt_body[0];

        // Compare source text of expressions
        let lhs_src = std::str::from_utf8(cmp_lhs.location().as_slice()).unwrap_or("");
        let rhs_src = std::str::from_utf8(cmp_rhs.location().as_slice()).unwrap_or("");
        let cons_src = std::str::from_utf8(cons_expr.location().as_slice()).unwrap_or("");
        let alt_src = std::str::from_utf8(alt_expr.location().as_slice()).unwrap_or("");

        // Determine if it's max or min pattern
        let suggestion = match op_bytes {
            // a > b ? a : b  => max  |  a > b ? b : a  => min
            b">" | b">=" => {
                if lhs_src == cons_src && rhs_src == alt_src {
                    "max"
                } else if lhs_src == alt_src && rhs_src == cons_src {
                    "min"
                } else {
                    return Vec::new();
                }
            }
            // a < b ? a : b  => min  |  a < b ? b : a  => max
            b"<" | b"<=" => {
                if lhs_src == cons_src && rhs_src == alt_src {
                    "min"
                } else if lhs_src == alt_src && rhs_src == cons_src {
                    "max"
                } else {
                    return Vec::new();
                }
            }
            _ => return Vec::new(),
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `[{lhs_src}, {rhs_src}].{suggestion}` instead."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MinMaxComparison, "cops/style/min_max_comparison");
}
