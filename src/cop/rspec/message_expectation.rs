use crate::cop::node_type::CALL_NODE;
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
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
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
        // The first arg could be `receive(...)` directly or chained like `receive(...).with(...)`
        let receive_call = if let Some(c) = first_arg.as_call_node() {
            // Could be `receive(...)` or `receive(...).with(...)` etc
            if c.name().as_slice() == b"receive" && c.receiver().is_none() {
                Some(c)
            } else if let Some(recv) = c.receiver() {
                // Check if the receiver chain eventually has `receive`
                recv.as_call_node().filter(|inner| {
                    inner.name().as_slice() == b"receive" && inner.receiver().is_none()
                })
            } else {
                None
            }
        } else {
            None
        };

        if receive_call.is_none() {
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
