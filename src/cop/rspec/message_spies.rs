use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Default style is `have_received` — flags `expect(...).to receive(...)`.
///
/// Corpus investigation (2026-03-29):
///
/// - FN=3 in rspec/rspec: `receive` nested inside the argument passed to
///   `expect(...)`, e.g.
///   `expect(allow(test_double).to receive(:foo)).to have_string_representation(...)`.
///   RuboCop captures the argument to `expect(...)` for the message text, then
///   runs a subtree search over the full expectation node, so it still finds the
///   nested `receive`. nitrocop only searched `.to(...)` matcher arguments,
///   missing those cases and falling back to a generic message text.
/// - FP=1 in nats-io/nats-pure.rb: `expect { ... }.to receive(:stop)` was
///   flagged. RuboCop's matcher requires a plain `expect(...)` send as the
///   receiver, not block-form `expect { ... }`.
///
/// Fixed by:
/// - searching the full expectation subtree for `receive`/`have_received`
/// - extracting the sole argument to `expect(...)` for the dynamic message text
/// - skipping block-form `expect { ... }`
///
/// Block-body FP fix (2026-03-31):
///
/// - FP=18 came from `do...end` blocks attached to the expectation chain, for example
///   `expect(foo).to have_received(:bar) do ... allow(baz).to receive(:qux) end`.
///   RuboCop still flags the outer `expect(foo).to receive(:bar) do ... end`
///   form, but it does not descend into those `do...end` bodies and does not
///   treat inner `allow(...).to receive(...)` setup calls as offenses for the
///   outer expectation.
/// - Brace blocks are different: `expect(foo).to receive(:bar) { allow(baz).to
///   receive(:qux) }` keeps the block inside the matcher argument subtree, and
///   RuboCop flags both `receive` selectors.
/// - nitrocop searched every attached `call.block()` recursively, which made
///   `do...end` implementation blocks part of the matcher subtree and produced
///   false positives on those inner setup calls.
/// - Fixed by keeping the receiver/argument recursion and only descending into
///   attached brace blocks while searching for matcher calls.
pub struct MessageSpies;

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
        // Config: EnforcedStyle — "have_received" (default) or "receive"
        let enforced_style = config.get_str("EnforcedStyle", "have_received");
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return;
        }

        // Check receiver is `expect(...)`
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if recv_call.name().as_slice() != b"expect" || recv_call.receiver().is_some() {
            return;
        }

        // RuboCop only matches plain `expect(...)`, not block-form
        // `expect { ... }`, which Prism still exposes as a CallNode with a
        // block attached.
        if recv_call.block().is_some() {
            return;
        }

        let (expect_arg_start, expect_arg_end) = match sole_expect_argument_range(recv_call) {
            Some(range) => range,
            None => return,
        };

        let target_name = if enforced_style == "receive" {
            b"have_received" as &[u8]
        } else {
            b"receive" as &[u8]
        };

        let mut found = Vec::new();
        find_matcher_calls(source, node, target_name, &mut found);
        if found.is_empty() {
            return;
        }

        let message = if enforced_style == "receive" {
            "Prefer `receive` for setting message expectations.".to_string()
        } else {
            let receiver_source = source.byte_slice(expect_arg_start, expect_arg_end, "the object");
            format!(
                "Prefer `have_received` for setting message expectations. Setup `{receiver_source}` as a spy using `allow` or `instance_spy`."
            )
        };

        push_diagnostics(self, source, diagnostics, &found, message);
    }
}

/// Recursively search a node subtree for `(send nil? target_name ...)` calls,
/// matching RuboCop's `def_node_search :receive_message` behavior.
/// Handles compound expectations like `receive(:a).and receive(:b)` where
/// the second `receive` is an argument to `.and`/`.or`, and nested cases like
/// `expect(allow(foo).to receive(:bar)).to matcher(...)` where the `receive`
/// call lives inside the argument passed to `expect(...)`.
fn find_matcher_calls(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    target_name: &[u8],
    out: &mut Vec<usize>,
) {
    if let Some(call) = node.as_call_node() {
        // Check if this is a bare `receive(...)` or `have_received(...)` call
        if call.name().as_slice() == target_name && call.receiver().is_none() {
            let loc = call.location();
            out.push(loc.start_offset());
        }
        // Recurse into receiver
        if let Some(recv) = call.receiver() {
            find_matcher_calls(source, &recv, target_name, out);
        }
        // Recurse into arguments
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                find_matcher_calls(source, &arg, target_name, out);
            }
        }
        // Brace blocks remain in RuboCop's matcher subtree for this cop, but
        // do/end blocks attached to the expectation chain do not.
        if let Some(block) = call.block().and_then(|b| b.as_block_node()) {
            if attached_block_uses_braces(source, &block) {
                if let Some(body) = block.body() {
                    find_matcher_calls(source, &body, target_name, out);
                }
            }
        }
    } else if let Some(stmts) = node.as_statements_node() {
        for child in stmts.body().iter() {
            find_matcher_calls(source, &child, target_name, out);
        }
    }
}

fn attached_block_uses_braces(source: &SourceFile, block: &ruby_prism::BlockNode<'_>) -> bool {
    let open = block.opening_loc();
    source
        .as_bytes()
        .get(open.start_offset())
        .copied()
        .is_some_and(|byte| byte == b'{')
}

fn sole_expect_argument_range(call: ruby_prism::CallNode<'_>) -> Option<(usize, usize)> {
    let args = call.arguments()?;
    let mut args = args.arguments().iter();
    let arg = args.next()?;
    if args.next().is_some() {
        return None;
    }
    let loc = arg.location();
    Some((loc.start_offset(), loc.end_offset()))
}

fn push_diagnostics(
    cop: &MessageSpies,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    offsets: &[usize],
    message: String,
) {
    for &start_offset in offsets {
        let (line, column) = source.offset_to_line_col(start_offset);
        diagnostics.push(cop.diagnostic(source, line, column, message.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MessageSpies, "cops/rspec/message_spies");

    #[test]
    fn compound_receive_produces_two_offenses() {
        let source = b"expect(foo).to receive(:bar).and receive(:baz)\n";
        let diags = crate::testutil::run_cop_full(&MessageSpies, source);
        assert_eq!(
            diags.len(),
            2,
            "compound receive should produce 2 offenses: {:?}",
            diags
        );
        // First at `receive(:bar)` column 15
        assert_eq!(diags[0].location.column, 15);
        // Second at `receive(:baz)` column 33
        assert_eq!(diags[1].location.column, 33);
    }

    #[test]
    fn receive_style_flags_have_received() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("receive".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"expect(foo).to have_received(:bar)\n";
        let diags = crate::testutil::run_cop_full_with_config(&MessageSpies, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("receive"));
    }

    #[test]
    fn nested_expect_argument_uses_dynamic_source_in_message() {
        let source =
            b"expect(allow(test_double).to receive(:foo)).to have_string_representation(\"x\")\n";
        let diags = crate::testutil::run_cop_full(&MessageSpies, source);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.column, 29);
        assert_eq!(
            diags[0].message,
            "Prefer `have_received` for setting message expectations. Setup `allow(test_double).to receive(:foo)` as a spy using `allow` or `instance_spy`."
        );
    }

    #[test]
    fn block_expectation_does_not_flag_receive() {
        let source = b"expect { subject }.to receive(:stop)\n";
        let diags = crate::testutil::run_cop_full(&MessageSpies, source);
        assert!(diags.is_empty(), "block expectations should not be flagged");
    }

    #[test]
    fn receive_with_block_only_flags_outer_receive() {
        let source = b"expect(foo).to receive(:bar) do\n  allow(baz).to receive(:qux)\nend\n";
        let diags = crate::testutil::run_cop_full(&MessageSpies, source);
        assert_eq!(diags.len(), 1, "only the outer receive should be flagged");
        assert_eq!(diags[0].location.column, 15);
        assert_eq!(
            diags[0].message,
            "Prefer `have_received` for setting message expectations. Setup `foo` as a spy using `allow` or `instance_spy`."
        );
    }

    #[test]
    fn receive_with_brace_block_flags_outer_and_inner_receive() {
        let source = b"expect(foo).to receive(:bar) { allow(baz).to receive(:qux) { quux } }\n";
        let diags = crate::testutil::run_cop_full(&MessageSpies, source);
        assert_eq!(
            diags.len(),
            2,
            "brace blocks in matcher arguments should keep inner receives searchable"
        );
        assert_eq!(diags[0].location.column, 15);
        assert_eq!(diags[1].location.column, 45);
        assert_eq!(
            diags[0].message,
            "Prefer `have_received` for setting message expectations. Setup `foo` as a spy using `allow` or `instance_spy`."
        );
        assert_eq!(diags[1].message, diags[0].message);
    }
}
