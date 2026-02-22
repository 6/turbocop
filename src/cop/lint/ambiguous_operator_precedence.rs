use crate::cop::node_type::{AND_NODE, CALL_NODE, OR_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct AmbiguousOperatorPrecedence;

// Precedence levels (lower index = higher precedence)
const PRECEDENCE: &[&[&[u8]]] = &[
    &[b"**"],
    &[b"*", b"/", b"%"],
    &[b"+", b"-"],
    &[b"<<", b">>"],
    &[b"&"],
    &[b"|", b"^"],
];

fn precedence_level(op: &[u8]) -> Option<usize> {
    for (i, group) in PRECEDENCE.iter().enumerate() {
        if group.iter().any(|o| *o == op) {
            return Some(i);
        }
    }
    None
}

impl Cop for AmbiguousOperatorPrecedence {
    fn name(&self) -> &'static str {
        "Lint/AmbiguousOperatorPrecedence"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[AND_NODE, CALL_NODE, OR_NODE]
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
        // Handle `a || b && c` pattern
        if let Some(or_node) = node.as_or_node() {
            diagnostics.extend(self.check_or_and(source, &or_node));
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = call.name().as_slice();
        let outer_prec = match precedence_level(method) {
            Some(p) => p,
            None => return,
        };

        // Check arguments for higher-precedence operators
        // e.g., `a + b * c`: outer is `+` (prec 2), arg `b * c` is `*` (prec 1)
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if let Some(arg_call) = arg.as_call_node() {
                    let arg_method = arg_call.name().as_slice();
                    if let Some(arg_prec) = precedence_level(arg_method) {
                        if arg_prec < outer_prec {
                            // arg has higher precedence than outer
                            let loc = arg_call.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                "Wrap expressions with varying precedence with parentheses to avoid ambiguity.".to_string(),
                            ));
                        }
                    }
                }
            }
        }

        // Check if receiver is a lower-precedence operator
        // e.g., `a ** b + c`: outer is `+` (prec 2), recv `a ** b` is `**` (prec 0)
        if let Some(recv) = call.receiver() {
            if let Some(recv_call) = recv.as_call_node() {
                let recv_method = recv_call.name().as_slice();
                if let Some(recv_prec) = precedence_level(recv_method) {
                    if recv_prec < outer_prec {
                        // recv has higher precedence than outer
                        let loc = recv_call.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Wrap expressions with varying precedence with parentheses to avoid ambiguity.".to_string(),
                        ));
                    }
                }
            }
        }
    }
}

impl AmbiguousOperatorPrecedence {
    fn check_or_and(
        &self,
        source: &SourceFile,
        or_node: &ruby_prism::OrNode<'_>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if or_node.left().as_and_node().is_some() {
            let loc = or_node.left().location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(
                self.diagnostic(
                    source,
                    line,
                    column,
                    "Wrap expressions with varying precedence with parentheses to avoid ambiguity."
                        .to_string(),
                ),
            );
        }

        if or_node.right().as_and_node().is_some() {
            let loc = or_node.right().location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(
                self.diagnostic(
                    source,
                    line,
                    column,
                    "Wrap expressions with varying precedence with parentheses to avoid ambiguity."
                        .to_string(),
                ),
            );
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        AmbiguousOperatorPrecedence,
        "cops/lint/ambiguous_operator_precedence"
    );
}
