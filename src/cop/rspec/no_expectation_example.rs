use ruby_prism::Visit;

use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/NoExpectationExample - flags examples without expectations.
///
/// ## Corpus investigation (2026-03-29)
///
/// Corpus oracle reported FP=0, FN=4.
///
/// FN=4 root cause: keyword metadata matching was broader than RuboCop's
/// `SkipOrPending` mixin. We exempted any non-`false` `skip:`/`pending:` value,
/// which missed examples using dynamic expressions such as `skip: ENV['CI']`,
/// `skip: ActiveRecord.version.version < '5'`, and `:skip =>
/// options.include?(:allow_other_matchers)`.
///
/// Fix: only exempt symbol metadata and keyword metadata whose values match
/// RuboCop's pattern shape: literal `true`, `str`, or `dstr`.
///
/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=66, FN=0.
///
/// FP=66: receiver calls such as `group.it { }`, `group.specify { }`, and
/// `subject.example { }` were incorrectly treated as real RSpec example
/// declarations. RuboCop only checks receiverless example methods. Fixed by
/// requiring `receiver().is_none()` before evaluating example metadata/body.
///
/// FN=0: no missing detections were reported for this cop in corpus data.
///
/// Historical fixes (already implemented here):
/// - Exclude x-prefixed examples and examples with `:skip`/`:pending` metadata.
/// - Match `AllowedPatterns` against in-body method calls (e.g. `^assert_`).
/// - Include `focus` as a focused example method.
pub struct NoExpectationExample;

/// Returns true for regular and focused examples only.
/// Excludes x-prefixed (skipped) examples and pending/skip.
fn is_regular_or_focused_example(name: &[u8]) -> bool {
    matches!(
        name,
        b"it"
            | b"specify"
            | b"example"
            | b"scenario"
            | b"its"
            | b"fit"
            | b"fspecify"
            | b"fexample"
            | b"fscenario"
            | b"focus"
    )
}

/// Check if a call's arguments contain :skip/:pending symbol metadata, or keyword
/// metadata whose value matches RuboCop's SkipOrPending mixin (`true|str|dstr`).
fn has_skip_or_pending_metadata(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            // Symbol metadata: it 'test', :skip do
            if let Some(sym) = arg.as_symbol_node() {
                let val = sym.unescaped();
                if val == b"skip" || val == b"pending" {
                    return true;
                }
            }
            // Keyword hash metadata: it 'test', skip: true do
            if let Some(kh) = arg.as_keyword_hash_node() {
                for elem in kh.elements().iter() {
                    if let Some(assoc) = elem.as_assoc_node() {
                        if let Some(key_sym) = assoc.key().as_symbol_node() {
                            let key = key_sym.unescaped();
                            if (key == b"skip" || key == b"pending")
                                && is_skip_or_pending_metadata_value(&assoc.value())
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn is_skip_or_pending_metadata_value(value: &ruby_prism::Node<'_>) -> bool {
    value.as_true_node().is_some()
        || value.as_string_node().is_some()
        || value.as_interpolated_string_node().is_some()
}

impl Cop for NoExpectationExample {
    fn name(&self) -> &'static str {
        "RSpec/NoExpectationExample"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Only receiverless `it/specify/example/...` are real RSpec example declarations.
        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name().as_slice();
        if !is_regular_or_focused_example(method_name) {
            return;
        }

        // Skip examples with :skip or :pending metadata
        if has_skip_or_pending_metadata(&call) {
            return;
        }

        // Config: AllowedPatterns — description patterns to exempt from this cop
        let allowed_patterns = config.get_string_array("AllowedPatterns");

        // Compile regexes once per example (not per-method-call inside the body).
        // Most configs have 0-2 patterns, so this is typically very cheap.
        let compiled_patterns: Vec<regex::Regex> = match &allowed_patterns {
            Some(patterns) => patterns
                .iter()
                .filter_map(|p| regex::Regex::new(p).ok())
                .collect(),
            None => Vec::new(),
        };

        // Check AllowedPatterns against the example description
        if !compiled_patterns.is_empty() {
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    if arg.as_keyword_hash_node().is_some() {
                        continue;
                    }
                    if let Some(s) = arg.as_string_node() {
                        if let Ok(desc) = std::str::from_utf8(s.unescaped()) {
                            if compiled_patterns.iter().any(|re| re.is_match(desc)) {
                                return;
                            }
                        }
                    }
                    break;
                }
            }
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        // Check if the block body contains any expectation
        let mut finder = ExpectationFinder {
            found: false,
            method_patterns: &compiled_patterns,
        };
        if let Some(body) = block.body() {
            finder.visit(&body);
        }

        if !finder.found {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "No expectation found in this example.".to_string(),
            ));
        }
    }
}

struct ExpectationFinder<'a> {
    found: bool,
    method_patterns: &'a [regex::Regex],
}

impl<'pr> Visit<'pr> for ExpectationFinder<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.found {
            return;
        }
        let name = node.name().as_slice();
        // Check for expectation methods (receiverless only)
        if node.receiver().is_none()
            && (name == b"expect"
                || name == b"expect_any_instance_of"
                || name == b"is_expected"
                || name == b"are_expected"
                || name == b"should"
                || name == b"should_not"
                || name == b"should_receive"
                || name == b"should_not_receive"
                || name == b"pending"
                || name == b"skip")
        {
            self.found = true;
            return;
        }
        // Check AllowedPatterns against method names (e.g. ^expect_, ^assert_)
        // This matches RuboCop behavior where AllowedPatterns apply to
        // method call names within the example body.
        if node.receiver().is_none() && !self.method_patterns.is_empty() {
            if let Ok(name_str) = std::str::from_utf8(name) {
                for pat in self.method_patterns {
                    if pat.is_match(name_str) {
                        self.found = true;
                        return;
                    }
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NoExpectationExample, "cops/rspec/no_expectation_example");

    #[test]
    fn allowed_patterns_skips_matching_description() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedPatterns".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("^triggers".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"it 'triggers a callback' do\n  run_job\nend\n";
        let diags =
            crate::testutil::run_cop_full_with_config(&NoExpectationExample, source, config);
        assert!(
            diags.is_empty(),
            "AllowedPatterns should skip matching descriptions"
        );
    }
}
