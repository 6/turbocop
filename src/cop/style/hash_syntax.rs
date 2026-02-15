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
        let enforced_shorthand = config.get_str("EnforcedShorthandSyntax", "either");
        let use_rockets_symbol_vals = config.get_bool("UseHashRocketsWithSymbolValues", false);
        let prefer_rockets_nonalnum = config.get_bool("PreferHashRocketsForNonAlnumEndingSymbols", false);

        // EnforcedShorthandSyntax: check Ruby 3.1 hash value omission syntax
        // This is checked separately from the main EnforcedStyle
        if enforced_shorthand != "either" {
            let mut shorthand_diags = Vec::new();
            check_shorthand_syntax(self, source, &elements, enforced_shorthand, &mut shorthand_diags);
            if !shorthand_diags.is_empty() {
                return shorthand_diags;
            }
        }

        match enforced_style {
            "ruby19" | "ruby19_no_mixed_keys" => {
                // UseHashRocketsWithSymbolValues: if any value is a symbol, don't flag rockets
                if use_rockets_symbol_vals {
                    let has_symbol_value = elements.iter().any(|elem| {
                        if let Some(assoc) = elem.as_assoc_node() {
                            assoc.value().as_symbol_node().is_some()
                        } else {
                            false
                        }
                    });
                    if has_symbol_value {
                        return Vec::new();
                    }
                }

                let has_unconvertible = elements.iter().any(|elem| {
                    let assoc = match elem.as_assoc_node() {
                        Some(a) => a,
                        None => return false,
                    };
                    let key = assoc.key();
                    if key.as_symbol_node().is_none() {
                        return true;
                    }
                    if let Some(sym) = key.as_symbol_node() {
                        let name = sym.unescaped();
                        if !is_convertible_symbol_key(name) {
                            return true;
                        }
                        // PreferHashRocketsForNonAlnumEndingSymbols
                        if prefer_rockets_nonalnum && !name.is_empty() {
                            let last = name[name.len() - 1];
                            if !last.is_ascii_alphanumeric() && last != b'"' && last != b'\'' {
                                return true;
                            }
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

/// Check EnforcedShorthandSyntax for Ruby 3.1 hash value omission.
fn check_shorthand_syntax(
    cop: &HashSyntax,
    source: &SourceFile,
    elements: &[ruby_prism::Node<'_>],
    enforced_shorthand: &str,
    diags: &mut Vec<Diagnostic>,
) {
    let mut has_shorthand = false;
    let mut has_explicit = false;

    for elem in elements {
        let assoc = match elem.as_assoc_node() {
            Some(a) => a,
            None => continue,
        };
        let key = assoc.key();
        // Only applies to symbol keys in ruby19 style (key: value)
        if key.as_symbol_node().is_none() {
            continue;
        }
        // Check if value uses implicit node (shorthand `{x:}`)
        if assoc.value().as_implicit_node().is_some() {
            has_shorthand = true;
        } else {
            has_explicit = true;
        }
    }

    match enforced_shorthand {
        "always" => {
            // Flag explicit pairs that could use shorthand: value is a local variable
            // read whose name matches the key symbol
            for elem in elements {
                let assoc = match elem.as_assoc_node() {
                    Some(a) => a,
                    None => continue,
                };
                let key = assoc.key();
                let sym = match key.as_symbol_node() {
                    Some(s) => s,
                    None => continue,
                };
                let value = assoc.value();
                if value.as_implicit_node().is_some() {
                    continue; // Already using shorthand
                }
                // Check if value is a local variable read matching the key name
                if let Some(lvar) = value.as_local_variable_read_node() {
                    if lvar.name().as_slice() == sym.unescaped() {
                        let (line, column) =
                            source.offset_to_line_col(key.location().start_offset());
                        diags.push(cop.diagnostic(
                            source,
                            line,
                            column,
                            "Omit the hash value.".to_string(),
                        ));
                    }
                }
            }
        }
        "never" => {
            // Flag shorthand pairs (implicit node values)
            for elem in elements {
                let assoc = match elem.as_assoc_node() {
                    Some(a) => a,
                    None => continue,
                };
                if assoc.value().as_implicit_node().is_some() {
                    let (line, column) =
                        source.offset_to_line_col(assoc.key().location().start_offset());
                    diags.push(cop.diagnostic(
                        source,
                        line,
                        column,
                        "Include the hash value.".to_string(),
                    ));
                }
            }
        }
        "consistent" => {
            // All pairs must use the same style
            if has_shorthand && has_explicit {
                // Flag at the hash level
                let first_elem = elements.first().unwrap();
                let (line, column) =
                    source.offset_to_line_col(first_elem.location().start_offset());
                diags.push(cop.diagnostic(
                    source,
                    line,
                    column,
                    "Don't mix explicit and shorthand hash values.".to_string(),
                ));
            }
        }
        _ => {}
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
    // Note: `=` ending symbols (setter methods like `:foo=`) cannot use
    // Ruby 1.9 hash syntax, so they are NOT convertible.
    let (body, _suffix) = if name.len() > 1 {
        let last = name[name.len() - 1];
        if last == b'?' || last == b'!' {
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

    #[test]
    fn use_hash_rockets_with_symbol_values() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("UseHashRocketsWithSymbolValues".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        // Hash with symbol value should not be flagged when UseHashRocketsWithSymbolValues is true
        let source = b"{ :foo => :bar }\n";
        let diags = run_cop_full_with_config(&HashSyntax, source, config);
        assert!(diags.is_empty(), "Should allow rockets when value is a symbol");
    }

    #[test]
    fn shorthand_never_flags_omission() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedShorthandSyntax".into(), serde_yml::Value::String("never".into())),
            ]),
            ..CopConfig::default()
        };
        // Ruby 3.1 hash value omission: `{x:}` (shorthand)
        let source = b"x = 1; {x:}\n";
        let diags = run_cop_full_with_config(&HashSyntax, source, config);
        assert!(
            diags.iter().any(|d| d.message.contains("Include the hash value")),
            "Should flag shorthand with EnforcedShorthandSyntax: never"
        );
    }

    #[test]
    fn shorthand_either_allows_all() {
        // Default "either" should not flag anything shorthand-related
        let source = b"x = 1; {x:}\n";
        use crate::testutil::run_cop_full;
        let diags = run_cop_full(&HashSyntax, source);
        assert!(
            !diags.iter().any(|d| d.message.contains("hash value")),
            "Default 'either' should not flag shorthand"
        );
    }

    #[test]
    fn prefer_rockets_for_nonalnum_ending_symbols() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("PreferHashRocketsForNonAlnumEndingSymbols".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        // Hash with symbol key ending in `?` should not be flagged (non-alnum ending)
        let source = b"{ :production? => false }\n";
        let diags = run_cop_full_with_config(&HashSyntax, source, config);
        assert!(diags.is_empty(), "Should allow rockets for non-alnum ending symbols");
    }
}
