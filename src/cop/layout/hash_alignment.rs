use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{HASH_NODE, KEYWORD_HASH_NODE};

pub struct HashAlignment;

impl Cop for HashAlignment {
    fn name(&self) -> &'static str {
        "Layout/HashAlignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[HASH_NODE, KEYWORD_HASH_NODE]
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
        // AllowMultipleStyles: when true (default), accept any consistent style per-hash.
        // Our implementation already checks per-hash consistency so this is a no-op at true.
        let _allow_multiple = config.get_bool("AllowMultipleStyles", true);
        let _rocket_style = config.get_str("EnforcedHashRocketStyle", "key");
        let _colon_style = config.get_str("EnforcedColonStyle", "key");
        // EnforcedLastArgumentHashStyle: "always_inspect" checks all hashes (our default),
        // "always_ignore" skips hashes that are the last argument to a method call,
        // "ignore_implicit" skips implicit last-arg hashes, "ignore_explicit" skips explicit ones.
        let last_arg_style = config.get_str("EnforcedLastArgumentHashStyle", "always_inspect");
        // ArgumentAlignmentStyle is injected from Layout/ArgumentAlignment config.
        // When "with_fixed_indentation", RuboCop skips hash alignment on hashes that
        // are call arguments where the first pair is on the same line as the method
        // call (autocorrect_incompatible_with_other_cops? in RuboCop).
        let arg_alignment_style = config.get_str("ArgumentAlignmentStyle", "with_first_argument");
        let fixed_indentation = arg_alignment_style == "with_fixed_indentation";

        // Handle both HashNode (literal `{}`) and KeywordHashNode (keyword args `foo(a: 1)`)
        let is_keyword_hash = node.as_keyword_hash_node().is_some();
        let (elements, hash_node_start) = if let Some(hash_node) = node.as_hash_node() {
            (hash_node.elements(), hash_node.location().start_offset())
        } else if let Some(kw_hash_node) = node.as_keyword_hash_node() {
            (kw_hash_node.elements(), kw_hash_node.location().start_offset())
        } else {
            return;
        };
        if elements.len() < 2 {
            return;
        }

        let first = match elements.iter().next() {
            Some(e) => e,
            None => return,
        };

        // EnforcedLastArgumentHashStyle handling:
        // KeywordHashNode is always an implicit hash in a method call.
        // HashNode may be an explicit hash in a method call.
        if is_keyword_hash {
            match last_arg_style {
                "always_ignore" | "ignore_implicit" => return,
                _ => {}
            }
        } else {
            // For HashNode: "always_ignore" and "ignore_explicit" skip explicit
            // hashes that are call arguments. We approximate by checking if the
            // opening `{` is not the first token on its line (preceded by a call).
            let is_call_arg = !crate::cop::util::begins_its_line(source, hash_node_start);
            if is_call_arg {
                match last_arg_style {
                    "always_ignore" | "ignore_explicit" => return,
                    _ => {}
                }
            }
        }

        // Mirror RuboCop's autocorrect_incompatible_with_other_cops? check:
        // When Layout/ArgumentAlignment uses with_fixed_indentation, skip hash
        // alignment on hashes that are arguments to a method call where the first
        // pair is on the same line as the method call selector.
        if fixed_indentation {
            let first_begins_line = crate::cop::util::begins_its_line(
                source,
                first.location().start_offset(),
            );
            if is_keyword_hash {
                // KeywordHashNode is always inside a method call.
                // If the first element does NOT begin its line, the method call
                // name precedes it on the same line → skip.
                if !first_begins_line {
                    return;
                }
            } else {
                // For HashNode: if the opening `{` is not the first token on its
                // line (i.e. follows a method call) AND the first pair is on the
                // same line as the `{`, it's a call-argument hash on the same
                // line as the selector → skip.
                let hash_begins_line = crate::cop::util::begins_its_line(source, hash_node_start);
                if !hash_begins_line && !first_begins_line {
                    return;
                }
            }
        }

        let (first_line, first_col) = source.offset_to_line_col(first.location().start_offset());

        let mut last_checked_line = first_line;

        for elem in elements.iter().skip(1) {
            let (elem_line, elem_col) = source.offset_to_line_col(elem.location().start_offset());
            // Only check the first element on each new line
            if elem_line == last_checked_line {
                continue;
            }
            last_checked_line = elem_line;
            // Skip elements that don't begin their line (e.g. `}, status: 200`
            // where `}` is first on the line, not `status:`)
            if !crate::cop::util::begins_its_line(source, elem.location().start_offset()) {
                continue;
            }
            if elem_col != first_col {
                diagnostics.push(self.diagnostic(
                    source,
                    elem_line,
                    elem_col,
                    "Align the elements of a hash literal if they span more than one line."
                        .to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(HashAlignment, "cops/layout/hash_alignment");

    #[test]
    fn single_line_hash_no_offense() {
        let source = b"x = { a: 1, b: 2 }\n";
        let diags = run_cop_full(&HashAlignment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn config_options_are_read() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedHashRocketStyle".into(), serde_yml::Value::String("key".into())),
                ("EnforcedColonStyle".into(), serde_yml::Value::String("key".into())),
            ]),
            ..CopConfig::default()
        };
        // Key-aligned hash should be accepted
        let src = b"x = {\n  a: 1,\n  b: 2\n}\n";
        let diags = run_cop_full_with_config(&HashAlignment, src, config);
        assert!(diags.is_empty(), "key-aligned hash should be accepted");
    }

    #[test]
    fn fixed_indentation_skips_keyword_hash_on_same_line() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("ArgumentAlignmentStyle".into(), serde_yml::Value::String("with_fixed_indentation".into())),
            ]),
            ..CopConfig::default()
        };
        // Keyword hash args where first key is on same line as method call
        // should be skipped when ArgumentAlignment uses with_fixed_indentation
        let src = b"render html: \"hello\",\n  layout: \"application\"\n";
        let diags = run_cop_full_with_config(&HashAlignment, src, config);
        assert!(diags.is_empty(), "keyword hash on same line as call should be skipped with fixed indentation");
    }

    #[test]
    fn fixed_indentation_still_checks_keyword_hash_on_own_line() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("ArgumentAlignmentStyle".into(), serde_yml::Value::String("with_fixed_indentation".into())),
            ]),
            ..CopConfig::default()
        };
        // Keyword hash args where first key begins its own line should still be checked
        let src = b"render(\n  html: \"hello\",\n    layout: \"application\"\n)\n";
        let diags = run_cop_full_with_config(&HashAlignment, src, config);
        assert_eq!(diags.len(), 1, "keyword hash on own line should still be checked with fixed indentation");
    }

    #[test]
    fn default_config_flags_keyword_hash_on_same_line() {
        // Without with_fixed_indentation, keyword hash args are checked normally
        let src = b"render html: \"hello\",\n  layout: \"application\"\n";
        let diags = run_cop_full(&HashAlignment, src);
        assert_eq!(diags.len(), 1, "keyword hash should be flagged without fixed indentation");
    }

    #[test]
    fn always_ignore_skips_keyword_hash() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedLastArgumentHashStyle".into(), serde_yml::Value::String("always_ignore".into())),
            ]),
            ..CopConfig::default()
        };
        let src = b"render html: \"hello\",\n  layout: \"application\"\n";
        let diags = run_cop_full_with_config(&HashAlignment, src, config);
        assert!(diags.is_empty(), "always_ignore should skip keyword hash args");
    }

    #[test]
    fn ignore_implicit_skips_keyword_hash() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedLastArgumentHashStyle".into(), serde_yml::Value::String("ignore_implicit".into())),
            ]),
            ..CopConfig::default()
        };
        let src = b"render html: \"hello\",\n  layout: \"application\"\n";
        let diags = run_cop_full_with_config(&HashAlignment, src, config);
        assert!(diags.is_empty(), "ignore_implicit should skip implicit keyword hash args");
    }
}
