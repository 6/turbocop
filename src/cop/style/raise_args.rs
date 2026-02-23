use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RaiseArgs;

/// Extract the constant name from a node by reading its source text.
fn extract_const_name(node: &ruby_prism::Node<'_>) -> String {
    let loc = node.location();
    std::str::from_utf8(loc.as_slice())
        .unwrap_or("")
        .to_string()
}

/// Check if the argument to `.new` is an acceptable type that can't be
/// easily converted to exploded form (hash, splat, forwarding args).
fn is_acceptable_exploded_arg(node: &ruby_prism::Node<'_>) -> bool {
    node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
        || node.as_splat_node().is_some()
        || node.as_forwarding_arguments_node().is_some()
        || node.as_assoc_splat_node().is_some()
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

        let method_name = std::str::from_utf8(name).unwrap_or("raise");
        let enforced_style = config.get_str("EnforcedStyle", "exploded");

        match enforced_style {
            "exploded" => self.check_exploded(source, &call, config, diagnostics, method_name),
            "compact" => self.check_compact(source, &call, diagnostics, method_name),
            _ => {}
        }
    }
}

impl RaiseArgs {
    fn check_exploded(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        method_name: &str,
    ) {
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        // Exploded style only flags single-arg raise where arg is Error.new(...)
        if arg_list.len() != 1 {
            return;
        }

        let arg_call = match arg_list[0].as_call_node() {
            Some(c) => c,
            None => return,
        };

        if arg_call.name().as_slice() != b"new" {
            return;
        }

        let receiver = match arg_call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Check if .new has multiple arguments — allow raise Ex.new(arg1, arg2)
        if let Some(new_args) = arg_call.arguments() {
            let new_arg_list: Vec<_> = new_args.arguments().iter().collect();
            if new_arg_list.len() > 1 {
                return;
            }
            // Single arg: check if it's a hash, splat, or forwarding arg
            if new_arg_list.len() == 1 && is_acceptable_exploded_arg(&new_arg_list[0]) {
                return;
            }
        }

        // Check AllowedCompactTypes
        let allowed_compact_types = config
            .get_string_array("AllowedCompactTypes")
            .unwrap_or_default();
        let const_name = extract_const_name(&receiver);
        if !const_name.is_empty() && allowed_compact_types.iter().any(|t| t == &const_name) {
            return;
        }

        let loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Provide an exception class and message as arguments to `{method_name}`."),
        ));
    }

    fn check_compact(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
        method_name: &str,
    ) {
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        // Compact style flags raise with 2+ args (raise Error, msg)
        if arg_list.len() < 2 {
            return;
        }

        // If the first arg is a send to .new with a hash arg, don't flag
        // (matches RuboCop: `exception.first_argument&.hash_type?`)
        if let Some(first_call) = arg_list[0].as_call_node() {
            if first_call.name().as_slice() == b"new" {
                if let Some(new_args) = first_call.arguments() {
                    let new_arg_list: Vec<_> = new_args.arguments().iter().collect();
                    if !new_arg_list.is_empty()
                        && (new_arg_list[0].as_hash_node().is_some()
                            || new_arg_list[0].as_keyword_hash_node().is_some())
                    {
                        return;
                    }
                }
            }
        }

        let loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Provide an exception object as an argument to `{method_name}`."),
        ));
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
                (
                    "EnforcedStyle".into(),
                    serde_yml::Value::String("exploded".into()),
                ),
                (
                    "AllowedCompactTypes".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String(
                        "MyWrappedError".into(),
                    )]),
                ),
            ]),
            ..CopConfig::default()
        };
        // MyWrappedError.new should be allowed
        let source = b"raise MyWrappedError.new(obj)\n";
        let diags = run_cop_full_with_config(&RaiseArgs, source, config);
        assert!(
            diags.is_empty(),
            "AllowedCompactTypes should exempt MyWrappedError"
        );
    }

    #[test]
    fn allowed_compact_types_does_not_exempt_other() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                (
                    "EnforcedStyle".into(),
                    serde_yml::Value::String("exploded".into()),
                ),
                (
                    "AllowedCompactTypes".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String(
                        "MyWrappedError".into(),
                    )]),
                ),
            ]),
            ..CopConfig::default()
        };
        // StandardError.new should still be flagged
        let source = b"raise StandardError.new('message')\n";
        let diags = run_cop_full_with_config(&RaiseArgs, source, config);
        assert_eq!(diags.len(), 1, "Non-allowed type should still be flagged");
    }
}
