use crate::cop::shared::node_type::{CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE, SYMBOL_NODE};
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/VerifiedDoubles - flags `double(...)` and `spy(...)` calls.
///
/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=11, FN=0.
///
/// FP=11: calls like `double({ body: { id: '1234' } })` were treated as
/// named doubles because the first argument is a `HashNode` (explicit braces),
/// not a `KeywordHashNode`. RuboCop treats this as nameless methods/stubs data,
/// so with default `IgnoreNameless: true` it should be ignored. Fixed by
/// treating both `KeywordHashNode` and `HashNode` first args as nameless.
///
/// FN=0: no missing detections were reported for this cop in corpus data.
///
/// Historical fix (already implemented here): this cop now flags `double/spy`
/// regardless of name argument type, with filtering controlled by
/// `IgnoreNameless` and `IgnoreSymbolicNames`.
pub struct VerifiedDoubles;
impl Cop for VerifiedDoubles {
    fn name(&self) -> &'static str {
        "RSpec/VerifiedDoubles"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE, SYMBOL_NODE]
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
        // Config: IgnoreNameless — ignore doubles without a name argument
        let ignore_nameless = config.get_bool("IgnoreNameless", true);
        // Config: IgnoreSymbolicNames — ignore doubles with symbolic names
        let ignore_symbolic = config.get_bool("IgnoreSymbolicNames", false);
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if method_name != b"double" && method_name != b"spy" {
            return;
        }

        // Must be receiverless
        if call.receiver().is_some() {
            return;
        }

        // Check arguments for name
        let (has_name_arg, is_symbolic) = if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.is_empty()
                || arg_list[0].as_keyword_hash_node().is_some()
                || arg_list[0].as_hash_node().is_some()
            {
                (false, false)
            } else {
                let sym = arg_list[0].as_symbol_node().is_some();
                (true, sym)
            }
        } else {
            (false, false)
        };

        // IgnoreNameless: skip doubles without a name argument
        if ignore_nameless && !has_name_arg {
            return;
        }

        // IgnoreSymbolicNames: skip doubles with symbolic names
        if ignore_symbolic && is_symbolic {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Prefer using verifying doubles over normal doubles.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(VerifiedDoubles, "cops/rspec/verified_doubles");

    #[test]
    fn ignore_nameless_false_flags_nameless_doubles() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("IgnoreNameless".into(), serde_yml::Value::Bool(false))]),
            ..CopConfig::default()
        };
        let source = b"double\n";
        let diags = crate::testutil::run_cop_full_with_config(&VerifiedDoubles, source, config);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn ignore_symbolic_names_skips_symbol_doubles() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("IgnoreSymbolicNames".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        let source = b"double(:foo)\n";
        let diags = crate::testutil::run_cop_full_with_config(&VerifiedDoubles, source, config);
        assert!(
            diags.is_empty(),
            "IgnoreSymbolicNames should skip symbol names"
        );
    }

    #[test]
    fn ignore_symbolic_names_false_flags_symbol_doubles() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("IgnoreSymbolicNames".into(), serde_yml::Value::Bool(false))]),
            ..CopConfig::default()
        };
        let source = b"double(:foo)\n";
        let diags = crate::testutil::run_cop_full_with_config(&VerifiedDoubles, source, config);
        assert_eq!(diags.len(), 1);
    }
}
