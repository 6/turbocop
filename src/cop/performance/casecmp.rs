use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Casecmp;

/// Check if a node is a valid RHS for casecmp: string literal, downcase/upcase call,
/// or parenthesized string.
fn is_valid_casecmp_operand(node: &ruby_prism::Node<'_>) -> bool {
    // String literal
    if node.as_string_node().is_some() || node.as_interpolated_string_node().is_some() {
        return true;
    }
    // downcase/upcase call
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if (name == b"downcase" || name == b"upcase") && call.arguments().is_none() {
            return true;
        }
    }
    // Parenthesized string: (begin str)
    if let Some(parens) = node.as_parentheses_node() {
        if let Some(body) = parens.body() {
            if let Some(stmts) = body.as_statements_node() {
                let body_nodes: Vec<_> = stmts.body().iter().collect();
                if body_nodes.len() == 1 {
                    let inner = &body_nodes[0];
                    if inner.as_string_node().is_some()
                        || inner.as_interpolated_string_node().is_some()
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Check if a call node has safe navigation (&.) operator.
fn has_safe_navigation(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(op) = call.call_operator_loc() {
        return op.as_slice() == b"&.";
    }
    false
}

/// Check if a call is a downcase/upcase call with no arguments and no safe navigation.
fn is_case_method(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    (name == b"downcase" || name == b"upcase")
        && call.arguments().is_none()
        && !has_safe_navigation(call)
}

impl Cop for Casecmp {
    fn name(&self) -> &'static str {
        "Performance/Casecmp"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = outer_call.name().as_slice();

        // Handle == and != operators
        if method == b"==" || method == b"!=" {
            let receiver = match outer_call.receiver() {
                Some(r) => r,
                None => return,
            };

            let args: Vec<_> = match outer_call.arguments() {
                Some(a) => a.arguments().iter().collect(),
                None => return,
            };
            if args.len() != 1 {
                return;
            }
            let rhs = &args[0];

            // Pattern 1: x.downcase == valid_rhs
            if let Some(recv_call) = receiver.as_call_node() {
                if is_case_method(&recv_call) && is_valid_casecmp_operand(rhs) {
                    let case_method =
                        std::str::from_utf8(recv_call.name().as_slice()).unwrap_or("downcase");
                    let op = std::str::from_utf8(method).unwrap_or("==");
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Use `casecmp` instead of `{case_method} {op}`."),
                    ));
                    return;
                }
            }

            // Pattern 2: valid_lhs == x.downcase (reversed operand order)
            if let Some(rhs_call) = rhs.as_call_node() {
                if is_case_method(&rhs_call) && is_valid_casecmp_operand(&receiver) {
                    let case_method =
                        std::str::from_utf8(rhs_call.name().as_slice()).unwrap_or("downcase");
                    let op = std::str::from_utf8(method).unwrap_or("==");
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Use `casecmp` instead of `{op} {case_method}`."),
                    ));
                    return;
                }
            }
        }

        // Handle eql? method: x.downcase.eql?(y)
        if method == b"eql?" {
            let receiver = match outer_call.receiver() {
                Some(r) => r,
                None => return,
            };

            // receiver should be a downcase/upcase call
            let recv_call = match receiver.as_call_node() {
                Some(c) => c,
                None => return,
            };

            if !is_case_method(&recv_call) {
                return;
            }

            // Get the argument to eql?
            let args: Vec<_> = match outer_call.arguments() {
                Some(a) => a.arguments().iter().collect(),
                None => return,
            };
            if args.len() != 1 {
                return;
            }

            if is_valid_casecmp_operand(&args[0]) {
                let case_method =
                    std::str::from_utf8(recv_call.name().as_slice()).unwrap_or("downcase");
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `casecmp` instead of `{case_method} eql?`."),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Casecmp, "cops/performance/casecmp");
}
