use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, INTERPOLATED_STRING_NODE, KEYWORD_HASH_NODE, STRING_NODE};

pub struct ExampleWording;

/// Example methods that take a description string.
/// RuboCop's ExampleWording only matches `it` blocks (and focused/pending variants).
const EXAMPLE_METHODS: &[&[u8]] = &[
    b"it", b"fit", b"xit",
];

impl Cop for ExampleWording {
    fn name(&self) -> &'static str {
        "RSpec/ExampleWording"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTERPOLATED_STRING_NODE, KEYWORD_HASH_NODE, STRING_NODE]
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
        // Config: CustomTransform — hash of word replacements (unused: requires hash config)
        let custom_transform = config.get_string_hash("CustomTransform").unwrap_or_default();
        // Config: IgnoredWords — words to ignore in description checking
        let ignored_words = config.get_string_array("IgnoredWords");
        // Config: DisallowedExamples — example descriptions to disallow entirely
        let disallowed_examples = config.get_string_array("DisallowedExamples");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name().as_slice();
        if !EXAMPLE_METHODS.iter().any(|m| method_name == *m) {
            return;
        }

        // Get the first positional argument (the description string)
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        for arg in args.arguments().iter() {
            if arg.as_keyword_hash_node().is_some() {
                continue;
            }

            // Extract the description text
            let (desc_bytes, desc_loc) = if let Some(s) = arg.as_string_node() {
                (Some(s.unescaped().to_vec()), Some(s.content_loc()))
            } else if let Some(interp) = arg.as_interpolated_string_node() {
                let parts: Vec<_> = interp.parts().iter().collect();
                if let Some(first) = parts.first() {
                    if let Some(s) = first.as_string_node() {
                        (Some(s.unescaped().to_vec()), Some(s.content_loc()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            if let (Some(desc), Some(loc)) = (desc_bytes, desc_loc) {
                let desc_str = std::str::from_utf8(&desc).unwrap_or("");

                // Check DisallowedExamples
                if let Some(ref disallowed) = disallowed_examples {
                    let trimmed = desc_str.trim();
                    for d in disallowed {
                        if trimmed.eq_ignore_ascii_case(d) {
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!("Avoid disallowed example description '{d}'."),
                            ));
                        }
                    }
                }

                // Check IgnoredWords — if description starts with an ignored word, skip should-check
                let skip_should = if let Some(ref words) = ignored_words {
                    let first_word = desc_str.split_whitespace().next().unwrap_or("");
                    words.iter().any(|w| w.eq_ignore_ascii_case(first_word))
                } else {
                    false
                };

                if !skip_should && starts_with_should(&desc) {
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    // CustomTransform: suggest replacement for the word after "should"
                    let msg = if !custom_transform.is_empty() {
                        let prefix_len = should_prefix_len(&desc);
                        let after_should = desc_str.get(prefix_len..).unwrap_or("").trim_start();
                        let next_word = after_should.split_whitespace().next().unwrap_or("");
                        if let Some(replacement) = custom_transform.get(next_word) {
                            let rest = after_should.get(next_word.len()..).unwrap_or("").trim_start();
                            if replacement.is_empty() {
                                format!("Do not use should when describing your tests. Use `{rest}` instead.")
                            } else {
                                format!("Do not use should when describing your tests. Use `{replacement} {rest}` instead.")
                            }
                        } else {
                            "Do not use should when describing your tests.".to_string()
                        }
                    } else {
                        "Do not use should when describing your tests.".to_string()
                    };
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        msg,
                    ));
                }

                // Check for "will"/"won't" prefix
                if starts_with_will(&desc) {
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not use the future tense when describing your tests.".to_string(),
                    ));
                }

                // Check for "it " prefix (repeating "it" inside it blocks)
                if starts_with_it_prefix(&desc) {
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not repeat 'it' when describing your tests.".to_string(),
                    ));
                }
            }
            break;
        }

    }
}

/// Check if a byte slice starts with "should" (case-insensitive).
fn starts_with_should(desc: &[u8]) -> bool {
    if desc.len() < 6 {
        return false;
    }
    let lower: Vec<u8> = desc[..6].iter().map(|b| b.to_ascii_lowercase()).collect();
    if lower != b"should" {
        return false;
    }
    // "should" alone or followed by space/word-boundary or "n't"/"n\xe2\x80\x99t"
    desc.len() == 6 || desc[6] == b' ' || desc[6] == b'\'' || desc[6] == b'n'
}

/// Return the length of the "should" prefix (6 for "should", 9 for "shouldn't", etc.)
fn should_prefix_len(desc: &[u8]) -> usize {
    if desc.len() >= 6 {
        let lower6: Vec<u8> = desc[..6].iter().map(|b| b.to_ascii_lowercase()).collect();
        if lower6 == b"should" {
            // Check for "shouldn't" or "shouldn\xe2\x80\x99t"
            if desc.len() >= 9 && desc[6] == b'n' && (desc[7] == b'\'' || desc[7] == b'\xe2') {
                return 9; // shouldn't
            }
            return 6;
        }
    }
    0
}

/// Check if a byte slice starts with "will"/"won't" (case-insensitive).
fn starts_with_will(desc: &[u8]) -> bool {
    if desc.len() >= 4 {
        let lower: Vec<u8> = desc[..4].iter().map(|b| b.to_ascii_lowercase()).collect();
        if lower == b"will" {
            return desc.len() == 4 || desc[4] == b' ';
        }
    }
    if desc.len() >= 5 {
        let lower: Vec<u8> = desc[..5].iter().map(|b| b.to_ascii_lowercase()).collect();
        if lower == b"won't" || lower == b"won\xe2\x80" {
            return true;
        }
    }
    false
}

/// Check if a byte slice starts with "it " (case-insensitive).
fn starts_with_it_prefix(desc: &[u8]) -> bool {
    if desc.len() < 3 {
        return false;
    }
    let lower: Vec<u8> = desc[..2].iter().map(|b| b.to_ascii_lowercase()).collect();
    lower == b"it" && desc[2] == b' '
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExampleWording, "cops/rspec/example_wording");

    #[test]
    fn disallowed_examples_flags_matching() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "DisallowedExamples".into(),
                serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("works".into()),
                ]),
            )]),
            ..CopConfig::default()
        };
        let source = b"it 'works' do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&ExampleWording, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("disallowed"));
    }

    #[test]
    fn custom_transform_suggests_replacement() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let mut transform = serde_yml::Mapping::new();
        transform.insert(
            serde_yml::Value::String("be".into()),
            serde_yml::Value::String("is".into()),
        );
        let config = CopConfig {
            options: HashMap::from([(
                "CustomTransform".into(),
                serde_yml::Value::Mapping(transform),
            )]),
            ..CopConfig::default()
        };
        let source = b"it 'should be valid' do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&ExampleWording, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("is valid"), "CustomTransform should suggest 'is valid' replacement, got: {}", diags[0].message);
    }

    #[test]
    fn ignored_words_skips_should_check() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "IgnoredWords".into(),
                serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("should".into()),
                ]),
            )]),
            ..CopConfig::default()
        };
        let source = b"it 'should do something' do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&ExampleWording, source, config);
        assert!(diags.is_empty(), "IgnoredWords should skip 'should' check");
    }
}
