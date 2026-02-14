use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
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
        let assoc = match node.as_assoc_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let enforced_style = config
            .options
            .get("EnforcedStyle")
            .and_then(|v| v.as_str())
            .unwrap_or("ruby19");

        match enforced_style {
            "ruby19" => {
                // Flag `:key => value` style when key is a simple symbol
                let key = assoc.key();
                if key.as_symbol_node().is_some() {
                    if let Some(op_loc) = assoc.operator_loc() {
                        if op_loc.as_slice() == b"=>" {
                            let (line, column) = source.offset_to_line_col(key.location().start_offset());
                            return vec![Diagnostic {
                                path: source.path_str().to_string(),
                                location: Location { line, column },
                                severity: Severity::Convention,
                                cop_name: self.name().to_string(),
                                message: "Use the new Ruby 1.9 hash syntax.".to_string(),
                            }];
                        }
                    }
                }
            }
            "hash_rockets" => {
                // Flag `key: value` style (no `=>` operator)
                let key = assoc.key();
                if key.as_symbol_node().is_some() {
                    match assoc.operator_loc() {
                        None => {
                            let (line, column) =
                                source.offset_to_line_col(key.location().start_offset());
                            return vec![Diagnostic {
                                path: source.path_str().to_string(),
                                location: Location { line, column },
                                severity: Severity::Convention,
                                cop_name: self.name().to_string(),
                                message: "Use hash rockets syntax.".to_string(),
                            }];
                        }
                        Some(op_loc) => {
                            if op_loc.as_slice() != b"=>" {
                                let (line, column) =
                                    source.offset_to_line_col(key.location().start_offset());
                                return vec![Diagnostic {
                                    path: source.path_str().to_string(),
                                    location: Location { line, column },
                                    severity: Severity::Convention,
                                    cop_name: self.name().to_string(),
                                    message: "Use hash rockets syntax.".to_string(),
                                }];
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{
        assert_cop_no_offenses_full, assert_cop_offenses_full, run_cop_full_with_config,
    };

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &HashSyntax,
            include_bytes!("../../../testdata/cops/style/hash_syntax/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &HashSyntax,
            include_bytes!("../../../testdata/cops/style/hash_syntax/no_offense.rb"),
        );
    }

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
}
