use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, OR_NODE};

pub struct Blank;

/// Extract the receiver source text from a CallNode, returning None if absent.
fn receiver_source<'a>(call: &ruby_prism::CallNode<'a>) -> Option<&'a [u8]> {
    call.receiver().map(|r| r.location().as_slice())
}

/// Check if the left side of an OR node matches a nil-check pattern:
/// - `foo.nil?`
/// - `foo == nil`
/// - `nil == foo`
/// - `!foo`
/// Returns (receiver source bytes, left side source bytes) if matched.
fn nil_check_receiver<'a>(node: &ruby_prism::Node<'a>) -> Option<(&'a [u8], &'a [u8])> {
    let call = node.as_call_node()?;
    let method = call.name().as_slice();
    let left_src = node.location().as_slice();

    if method == b"nil?" {
        // foo.nil?
        return receiver_source(&call).map(|r| (r, left_src));
    }

    if method == b"==" {
        // foo == nil  or  nil == foo
        let recv = call.receiver()?;
        let args = call.arguments()?;
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return None;
        }
        let arg = &arg_list[0];

        if arg.as_nil_node().is_some() {
            // foo == nil → receiver is foo
            return Some((recv.location().as_slice(), left_src));
        }
        if recv.as_nil_node().is_some() {
            // nil == foo → receiver is arg
            return Some((arg.location().as_slice(), left_src));
        }
    }

    if method == b"!" {
        // !foo
        return receiver_source(&call).map(|r| (r, left_src));
    }

    None
}

impl Cop for Blank {
    fn name(&self) -> &'static str {
        "Rails/Blank"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, OR_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let nil_or_empty = config.get_bool("NilOrEmpty", true);
        let not_present = config.get_bool("NotPresent", true);
        let _unless_present = config.get_bool("UnlessPresent", true);

        // NilOrEmpty: foo.nil? || foo.empty?
        if nil_or_empty {
            if let Some(or_node) = node.as_or_node() {
                let left = or_node.left();
                let right = or_node.right();

                if let Some((nil_recv, left_src)) = nil_check_receiver(&left) {
                    // Right side must be `<same>.empty?`
                    if let Some(right_call) = right.as_call_node() {
                        if right_call.name().as_slice() == b"empty?" {
                            if let Some(empty_recv) = receiver_source(&right_call) {
                                if nil_recv == empty_recv {
                                    let loc = node.location();
                                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                                    let recv_str = std::str::from_utf8(nil_recv).unwrap_or("object");
                                    let left_str = std::str::from_utf8(left_src).unwrap_or("nil?");
                                    let right_str = std::str::from_utf8(right.location().as_slice()).unwrap_or("empty?");
                                    diagnostics.push(self.diagnostic(
                                        source,
                                        line,
                                        column,
                                        format!("Use `{recv_str}.blank?` instead of `{left_str} || {right_str}`."),
                                    ));
                                }
                            }
                        }
                    }
                }
                return;
            }
        }

        // NotPresent: !foo.present?
        if not_present {
            let call = match node.as_call_node() {
                Some(c) => c,
                None => return,
            };

            if call.name().as_slice() != b"!" {
                return;
            }

            let receiver = match call.receiver() {
                Some(r) => r,
                None => return,
            };

            let inner_call = match receiver.as_call_node() {
                Some(c) => c,
                None => return,
            };

            if inner_call.name().as_slice() != b"present?" {
                return;
            }

            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Use `blank?` instead of `!present?`.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Blank, "cops/rails/blank");
}
