use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MessageExpectation;

/// Default style is `allow` — flags `expect(...).to receive` in favor of `allow`.
impl Cop for MessageExpectation {
    fn name(&self) -> &'static str {
        "RSpec/MessageExpectation"
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
        // Look for: expect(foo).to receive(:bar)
        // The pattern is a call chain: expect(foo).to(receive(:bar))
        // We flag the `expect(...)` part.
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return Vec::new();
        }

        // Check the argument is `receive` or similar
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let first_arg = &arg_list[0];
        // The first arg could be `receive(...)` directly or chained like `receive(...).with(...)`
        let receive_call = if let Some(c) = first_arg.as_call_node() {
            // Could be `receive(...)` or `receive(...).with(...)` etc
            if c.name().as_slice() == b"receive" && c.receiver().is_none() {
                Some(c)
            } else if let Some(recv) = c.receiver() {
                // Check if the receiver chain eventually has `receive`
                if let Some(inner) = recv.as_call_node() {
                    if inner.name().as_slice() == b"receive" && inner.receiver().is_none() {
                        Some(inner)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if receive_call.is_none() {
            return Vec::new();
        }

        // Check that the receiver of `.to` is `expect(...)` (not `allow(...)`)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let recv_name = recv_call.name().as_slice();
        if recv_name != b"expect" {
            return Vec::new();
        }
        // Must be receiverless expect (not obj.expect)
        if recv_call.receiver().is_some() {
            return Vec::new();
        }

        let loc = recv_call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Prefer `allow` for setting message expectations.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MessageExpectation, "cops/rspec/message_expectation");
}
