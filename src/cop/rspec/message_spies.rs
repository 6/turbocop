use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MessageSpies;

/// Default style is `have_received` — flags `expect(...).to receive(...)`.
impl Cop for MessageSpies {
    fn name(&self) -> &'static str {
        "RSpec/MessageSpies"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return Vec::new();
        }

        // Check receiver is `expect(...)`
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if recv_call.name().as_slice() != b"expect" || recv_call.receiver().is_some() {
            return Vec::new();
        }

        // Check that the matcher argument is `receive` (not `have_received`)
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // Walk into chained calls to find the root matcher name
        let matcher_call = find_root_call(&arg_list[0]);
        let matcher_call = match matcher_call {
            Some(c) => c,
            None => return Vec::new(),
        };

        if matcher_call.name().as_slice() != b"receive" || matcher_call.receiver().is_some() {
            return Vec::new();
        }

        let loc = matcher_call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        // Only flag the `receive` part
        let len = b"receive".len();
        let _ = len;
        vec![self.diagnostic(
            source,
            line,
            column,
            "Prefer `have_received` for setting message expectations. Setup the object as a spy using `allow` or `instance_spy`.".to_string(),
        )]
    }
}

fn find_root_call<'a>(node: &ruby_prism::Node<'a>) -> Option<ruby_prism::CallNode<'a>> {
    let call = node.as_call_node()?;
    // Walk receiver chain to find the root
    let mut current = call;
    while let Some(recv) = current.receiver() {
        if let Some(inner) = recv.as_call_node() {
            current = inner;
        } else {
            break;
        }
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MessageSpies, "cops/rspec/message_spies");
}
