use crate::cop::node_type::CALL_NODE;
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-14)
///
/// FP=1 (rpush/rpush): `expect(fake_http2_request).to receive(:on).with(:close), &on_close`
/// was flagged. RuboCop's NodePattern `(send ... :to #receive_message?)` only matches
/// when `.to` has exactly one argument (no block_pass). When `&proc` is passed as a
/// block argument, the Parser AST adds it as a `block_pass` child of the send node,
/// making the pattern not match. Fixed by checking if `.to` has a BlockArgumentNode.
pub struct MessageExpectation;

/// Default style is `allow` — flags `expect(...).to receive` in favor of `allow`.
impl Cop for MessageExpectation {
    fn name(&self) -> &'static str {
        "RSpec/MessageExpectation"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        // Config: EnforcedStyle — "allow" (default) or "expect"
        let enforced_style = config.get_str("EnforcedStyle", "allow");

        // Look for: expect(foo).to receive(:bar)
        // The pattern is a call chain: expect(foo).to(receive(:bar))
        // We flag the `expect(...)` part.
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" {
            return;
        }

        // If .to has a &proc block argument, skip — RuboCop's NodePattern
        // (send ... :to #receive_message?) requires exactly one argument with no
        // block_pass. A block_pass adds an extra child to the send node, preventing
        // the pattern from matching.
        if call
            .block()
            .is_some_and(|b| b.as_block_argument_node().is_some())
        {
            return;
        }

        // Check the argument is `receive` or similar
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let first_arg = &arg_list[0];
        let matcher_call = match first_arg.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if !call_chain_includes_receive(matcher_call) {
            return;
        }

        // Check that the receiver of `.to` is `expect(...)` (not `allow(...)`)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let recv_name = recv_call.name().as_slice();
        if recv_call.receiver().is_some() {
            return;
        }

        if enforced_style == "expect" {
            // "expect" style: flag `allow(...).to receive(...)`, prefer `expect`
            if recv_name != b"allow" {
                return;
            }
            let loc = recv_call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer `expect` for setting message expectations.".to_string(),
            ));
        } else {
            // Default "allow" style: flag `expect(...).to receive(...)`, prefer `allow`
            if recv_name != b"expect" {
                return;
            }
            let loc = recv_call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer `allow` for setting message expectations.".to_string(),
            ));
        }
    }
}

fn call_chain_includes_receive(call: ruby_prism::CallNode<'_>) -> bool {
    if call.name().as_slice() == b"receive" && call.receiver().is_none() {
        return true;
    }

    if let Some(recv) = call.receiver() {
        if let Some(recv_call) = recv.as_call_node() {
            return call_chain_includes_receive(recv_call);
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MessageExpectation, "cops/rspec/message_expectation");

    #[test]
    fn expect_style_flags_allow_receive() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("expect".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"allow(foo).to receive(:bar)\n";
        let diags = crate::testutil::run_cop_full_with_config(&MessageExpectation, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("expect"));
    }

    #[test]
    fn expect_style_does_not_flag_expect_receive() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("expect".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"expect(foo).to receive(:bar)\n";
        let diags = crate::testutil::run_cop_full_with_config(&MessageExpectation, source, config);
        assert!(diags.is_empty());
    }
}
