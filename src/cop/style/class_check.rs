use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Matches both explicit- and implicit-receiver `is_a?`/`kind_of?` sends.
///
/// The corpus false negatives here were receiverless calls such as
/// `if kind_of?(ExtManagementSystem)`, which RuboCop flags via `on_send`.
/// The previous implementation returned early unless Prism reported a receiver.
pub struct ClassCheck;

impl Cop for ClassCheck {
    fn name(&self) -> &'static str {
        "Style/ClassCheck"
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
        let enforced_style = config.get_str("EnforcedStyle", "is_a?");

        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call_node.name();
        let method_bytes = method_name.as_slice();

        // Must be is_a? or kind_of?
        if method_bytes != b"is_a?" && method_bytes != b"kind_of?" {
            return;
        }

        // Check against enforced style
        let (prefer, current) = if enforced_style == "is_a?" {
            ("is_a?", "kind_of?")
        } else {
            ("kind_of?", "is_a?")
        };

        // Only flag the non-preferred style
        if method_bytes != current.as_bytes() {
            return;
        }

        let msg_loc = call_node
            .message_loc()
            .unwrap_or_else(|| call_node.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Prefer `Object#{}` over `Object#{}`.", prefer, current),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(ClassCheck, "cops/style/class_check");

    #[test]
    fn flags_receiverless_is_a_when_kind_of_is_enforced() {
        let mut config = CopConfig::default();
        config.options.insert(
            "EnforcedStyle".to_string(),
            serde_yml::Value::String("kind_of?".into()),
        );

        crate::testutil::assert_cop_offenses_full_with_config(
            &ClassCheck,
            b"is_a?(Date)\n^^^^^ Style/ClassCheck: Prefer `Object#kind_of?` over `Object#is_a?`.\n",
            config,
        );
    }
}
