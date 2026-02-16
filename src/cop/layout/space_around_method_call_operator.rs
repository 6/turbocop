use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceAroundMethodCallOperator;

impl Cop for SpaceAroundMethodCallOperator {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundMethodCallOperator"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Handle CallNode (method calls with . or &.)
        if let Some(call) = node.as_call_node() {
            if let Some(dot_loc) = call.call_operator_loc() {
                let dot_slice = dot_loc.as_slice();
                // Only check . and &. operators
                if dot_slice == b"." || dot_slice == b"&." {
                    // Check space before dot (between receiver end and dot start)
                    if let Some(receiver) = call.receiver() {
                        let recv_end = receiver.location().end_offset();
                        let dot_start = dot_loc.start_offset();
                        if dot_start > recv_end {
                            let bytes = &source.as_bytes()[recv_end..dot_start];
                            if bytes.iter().all(|&b| b == b' ' || b == b'\t') && !bytes.is_empty() {
                                // Space before dot on the same line
                                let (recv_end_line, _) = source.offset_to_line_col(recv_end);
                                let (dot_start_line, _) = source.offset_to_line_col(dot_start);
                                if recv_end_line == dot_start_line {
                                    let (line, col) = source.offset_to_line_col(recv_end);
                                    diagnostics.push(self.diagnostic(
                                        source,
                                        line,
                                        col,
                                        "Avoid using spaces around a method call operator."
                                            .to_string(),
                                    ));
                                }
                            }
                        }
                    }

                    // Check space after dot (between dot end and method start)
                    if let Some(msg_loc) = call.message_loc() {
                        let dot_end = dot_loc.end_offset();
                        let msg_start = msg_loc.start_offset();
                        if msg_start > dot_end {
                            let bytes = &source.as_bytes()[dot_end..msg_start];
                            if bytes.iter().all(|&b| b == b' ' || b == b'\t') && !bytes.is_empty() {
                                let (dot_end_line, _) = source.offset_to_line_col(dot_end);
                                let (msg_start_line, _) = source.offset_to_line_col(msg_start);
                                if dot_end_line == msg_start_line {
                                    let (line, col) = source.offset_to_line_col(dot_end);
                                    diagnostics.push(self.diagnostic(
                                        source,
                                        line,
                                        col,
                                        "Avoid using spaces around a method call operator."
                                            .to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Handle ConstantPathNode (:: operator)
        if let Some(cp) = node.as_constant_path_node() {
            // Only check when there's a name (e.g., `Foo::Bar`, not bare `::`)
            if cp.name().is_some() {
                let delim_loc = cp.delimiter_loc();
                let delim_end = delim_loc.end_offset();
                let name_loc = cp.name_loc();
                let name_start = name_loc.start_offset();
                if name_start > delim_end {
                    let bytes = &source.as_bytes()[delim_end..name_start];
                    if bytes.iter().all(|&b| b == b' ' || b == b'\t') && !bytes.is_empty() {
                        let (delim_line, _) = source.offset_to_line_col(delim_end);
                        let (name_line, _) = source.offset_to_line_col(name_start);
                        if delim_line == name_line {
                            let (line, col) = source.offset_to_line_col(delim_end);
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                col,
                                "Avoid using spaces around a method call operator.".to_string(),
                            ));
                        }
                    }
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SpaceAroundMethodCallOperator,
        "cops/layout/space_around_method_call_operator"
    );
}
