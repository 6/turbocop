use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SignalException;

impl Cop for SignalException {
    fn name(&self) -> &'static str {
        "Style/SignalException"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Only bare raise/fail (no receiver)
        if call.receiver().is_some() {
            return Vec::new();
        }

        let name = call.name().as_slice();
        if name != b"raise" && name != b"fail" {
            return Vec::new();
        }

        let enforced_style = config.get_str("EnforcedStyle", "only_raise");

        let loc = call.message_loc().unwrap_or_else(|| call.location());

        match enforced_style {
            "only_raise" => {
                if name == b"fail" {
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(source, line, column, "Use `raise` instead of `fail` to rethrow exceptions.".to_string())];
                }
            }
            "only_fail" => {
                if name == b"raise" {
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(source, line, column, "Use `fail` instead of `raise` to rethrow exceptions.".to_string())];
                }
            }
            _ => {}
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full_with_config;

    crate::cop_fixture_tests!(SignalException, "cops/style/signal_exception");

    #[test]
    fn config_only_fail() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("only_fail".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"raise RuntimeError, \"msg\"\n";
        let diags = run_cop_full_with_config(&SignalException, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Use `fail`"));
    }
}
