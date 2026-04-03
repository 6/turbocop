use crate::cop::shared::node_type::{
    CALL_NODE, INTERPOLATED_STRING_NODE, KEYWORD_HASH_NODE, STRING_NODE,
};
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Previous investigation: 63 FPs were caused by flagging pending examples (no block),
/// e.g. `it "should do something"` without a `do...end`. Fixed by checking
/// `call.block().is_some()`.
///
/// Round 2: FP=28, FN=4.
///
/// FP root causes (all fixed):
/// 1. Cop checked `fit`/`xit` in addition to `it`, but RuboCop's ExampleWording
///    only matches `(block (send _ :it ...))` — only the `it` method.
/// 2. Cop emitted multiple offenses per description (e.g., both "should" and
///    DisallowedExamples), but RuboCop uses if/elsif so only one offense fires.
/// 3. DisallowedExamples comparison didn't match RuboCop's `preprocess` logic
///    (strip + squeeze spaces + downcase).
/// 4. `starts_with_should` accepted `desc[6] == b'n'` for "shouldn't" but
///    also matched "shouldnt" (no apostrophe). RuboCop uses `\b` (word boundary).
///    Fixed with proper `is_word_char` check. Same for `starts_with_will`.
/// 5. `it 'desc', &(proc do...end)` — Prism sees the `&(proc do...end)` as a
///    `BlockArgumentNode` on the `it` call, not a `BlockNode`. RuboCop's `on_block`
///    only fires for `BlockNode`. Fixed by checking `block.as_block_node().is_some()`.
/// 6. Cop rejected calls with a receiver (`call.receiver().is_some()`), but RuboCop's
///    pattern uses `_` for receiver (matches anything). Fixed by removing the check.
///
/// FN root causes (all fixed):
/// - 3 FNs: "should," (comma after "should") not detected. Our check required
///   specific chars after "should" instead of proper word boundary. Fixed with
///   `is_word_char` check.
/// - 1 FN: `group.it('works') { }` — receiver present. Fixed by removing
///   receiver filter.
///
/// ## Corpus investigation (2026-03-25)
///
/// 4 FPs in jpmobile repo: double-quoted strings with `\xf8\x9f` / `\xf9\x8b` hex
/// escapes produce non-UTF-8 unescaped bytes. RuboCop's Parser gem cannot represent
/// these as valid string content, so it never fires on them. Fixed by skipping
/// descriptions where `unescaped()` is not valid UTF-8.
pub struct ExampleWording;

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
        &[
            CALL_NODE,
            INTERPOLATED_STRING_NODE,
            KEYWORD_HASH_NODE,
            STRING_NODE,
        ]
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
        let custom_transform = config
            .get_string_hash("CustomTransform")
            .unwrap_or_default();
        // Config: IgnoredWords — words to ignore in description checking
        let ignored_words = config.get_string_array("IgnoredWords");
        // Config: DisallowedExamples — example descriptions to disallow entirely
        let disallowed_examples = config.get_string_array("DisallowedExamples");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // RuboCop's ExampleWording pattern uses `_` for receiver (matches anything
        // including nil), so we do NOT filter out calls with a receiver.

        // RuboCop's ExampleWording only matches `it` blocks — not `fit`/`xit`/`specify` etc.
        let method_name = call.name().as_slice();
        if method_name != b"it" {
            return;
        }

        // RuboCop's on_block callback only fires for actual block nodes (do...end / { }).
        // Skip if no block at all (pending examples) or if the "block" is actually a
        // block argument (&expr), e.g. `it 'desc', &(proc do...end)`.
        match call.block() {
            Some(block) if block.as_block_node().is_some() => {}
            _ => return,
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
                // RuboCop's Parser gem cannot represent non-UTF-8 string content,
                // so it never fires on descriptions with invalid UTF-8 bytes
                // (e.g., \xf8\x9f hex escapes in double-quoted strings).
                let desc_str = match std::str::from_utf8(&desc) {
                    Ok(s) => s,
                    Err(_) => break,
                };

                // Check IgnoredWords — if description starts with an ignored word, skip should-check
                let skip_should = if let Some(ref words) = ignored_words {
                    let first_word = desc_str.split_whitespace().next().unwrap_or("");
                    words.iter().any(|w| w.eq_ignore_ascii_case(first_word))
                } else {
                    false
                };

                // RuboCop uses if/elsif — only ONE offense fires per description.
                // Priority: should > will > it prefix > DisallowedExamples
                if !skip_should && starts_with_should(&desc) {
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    // CustomTransform: suggest replacement for the word after "should"
                    let msg = if !custom_transform.is_empty() {
                        let prefix_len = should_prefix_len(&desc);
                        let after_should = desc_str.get(prefix_len..).unwrap_or("").trim_start();
                        let next_word = after_should.split_whitespace().next().unwrap_or("");
                        if let Some(replacement) = custom_transform.get(next_word) {
                            let rest = after_should
                                .get(next_word.len()..)
                                .unwrap_or("")
                                .trim_start();
                            if replacement.is_empty() {
                                format!(
                                    "Do not use should when describing your tests. Use `{rest}` instead."
                                )
                            } else {
                                format!(
                                    "Do not use should when describing your tests. Use `{replacement} {rest}` instead."
                                )
                            }
                        } else {
                            "Do not use should when describing your tests.".to_string()
                        }
                    } else {
                        "Do not use should when describing your tests.".to_string()
                    };
                    diagnostics.push(self.diagnostic(source, line, column, msg));
                } else if starts_with_will(&desc) {
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not use the future tense when describing your tests.".to_string(),
                    ));
                } else if starts_with_it_prefix(&desc) {
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not repeat 'it' when describing your tests.".to_string(),
                    ));
                } else if let Some(ref disallowed) = disallowed_examples {
                    // DisallowedExamples: RuboCop preprocesses with strip + squeeze(' ') + downcase
                    let preprocessed = preprocess_description(desc_str);
                    for d in disallowed {
                        let preprocessed_d = preprocess_description(d);
                        if preprocessed == preprocessed_d {
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                "Your example description is insufficient.".to_string(),
                            ));
                            break;
                        }
                    }
                }
            }
            break;
        }
    }
}

/// Check if a byte is a word character (alphanumeric or underscore).
fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Check if a byte slice starts with "should" (case-insensitive) at a word boundary.
/// Matches RuboCop's `/\Ashould(?:n't|n\xe2\x80\x99t)?\b/i`.
fn starts_with_should(desc: &[u8]) -> bool {
    if desc.len() < 6 {
        return false;
    }
    let lower: Vec<u8> = desc[..6].iter().map(|b| b.to_ascii_lowercase()).collect();
    if lower != b"should" {
        return false;
    }
    // Check for shouldn\xe2\x80\x99t (Unicode right single quote)
    if desc.len() >= 11
        && desc[6].eq_ignore_ascii_case(&b'n')
        && desc[7] == b'\xe2'
        && desc[8] == b'\x80'
        && desc[9] == b'\x99'
        && desc[10].eq_ignore_ascii_case(&b't')
    {
        return desc.len() == 11 || !is_word_char(desc[11]);
    }
    // Check for shouldn't (ASCII apostrophe)
    if desc.len() >= 9
        && desc[6].eq_ignore_ascii_case(&b'n')
        && desc[7] == b'\''
        && desc[8].eq_ignore_ascii_case(&b't')
    {
        return desc.len() == 9 || !is_word_char(desc[9]);
    }
    // Plain "should" — word boundary: end of string or non-word char
    desc.len() == 6 || !is_word_char(desc[6])
}

/// Return the length of the "should" prefix (6 for "should", 9 for "shouldn't",
/// 11 for "shouldn\xe2\x80\x99t").
fn should_prefix_len(desc: &[u8]) -> usize {
    if desc.len() >= 6 {
        let lower6: Vec<u8> = desc[..6].iter().map(|b| b.to_ascii_lowercase()).collect();
        if lower6 == b"should" {
            // Check for "shouldn\xe2\x80\x99t" (Unicode right single quote)
            if desc.len() >= 11
                && desc[6].eq_ignore_ascii_case(&b'n')
                && desc[7] == b'\xe2'
                && desc[8] == b'\x80'
                && desc[9] == b'\x99'
                && desc[10].eq_ignore_ascii_case(&b't')
            {
                return 11;
            }
            // Check for "shouldn't" (ASCII apostrophe)
            if desc.len() >= 9
                && desc[6].eq_ignore_ascii_case(&b'n')
                && desc[7] == b'\''
                && desc[8].eq_ignore_ascii_case(&b't')
            {
                return 9;
            }
            return 6;
        }
    }
    0
}

/// Check if a byte slice starts with "will"/"won't" (case-insensitive) at a word boundary.
/// Matches RuboCop's `/\A(?:will|won't|won\xe2\x80\x99t)\b/i`.
fn starts_with_will(desc: &[u8]) -> bool {
    if desc.len() >= 4 {
        let lower: Vec<u8> = desc[..4].iter().map(|b| b.to_ascii_lowercase()).collect();
        if lower == b"will" {
            return desc.len() == 4 || !is_word_char(desc[4]);
        }
    }
    // won't (ASCII apostrophe)
    if desc.len() >= 5 {
        let lower: Vec<u8> = desc[..5].iter().map(|b| b.to_ascii_lowercase()).collect();
        if lower == b"won't" {
            return desc.len() == 5 || !is_word_char(desc[5]);
        }
    }
    // won\xe2\x80\x99t (Unicode right single quote)
    if desc.len() >= 7 {
        let lower3: Vec<u8> = desc[..3].iter().map(|b| b.to_ascii_lowercase()).collect();
        if lower3 == b"won"
            && desc[3] == b'\xe2'
            && desc[4] == b'\x80'
            && desc[5] == b'\x99'
            && desc[6].eq_ignore_ascii_case(&b't')
        {
            return desc.len() == 7 || !is_word_char(desc[7]);
        }
    }
    false
}

/// Preprocess a description string for DisallowedExamples comparison.
/// Matches RuboCop's `preprocess`: strip + squeeze(' ') + downcase.
fn preprocess_description(s: &str) -> String {
    let trimmed = s.trim();
    let mut result = String::with_capacity(trimmed.len());
    let mut last_was_space = false;
    for c in trimmed.chars() {
        if c == ' ' {
            if !last_was_space {
                result.push(' ');
            }
            last_was_space = true;
        } else {
            result.extend(c.to_lowercase());
            last_was_space = false;
        }
    }
    result
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
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("works".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"it 'works' do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&ExampleWording, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("insufficient"));
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
        assert!(
            diags[0].message.contains("is valid"),
            "CustomTransform should suggest 'is valid' replacement, got: {}",
            diags[0].message
        );
    }

    #[test]
    fn block_arg_not_flagged() {
        // it 'desc', &(proc do...end) — Prism sees this as a BlockArgumentNode,
        // not a BlockNode. RuboCop's on_block pattern only matches actual blocks.
        let source = b"it 'should convert', &(proc do\n  x\nend)\n";
        let diags = crate::testutil::run_cop_full(&ExampleWording, source);
        assert!(
            diags.is_empty(),
            "block argument should not be flagged, got: {:?}",
            diags
        );
    }

    #[test]
    fn should_followed_by_comma() {
        // "should, if ..." — comma is a word boundary, RuboCop flags this
        let source = b"it 'should, if given, cache the file' do\nend\n";
        let diags = crate::testutil::run_cop_full(&ExampleWording, source);
        assert_eq!(diags.len(), 1, "should followed by comma should be flagged");
    }

    #[test]
    fn shouldnt_without_apostrophe_not_flagged() {
        // "shouldnt" (no apostrophe) — 'n' is a word char, no word boundary after "should"
        let source = b"it 'shouldnt create a record' do\nend\n";
        let diags = crate::testutil::run_cop_full(&ExampleWording, source);
        assert!(
            diags.is_empty(),
            "shouldnt without apostrophe should not be flagged, got: {:?}",
            diags
        );
    }

    #[test]
    fn receiver_it_is_checked() {
        // RuboCop's pattern uses _ for receiver, matching group.it('works') { }
        use crate::cop::CopConfig;
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "DisallowedExamples".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("works".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"group.it('works') { }\n";
        let diags = crate::testutil::run_cop_full_with_config(&ExampleWording, source, config);
        assert_eq!(diags.len(), 1, "group.it('works') should be flagged");
    }

    #[test]
    fn xit_and_fit_not_flagged() {
        // RuboCop's ExampleWording only matches :it, not :xit or :fit
        let source = b"xit 'should do something' do\nend\n";
        let diags = crate::testutil::run_cop_full(&ExampleWording, source);
        assert!(diags.is_empty(), "xit should not be flagged");
        let source2 = b"fit 'should do something' do\nend\n";
        let diags2 = crate::testutil::run_cop_full(&ExampleWording, source2);
        assert!(diags2.is_empty(), "fit should not be flagged");
    }

    #[test]
    fn ignored_words_skips_should_check() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "IgnoredWords".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("should".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"it 'should do something' do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&ExampleWording, source, config);
        assert!(diags.is_empty(), "IgnoredWords should skip 'should' check");
    }
}
