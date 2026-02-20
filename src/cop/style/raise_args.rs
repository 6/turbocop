use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct RaiseArgs;

/// Extract the constant name from a node by reading its source text.
fn extract_const_name(node: &ruby_prism::Node<'_>) -> String {
    let loc = node.location();
    std::str::from_utf8(loc.as_slice())
        .unwrap_or("")
        .to_string()
}

impl Cop for RaiseArgs {
    fn name(&self) -> &'static str {
        "Style/RaiseArgs"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();
        if name != b"raise" && name != b"fail" {
            return;
        }

        // Only bare raise/fail (no receiver)
        if call.receiver().is_some() {
            return;
        }

        let enforced_style = config.get_str("EnforcedStyle", "explode");
        let allowed_compact_types = config.get_string_array("AllowedCompactTypes").unwrap_or_default();

        if enforced_style != "explode" {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Check if the first argument is a call to `.new`
        if let Some(arg_call) = arg_list[0].as_call_node() {
            if arg_call.name().as_slice() == b"new" {
                if let Some(receiver) = arg_call.receiver() {
                    // Check AllowedCompactTypes: extract the constant name
                    let const_name = extract_const_name(&receiver);
                    if !const_name.is_empty() && allowed_compact_types.iter().any(|t| t == &const_name) {
                        return;
                    }
                    let loc = call.message_loc().unwrap_or_else(|| call.location());
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(source, line, column, "Provide an exception class and message as separate arguments.".to_string()));
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};

    crate::cop_fixture_tests!(RaiseArgs, "cops/style/raise_args");

    #[test]
    fn bare_raise_is_ignored() {
        let source = b"raise\n";
        let diags = run_cop_full(&RaiseArgs, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn allowed_compact_types_exempts_type() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("explode".into())),
                ("AllowedCompactTypes".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("MyWrappedError".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        // MyWrappedError.new should be allowed
        let source = b"raise MyWrappedError.new(obj)\n";
        let diags = run_cop_full_with_config(&RaiseArgs, source, config);
        assert!(diags.is_empty(), "AllowedCompactTypes should exempt MyWrappedError");
    }

    #[test]
    fn allowed_compact_types_does_not_exempt_other() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("explode".into())),
                ("AllowedCompactTypes".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("MyWrappedError".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        // StandardError.new should still be flagged
        let source = b"raise StandardError.new('message')\n";
        let diags = run_cop_full_with_config(&RaiseArgs, source, config);
        assert_eq!(diags.len(), 1, "Non-allowed type should still be flagged");
    }
}
