use ruby_prism::Visit;

use crate::cop::util::{is_rspec_example, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE};

pub struct NoExpectationExample;

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
        &[BLOCK_NODE, CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Config: AllowedPatterns — description patterns to exempt from this cop
        let allowed_patterns = config.get_string_array("AllowedPatterns");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        if !is_rspec_example(method_name) {
            return Vec::new();
        }

        // Skip `pending` and `skip` examples -- they intentionally have no expectations
        if method_name == b"pending" || method_name == b"skip" {
            return Vec::new();
        }

        // Check AllowedPatterns against the example description
        if let Some(ref patterns) = allowed_patterns {
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    if arg.as_keyword_hash_node().is_some() {
                        continue;
                    }
                    let desc_text = if let Some(s) = arg.as_string_node() {
                        Some(std::str::from_utf8(s.unescaped()).unwrap_or("").to_string())
                    } else {
                        None
                    };
                    if let Some(ref desc) = desc_text {
                        for pat in patterns {
                            if let Ok(re) = regex::Regex::new(pat) {
                                if re.is_match(desc) {
                                    return Vec::new();
                                }
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
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        // Build compiled allowed patterns for method-name matching
        // Default patterns are ^expect_ and ^assert_ (from rubocop-rspec default config)
        let method_patterns: Vec<regex::Regex> = if let Some(ref patterns) = allowed_patterns {
            patterns
                .iter()
                .filter_map(|p| regex::Regex::new(p).ok())
                .collect()
        } else {
            Vec::new()
        };

        // Check if the block body contains any expectation
        let mut finder = ExpectationFinder {
            found: false,
            method_patterns: &method_patterns,
        };
        if let Some(body) = block.body() {
            finder.visit(&body);
        }

        if !finder.found {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            vec![self.diagnostic(
                source,
                line,
                column,
                "No expectation found in this example.".to_string(),
            )]
        } else {
            Vec::new()
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
        // Check for: expect, is_expected, assert*, should, should_not, pending, skip
        if node.receiver().is_none()
            && (name == b"expect"
                || name == b"expect_any_instance_of"
                || name == b"is_expected"
                || name.starts_with(b"assert")
                || name == b"pending"
                || name == b"skip")
        {
            self.found = true;
            return;
        }
        // Check for `should` and `should_not` (with any receiver)
        if name == b"should" || name == b"should_not" {
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
                serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("^triggers".into()),
                ]),
            )]),
            ..CopConfig::default()
        };
        let source = b"it 'triggers a callback' do\n  run_job\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&NoExpectationExample, source, config);
        assert!(diags.is_empty(), "AllowedPatterns should skip matching descriptions");
    }
}
