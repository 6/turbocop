use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct HashSyntax;

impl Cop for HashSyntax {
    fn name(&self) -> &'static str {
        "Style/HashSyntax"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Handle both explicit hashes `{ k: v }` and implicit keyword hashes `foo(k: v)`
        let elements: Vec<ruby_prism::Node<'_>> =
            if let Some(hash_node) = node.as_hash_node() {
                hash_node.elements().iter().collect()
            } else if let Some(kw_hash) = node.as_keyword_hash_node() {
                kw_hash.elements().iter().collect()
            } else {
                return Vec::new();
            };

        let enforced_style = config.get_str("EnforcedStyle", "ruby19");

        match enforced_style {
            "ruby19" | "ruby19_no_mixed_keys" => {
                // For ruby19: flag `:key => value` when the key is a symbol
                // that can be expressed in the new syntax. Skip the entire hash
                // if any key can't be converted (string keys, non-symbol keys).
                let has_unconvertible = elements.iter().any(|elem| {
                    let assoc = match elem.as_assoc_node() {
                        Some(a) => a,
                        None => return false,
                    };
                    let key = assoc.key();
                    if key.as_symbol_node().is_none() {
                        // String key or other non-symbol key
                        return true;
                    }
                    // Check if symbol key can be expressed in ruby19 syntax
                    // (must be a valid identifier: starts with letter/underscore,
                    // contains only word chars, optionally ends with ? or !)
                    if let Some(sym) = key.as_symbol_node() {
                        let name = sym.unescaped();
                        if !is_convertible_symbol_key(name) {
                            return true;
                        }
                    }
                    false
                });

                if has_unconvertible {
                    return Vec::new();
                }

                let mut diags = Vec::new();
                for elem in &elements {
                    let assoc = match elem.as_assoc_node() {
                        Some(a) => a,
                        None => continue,
                    };
                    let key = assoc.key();
                    if key.as_symbol_node().is_some() {
                        if let Some(op_loc) = assoc.operator_loc() {
                            if op_loc.as_slice() == b"=>" {
                                let (line, column) =
                                    source.offset_to_line_col(key.location().start_offset());
                                diags.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    "Use the new Ruby 1.9 hash syntax.".to_string(),
                                ));
                            }
                        }
                    }
                }
                diags
            }
            "hash_rockets" => {
                let mut diags = Vec::new();
                for elem in &elements {
                    let assoc = match elem.as_assoc_node() {
                        Some(a) => a,
                        None => continue,
                    };
                    let key = assoc.key();
                    if key.as_symbol_node().is_some() {
                        let uses_rocket = assoc
                            .operator_loc()
                            .is_some_and(|op| op.as_slice() == b"=>");
                        if !uses_rocket {
                            let (line, column) =
                                source.offset_to_line_col(key.location().start_offset());
                            diags.push(self.diagnostic(
                                source,
                                line,
                                column,
                                "Use hash rockets syntax.".to_string(),
                            ));
                        }
                    }
                }
                diags
            }
            "no_mixed_keys" => {
                // All keys must use the same syntax
                let mut has_ruby19 = false;
                let mut has_rockets = false;
                for elem in &elements {
                    let assoc = match elem.as_assoc_node() {
                        Some(a) => a,
                        None => continue,
                    };
                    if let Some(op_loc) = assoc.operator_loc() {
                        if op_loc.as_slice() == b"=>" {
                            has_rockets = true;
                        } else {
                            has_ruby19 = true;
                        }
                    } else {
                        has_ruby19 = true;
                    }
                }
                if has_ruby19 && has_rockets {
                    let (line, column) = source.offset_to_line_col(node.location().start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Don't mix styles in the same hash.".to_string(),
                    )];
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    }
}

/// Check if a symbol key can be expressed in Ruby 1.9 hash syntax.
/// Valid: `:foo` → `foo:`, `:foo_bar` → `foo_bar:`, `:foo?` → `foo?:`
/// Invalid: `:"foo-bar"`, `:"foo bar"`, `:"123"`
fn is_convertible_symbol_key(name: &[u8]) -> bool {
    if name.is_empty() {
        return false;
    }
    // Must start with a letter or underscore
    let first = name[0];
    if !first.is_ascii_alphabetic() && first != b'_' {
        return false;
    }
    // Rest must be word characters, optionally ending with ? or !
    let (body, _suffix) = if name.len() > 1 {
        let last = name[name.len() - 1];
        if last == b'?' || last == b'!' || last == b'=' {
            (&name[1..name.len() - 1], Some(last))
        } else {
            (&name[1..], None)
        }
    } else {
        (&[] as &[u8], None)
    };
    body.iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full_with_config;

    crate::cop_fixture_tests!(HashSyntax, "cops/style/hash_syntax");

    #[test]
    fn config_hash_rockets() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("hash_rockets".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"{ a: 1 }\n";
        let diags = run_cop_full_with_config(&HashSyntax, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("hash rockets"));
    }

    #[test]
    fn mixed_key_types_skipped_in_ruby19() {
        use crate::testutil::run_cop_full;
        // Hash with string key and symbol key — should not be flagged
        let source = b"{ \"@type\" => \"Person\", :name => \"foo\" }\n";
        let diags = run_cop_full(&HashSyntax, source);
        assert!(diags.is_empty(), "Mixed key hash should not be flagged");
    }
}
