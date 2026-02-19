use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct MethodCallWithArgsParentheses;

const IGNORED_METHODS: &[&[u8]] = &[
    b"require",
    b"require_relative",
    b"include",
    b"extend",
    b"prepend",
    b"puts",
    b"print",
    b"p",
    b"pp",
    b"raise",
    b"fail",
    b"attr_reader",
    b"attr_writer",
    b"attr_accessor",
    b"private",
    b"protected",
    b"public",
    b"module_function",
    b"gem",
    b"source",
    b"yield",
    b"return",
    b"super",
];

fn is_operator(name: &[u8]) -> bool {
    matches!(
        name,
        b"+" | b"-" | b"*" | b"/" | b"%" | b"**" | b"==" | b"!=" | b"<" | b">" | b"<="
            | b">=" | b"<=>" | b"<<" | b">>" | b"&" | b"|" | b"^" | b"~" | b"!" | b"[]"
            | b"[]=" | b"=~" | b"!~" | b"+@" | b"-@"
    )
}

/// Check if name is a setter method (ends with `=`)
fn is_setter(name: &[u8]) -> bool {
    name.last() == Some(&b'=') && name.len() > 1 && name != b"==" && name != b"!="
}

/// Check if a method name matches any pattern in the list (substring match).
fn matches_any_pattern(name_str: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        // Simple: if pattern starts with ^ it's a prefix match, otherwise substring
        if pattern.starts_with('^') {
            let prefix = &pattern[1..];
            if name_str.starts_with(prefix) {
                return true;
            }
        } else if name_str.contains(pattern.as_str()) {
            return true;
        }
    }
    false
}

/// Check if the method name starts with an uppercase letter (CamelCase).
fn is_camel_case_method(name: &[u8]) -> bool {
    name.first().is_some_and(|b| b.is_ascii_uppercase())
}

/// Check if a call node is inside string interpolation by examining the source
/// bytes before the call location for `#{` context.
fn is_inside_string_interpolation(source: &SourceFile, call_start: usize) -> bool {
    let bytes = source.as_bytes();
    // Walk backwards from the call start looking for `#{` before a closing `}`
    // This is an approximation: look for `#{` without an intervening `}`.
    let mut i = call_start;
    while i > 0 {
        i -= 1;
        if i > 0 && bytes[i - 1] == b'#' && bytes[i] == b'{' {
            return true;
        }
        if bytes[i] == b'}' {
            // Found a closing brace before `#{`, not in interpolation
            return false;
        }
        // Stop at newline — interpolation doesn't span lines in practice
        if bytes[i] == b'\n' {
            return false;
        }
    }
    false
}

impl Cop for MethodCallWithArgsParentheses {
    fn name(&self) -> &'static str {
        "Style/MethodCallWithArgsParentheses"
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
    ) -> Vec<Diagnostic> {
        let ignore_macros = config.get_bool("IgnoreMacros", true);
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");
        let included_macros = config.get_string_array("IncludedMacros");
        let included_macro_patterns = config.get_string_array("IncludedMacroPatterns");
        let allow_multiline = config.get_bool("AllowParenthesesInMultilineCall", false);
        let allow_chaining = config.get_bool("AllowParenthesesInChaining", false);
        let allow_camel = config.get_bool("AllowParenthesesInCamelCaseMethod", false);
        let allow_interp = config.get_bool("AllowParenthesesInStringInterpolation", false);
        let enforced_style = config.get_str("EnforcedStyle", "require_parentheses");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let name = call.name().as_slice();

        // Skip operators and setters in both styles
        if is_operator(name) || is_setter(name) {
            return Vec::new();
        }

        // Skip methods in the built-in ignore list
        if IGNORED_METHODS.contains(&name) {
            return Vec::new();
        }

        // Must have arguments
        if call.arguments().is_none() {
            return Vec::new();
        }

        let name_str = std::str::from_utf8(name).unwrap_or("");
        let has_parens = call.opening_loc().is_some();
        let is_receiverless = call.receiver().is_none();

        match enforced_style {
            "omit_parentheses" => {
                // Flag calls WITH parens; various exceptions allow parens
                if !has_parens {
                    return Vec::new();
                }

                // AllowParenthesesInCamelCaseMethod: allow parens for CamelCase methods
                if allow_camel && is_camel_case_method(name) {
                    return Vec::new();
                }

                // AllowParenthesesInMultilineCall: allow parens for multiline calls
                if allow_multiline {
                    let call_loc = call.location();
                    let (start_line, _) = source.offset_to_line_col(call_loc.start_offset());
                    let (end_line, _) = source.offset_to_line_col(call_loc.end_offset());
                    if start_line != end_line {
                        return Vec::new();
                    }
                }

                // AllowParenthesesInChaining: allow parens when receiver is also a call
                if allow_chaining {
                    if let Some(receiver) = call.receiver() {
                        if receiver.as_call_node().is_some() {
                            return Vec::new();
                        }
                    }
                }

                // AllowParenthesesInStringInterpolation: allow parens inside #{...}
                if allow_interp {
                    let call_start = call.location().start_offset();
                    if is_inside_string_interpolation(source, call_start) {
                        return Vec::new();
                    }
                }

                let loc = call.message_loc().unwrap_or_else(|| call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Omit parentheses for method calls with arguments.".to_string(),
                )]
            }
            _ => {
                // "require_parentheses" (default)
                if has_parens {
                    return Vec::new();
                }

                // AllowedMethods: exempt specific method names
                if let Some(ref methods) = allowed_methods {
                    if methods.iter().any(|m| m == name_str) {
                        return Vec::new();
                    }
                }

                // AllowedPatterns: exempt methods matching patterns
                if let Some(ref patterns) = allowed_patterns {
                    if matches_any_pattern(name_str, patterns) {
                        return Vec::new();
                    }
                }

                // IgnoreMacros: skip receiverless calls (macro-style) unless
                // they are in IncludedMacros or IncludedMacroPatterns
                if is_receiverless && ignore_macros {
                    let in_included = included_macros
                        .as_ref()
                        .is_some_and(|macros| macros.iter().any(|m| m == name_str));
                    let in_included_patterns = included_macro_patterns
                        .as_ref()
                        .is_some_and(|patterns| matches_any_pattern(name_str, patterns));

                    if !in_included && !in_included_patterns {
                        return Vec::new();
                    }
                }

                let loc = call.message_loc().unwrap_or_else(|| call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use parentheses for method calls with arguments.".to_string(),
                )]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};

    crate::cop_fixture_tests!(
        MethodCallWithArgsParentheses,
        "cops/style/method_call_with_args_parentheses"
    );

    #[test]
    fn operators_are_ignored() {
        let source = b"x = 1 + 2\n";
        let diags = run_cop_full(&MethodCallWithArgsParentheses, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn method_without_args_is_ok() {
        let source = b"foo.bar\n";
        let diags = run_cop_full(&MethodCallWithArgsParentheses, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn omit_parentheses_flags_parens() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo.bar(1)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert_eq!(diags.len(), 1, "Should flag parens with omit_parentheses");
        assert!(diags[0].message.contains("Omit parentheses"));
    }

    #[test]
    fn omit_parentheses_allows_no_parens() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo.bar 1\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag calls without parens in omit_parentheses"
        );
    }

    #[test]
    fn allowed_methods_exempts() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedMethods".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("custom_log".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo.custom_log 'msg'\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag method in AllowedMethods list"
        );
    }

    #[test]
    fn allowed_patterns_exempts() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedPatterns".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("^assert".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo.assert_equal 'x', y\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag method matching AllowedPatterns"
        );
    }

    #[test]
    fn ignore_macros_skips_receiverless() {
        // Default IgnoreMacros is true, receiverless calls should be skipped
        let source = b"custom_macro :arg\n";
        let diags = run_cop_full(&MethodCallWithArgsParentheses, source);
        assert!(
            diags.is_empty(),
            "Should skip receiverless macro with IgnoreMacros:true"
        );
    }

    #[test]
    fn ignore_macros_false_flags_receiverless() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "IgnoreMacros".into(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        let source = b"custom_macro :arg\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert_eq!(
            diags.len(),
            1,
            "Should flag receiverless macro with IgnoreMacros:false"
        );
    }

    #[test]
    fn included_macros_forces_check() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "IncludedMacros".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String(
                    "custom_macro".into(),
                )]),
            )]),
            ..CopConfig::default()
        };
        // Even with IgnoreMacros:true (default), IncludedMacros forces checking
        let source = b"custom_macro :arg\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert_eq!(
            diags.len(),
            1,
            "Should flag macro in IncludedMacros despite IgnoreMacros:true"
        );
    }

    #[test]
    fn included_macro_patterns_forces_check() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "IncludedMacroPatterns".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("^validate".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"validates_presence :name\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert_eq!(
            diags.len(),
            1,
            "Should flag macro matching IncludedMacroPatterns"
        );
    }

    #[test]
    fn omit_allow_multiline_call() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                (
                    "EnforcedStyle".into(),
                    serde_yml::Value::String("omit_parentheses".into()),
                ),
                (
                    "AllowParenthesesInMultilineCall".into(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };
        let source = b"foo.bar(\n  1\n)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should allow parens in multiline call with AllowParenthesesInMultilineCall"
        );
    }

    #[test]
    fn omit_allow_chaining() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                (
                    "EnforcedStyle".into(),
                    serde_yml::Value::String("omit_parentheses".into()),
                ),
                (
                    "AllowParenthesesInChaining".into(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };
        let source = b"foo.bar(1).baz(2)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        // baz has receiver foo.bar(1) which is a call node, so baz should be allowed
        // foo.bar(1)'s receiver is foo (not a call), so it may be flagged
        // We check that at least baz(2) is allowed
        let baz_flagged = diags.iter().any(|d| d.location.column == 10);
        assert!(
            !baz_flagged,
            "Should allow parens on chained call with AllowParenthesesInChaining"
        );
    }

    #[test]
    fn omit_allow_camel_case() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                (
                    "EnforcedStyle".into(),
                    serde_yml::Value::String("omit_parentheses".into()),
                ),
                (
                    "AllowParenthesesInCamelCaseMethod".into(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };
        let source = b"Array(1)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should allow parens on CamelCase method with AllowParenthesesInCamelCaseMethod"
        );
    }

    #[test]
    fn omit_allow_string_interpolation() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                (
                    "EnforcedStyle".into(),
                    serde_yml::Value::String("omit_parentheses".into()),
                ),
                (
                    "AllowParenthesesInStringInterpolation".into(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };
        let source = b"x = \"#{foo.bar(1)}\"\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should allow parens inside string interpolation with AllowParenthesesInStringInterpolation"
        );
    }
}
