use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, FALSE_NODE, TRUE_NODE};

pub struct PredicateMatcher;

/// Default style `inflected`: flags `expect(foo.bar?).to be_truthy` →
/// prefer `expect(foo).to be_bar`.
impl Cop for PredicateMatcher {
    fn name(&self) -> &'static str {
        "RSpec/PredicateMatcher"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, FALSE_NODE, TRUE_NODE]
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
        // Config: Strict — when false, also match be(true)/be(false) in addition to be_truthy/be_falsey
        let strict = config.get_bool("Strict", true);
        // Config: EnforcedStyle — "inflected" (default) or "explicit"
        let enforced_style = config.get_str("EnforcedStyle", "inflected");
        // Config: AllowedExplicitMatchers — matchers to allow in explicit style
        let allowed_explicit = config.get_string_array("AllowedExplicitMatchers").unwrap_or_default();

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return;
        }

        if enforced_style == "explicit" {
            // Explicit style: flag `expect(foo).to be_valid` → prefer explicit predicate
            let args = match call.arguments() {
                Some(a) => a,
                None => return,
            };
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.is_empty() {
                return;
            }
            let matcher = &arg_list[0];
            let matcher_call = match matcher.as_call_node() {
                Some(c) => c,
                None => return,
            };
            if matcher_call.receiver().is_some() {
                return;
            }
            let matcher_name = matcher_call.name().as_slice();
            let matcher_str = std::str::from_utf8(matcher_name).unwrap_or("");
            // Check for be_xxx or have_xxx pattern
            if !(matcher_str.starts_with("be_") || matcher_str.starts_with("have_")) {
                return;
            }
            // AllowedExplicitMatchers: skip matchers in the allowlist
            if allowed_explicit.iter().any(|m| m == matcher_str) {
                return;
            }
            let predicate = matcher_to_predicate(matcher_str);
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Prefer using `{predicate}` over `{matcher_str}` matcher."),
            ));
        }

        // Inflected style (default): flag `expect(foo.predicate?).to be_truthy/be_falsey`
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let matcher = &arg_list[0];
        if !is_boolean_matcher(matcher, strict) {
            return;
        }

        // The receiver should be `expect(foo.predicate?)`
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        let expect_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if expect_call.name().as_slice() != b"expect" || expect_call.receiver().is_some() {
            return;
        }

        // Get the argument to expect — should be a predicate call (ends with ?)
        let expect_args = match expect_call.arguments() {
            Some(a) => a,
            None => return,
        };
        let expect_arg_list: Vec<_> = expect_args.arguments().iter().collect();
        if expect_arg_list.is_empty() {
            return;
        }

        let actual = &expect_arg_list[0];
        let predicate_call = match actual.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // The predicate call must have an explicit receiver (e.g., `foo.valid?`).
        // Bare method calls like `enabled?('x')` are NOT predicates on an object
        // and should not be flagged. This matches RuboCop's `(send !nil? ...)` pattern.
        if predicate_call.receiver().is_none() {
            return;
        }

        let pred_name = predicate_call.name().as_slice();
        if !pred_name.ends_with(b"?") {
            return;
        }

        // Skip respond_to? with more than 1 argument (second arg is include_all)
        if pred_name == b"respond_to?" {
            if let Some(args) = predicate_call.arguments() {
                if args.arguments().iter().count() > 1 {
                    return;
                }
            }
        }

        // Build the suggested matcher name
        let pred_str = std::str::from_utf8(pred_name).unwrap_or("");
        let suggested = predicate_to_matcher(pred_str);

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Prefer using `{suggested}` matcher over `{pred_str}`."),
        ));
    }
}

fn predicate_to_matcher(pred: &str) -> String {
    let base = &pred[..pred.len() - 1]; // strip trailing ?
    if base == "exist" || base == "exists" {
        "exist".to_string()
    } else if let Some(stripped) = base.strip_prefix("has_") {
        format!("have_{stripped}")
    } else if base == "include" {
        "include".to_string()
    } else if base == "respond_to" {
        "respond_to".to_string()
    } else if base == "is_a" {
        "be_a".to_string()
    } else if base == "instance_of" {
        "be_an_instance_of".to_string()
    } else {
        format!("be_{base}")
    }
}

/// Convert an inflected matcher back to a predicate method.
/// e.g. "be_valid" -> "valid?", "have_key" -> "has_key?"
fn matcher_to_predicate(matcher: &str) -> String {
    if let Some(rest) = matcher.strip_prefix("be_") {
        format!("{rest}?")
    } else if let Some(rest) = matcher.strip_prefix("have_") {
        format!("has_{rest}?")
    } else {
        format!("{matcher}?")
    }
}

fn is_boolean_matcher(node: &ruby_prism::Node<'_>, strict: bool) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    if call.receiver().is_some() {
        return false;
    }

    let name = call.name().as_slice();

    if matches!(
        name,
        b"be_truthy"
            | b"be_falsey"
            | b"be_falsy"
            | b"a_truthy_value"
            | b"a_falsey_value"
            | b"a_falsy_value"
    ) {
        return true;
    }

    // In non-strict mode, also match be(true)/be(false)/eq(true)/eq(false)/eql(true)/eql(false)/equal(true)/equal(false)
    if !strict && (name == b"be" || name == b"eq" || name == b"eql" || name == b"equal") {
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() == 1 {
                if arg_list[0].as_true_node().is_some() || arg_list[0].as_false_node().is_some() {
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PredicateMatcher, "cops/rspec/predicate_matcher");

    #[test]
    fn explicit_style_flags_be_valid() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("explicit".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"expect(foo).to be_valid\n";
        let diags = crate::testutil::run_cop_full_with_config(&PredicateMatcher, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("valid?"));
    }

    #[test]
    fn allowed_explicit_matchers_skips_listed() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("explicit".into())),
                ("AllowedExplicitMatchers".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("be_valid".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        let source = b"expect(foo).to be_valid\n";
        let diags = crate::testutil::run_cop_full_with_config(&PredicateMatcher, source, config);
        assert!(diags.is_empty(), "AllowedExplicitMatchers should skip listed matchers");
    }

    #[test]
    fn strict_false_flags_be_true() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "Strict".into(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        let source = b"expect(foo.valid?).to be(true)\n";
        let diags = crate::testutil::run_cop_full_with_config(&PredicateMatcher, source, config);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn strict_true_does_not_flag_be_true() {
        let source = b"expect(foo.valid?).to be(true)\n";
        let diags = crate::testutil::run_cop_full(&PredicateMatcher, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn bare_predicate_call_without_receiver_not_flagged() {
        // Bare method calls like `enabled?('x')` should not be flagged
        // because they have no receiver — they are helper methods, not predicates on an object.
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "Strict".into(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        let source = b"expect(enabled?('Layout/DotPosition')).to be(false)\n";
        let diags = crate::testutil::run_cop_full_with_config(&PredicateMatcher, source, config);
        assert!(diags.is_empty(), "Bare predicate calls without a receiver should not be flagged");
    }

    #[test]
    fn respond_to_with_multiple_args_not_flagged() {
        let source = b"expect(foo.respond_to?(:bar, true)).to be_truthy\n";
        let diags = crate::testutil::run_cop_full(&PredicateMatcher, source);
        assert!(diags.is_empty(), "respond_to? with multiple args should not be flagged");
    }

    #[test]
    fn strict_false_flags_eql_true() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "Strict".into(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        let source = b"expect(foo.valid?).to eql(true)\n";
        let diags = crate::testutil::run_cop_full_with_config(&PredicateMatcher, source, config);
        assert_eq!(diags.len(), 1, "eql(true) should be flagged in non-strict mode");
    }
}
