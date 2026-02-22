use crate::cop::node_type::{CALL_NODE, INTEGER_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ReceiveCounts;

impl Cop for ReceiveCounts {
    fn name(&self) -> &'static str {
        "RSpec/ReceiveCounts"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTEGER_NODE]
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
        // Look for .times call
        let times_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if times_call.name().as_slice() != b"times" {
            return;
        }

        // .times must have a receiver which is exactly/at_least/at_most(n)
        let count_call = match times_call.receiver() {
            Some(r) => match r.as_call_node() {
                Some(c) => c,
                None => return,
            },
            None => return,
        };

        let count_method = count_call.name().as_slice();
        if count_method != b"exactly" && count_method != b"at_least" && count_method != b"at_most" {
            return;
        }

        // The count call must chain from a receive call
        if !has_receive_in_chain_up(&count_call) {
            return;
        }

        // Get the numeric argument
        let args = match count_call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let int_node = match arg_list[0].as_integer_node() {
            Some(i) => i,
            None => return,
        };

        let value: i64 = match std::str::from_utf8(int_node.location().as_slice()) {
            Ok(s) => match s.parse() {
                Ok(v) => v,
                Err(_) => return,
            },
            Err(_) => return,
        };

        let count_method_str = std::str::from_utf8(count_method).unwrap_or("exactly");

        let suggestion = match (count_method_str, value) {
            ("exactly", 1) => "`.once`".to_string(),
            ("exactly", 2) => "`.twice`".to_string(),
            ("at_least", 1) => "`.at_least(:once)`".to_string(),
            ("at_least", 2) => "`.at_least(:twice)`".to_string(),
            ("at_most", 1) => "`.at_most(:once)`".to_string(),
            ("at_most", 2) => "`.at_most(:twice)`".to_string(),
            _ => return,
        };

        let current = format!(".{count_method_str}({value}).times");

        let loc = count_call
            .message_loc()
            .unwrap_or_else(|| count_call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use {suggestion} instead of `{current}`."),
        ));
    }
}

fn has_receive_in_chain_up(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(recv) = call.receiver() {
        if let Some(recv_call) = recv.as_call_node() {
            let name = recv_call.name().as_slice();
            if name == b"receive" {
                return true;
            }
            return has_receive_in_chain_up(&recv_call);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ReceiveCounts, "cops/rspec/receive_counts");
}
