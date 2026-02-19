use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE, SYMBOL_NODE};

pub struct VerifiedDoubles;

/// Flags `double("Name")` and `spy("Name")` — prefer verified doubles
/// like `instance_double`, `class_double`, etc.
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
    ) -> Vec<Diagnostic> {
        // Config: IgnoreNameless — ignore doubles without a name argument
        let ignore_nameless = config.get_bool("IgnoreNameless", true);
        // Config: IgnoreSymbolicNames — ignore doubles with symbolic names
        let ignore_symbolic = config.get_bool("IgnoreSymbolicNames", false);
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        if method_name != b"double" && method_name != b"spy" {
            return Vec::new();
        }

        // Must be receiverless
        if call.receiver().is_some() {
            return Vec::new();
        }

        // Check arguments for name
        let (has_name_arg, is_symbolic, is_string) = if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.is_empty() || arg_list[0].as_keyword_hash_node().is_some() {
                (false, false, false)
            } else {
                let sym = arg_list[0].as_symbol_node().is_some();
                let str = arg_list[0].as_string_node().is_some();
                (true, sym, str)
            }
        } else {
            (false, false, false)
        };

        // IgnoreNameless: skip doubles without a name argument
        if ignore_nameless && !has_name_arg {
            return Vec::new();
        }

        // IgnoreSymbolicNames: skip doubles with symbolic names
        if ignore_symbolic && is_symbolic {
            return Vec::new();
        }

        // Name must be a string or symbol to flag
        if has_name_arg && !is_string && !is_symbolic {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Prefer using verifying doubles over normal doubles.".to_string(),
        )]
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
            options: HashMap::from([(
                "IgnoreNameless".into(),
                serde_yml::Value::Bool(false),
            )]),
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
            options: HashMap::from([(
                "IgnoreSymbolicNames".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };
        let source = b"double(:foo)\n";
        let diags = crate::testutil::run_cop_full_with_config(&VerifiedDoubles, source, config);
        assert!(diags.is_empty(), "IgnoreSymbolicNames should skip symbol names");
    }

    #[test]
    fn ignore_symbolic_names_false_flags_symbol_doubles() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "IgnoreSymbolicNames".into(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        let source = b"double(:foo)\n";
        let diags = crate::testutil::run_cop_full_with_config(&VerifiedDoubles, source, config);
        assert_eq!(diags.len(), 1);
    }
}
