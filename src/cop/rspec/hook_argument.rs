use crate::cop::node_type::{CALL_NODE, SYMBOL_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct HookArgument;

/// Hook methods to check.
const HOOK_METHODS: &[&[u8]] = &[b"before", b"after", b"around"];

/// Scope args that mean "suite" or "context" — not flagged.
const NON_EXAMPLE_SCOPES: &[&[u8]] = &[b"suite", b"context", b"all"];

impl Cop for HookArgument {
    fn name(&self) -> &'static str {
        "RSpec/HookArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, SYMBOL_NODE]
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
        // Config: EnforcedStyle — "implicit" (default), "each", or "example"
        let enforced_style = config.get_str("EnforcedStyle", "implicit");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Check both receiverless `before(...)` and `config.before(...)`
        let is_hook = HOOK_METHODS.contains(&method_name);

        if !is_hook {
            return;
        }

        // Must have a block
        if call.block().is_none() {
            return;
        }

        let args = call.arguments();
        let arg_list: Vec<_> = args
            .map(|a| a.arguments().iter().collect::<Vec<_>>())
            .unwrap_or_default();

        // For non-implicit styles with no arguments, flag the implicit usage
        if enforced_style == "each" || enforced_style == "example" {
            // No args = implicit — should have explicit arg
            let has_scope_arg = if arg_list.is_empty() {
                false
            } else if let Some(sym) = arg_list[0].as_symbol_node() {
                let val = sym.unescaped();
                // Only flag if missing expected scope arg
                val == b"each" || val == b"example" || NON_EXAMPLE_SCOPES.contains(&val)
            } else {
                false
            };

            if !has_scope_arg {
                let expected = enforced_style;
                let hook_name = std::str::from_utf8(method_name).unwrap_or("before");
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `{hook_name}(:{expected})` instead of `{hook_name}`."),
                ));
            }

            // Check for wrong style: e.g., enforced "each" but got :example
            if !arg_list.is_empty() {
                if let Some(sym) = arg_list[0].as_symbol_node() {
                    let val = sym.unescaped();
                    if NON_EXAMPLE_SCOPES.contains(&val) {
                        return; // :suite/:context/:all are fine
                    }
                    let val_str = std::str::from_utf8(val).unwrap_or("");
                    if val_str != enforced_style {
                        let hook_name = std::str::from_utf8(method_name).unwrap_or("before");
                        let loc = call.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Use `{hook_name}(:{enforced_style})` instead of `{hook_name}(:{val_str})`."),
                        ));
                    }
                }
            }

            return;
        }

        // Default: "implicit" style — flag :each and :example arguments
        if arg_list.is_empty() {
            return;
        }

        // Ignore hooks with more than one argument
        if arg_list.len() > 1 {
            return;
        }

        let first_arg = &arg_list[0];

        // Check for symbol argument
        if let Some(sym) = first_arg.as_symbol_node() {
            let val = sym.unescaped();

            // Ignore :suite, :context, :all — those are different scopes
            if NON_EXAMPLE_SCOPES.contains(&val) {
                return;
            }

            // Flag :each and :example — should be implicit
            if val == b"each" || val == b"example" {
                let scope_str = std::str::from_utf8(val).unwrap_or("each");
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Omit the default `:{scope_str}` argument for RSpec hooks.",),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HookArgument, "cops/rspec/hook_argument");

    #[test]
    fn each_style_flags_implicit() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("each".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"before do\n  setup\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&HookArgument, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("before(:each)"));
    }

    #[test]
    fn each_style_accepts_each() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("each".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"before(:each) do\n  setup\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&HookArgument, source, config);
        assert!(diags.is_empty());
    }
}
