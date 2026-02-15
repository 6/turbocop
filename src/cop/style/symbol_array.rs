use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SymbolArray;

impl Cop for SymbolArray {
    fn name(&self) -> &'static str {
        "Style/SymbolArray"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let array_node = match node.as_array_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Must have `[` opening (not %i or %I)
        let opening = match array_node.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        if opening.as_slice() != b"[" {
            return Vec::new();
        }

        let elements = array_node.elements();
        let min_size = config.get_usize("MinSize", 2);
        let enforced_style = config.get_str("EnforcedStyle", "percent");

        // "brackets" style: never flag bracket arrays — they ARE the preferred style
        if enforced_style == "brackets" {
            return Vec::new();
        }

        if elements.len() < min_size {
            return Vec::new();
        }

        // All elements must be symbol nodes
        for elem in elements.iter() {
            if elem.as_symbol_node().is_none() {
                return Vec::new();
            }
        }

        let (line, column) = source.offset_to_line_col(opening.start_offset());
        vec![self.diagnostic(source, line, column, "Use `%i` or `%I` for an array of symbols.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SymbolArray, "cops/style/symbol_array");

    #[test]
    fn config_min_size_5() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([("MinSize".into(), serde_yml::Value::Number(5.into()))]),
            ..CopConfig::default()
        };
        // 5 symbols should trigger with MinSize:5
        let source = b"x = [:a, :b, :c, :d, :e]\n";
        let diags = run_cop_full_with_config(&SymbolArray, source, config.clone());
        assert!(!diags.is_empty(), "Should fire with MinSize:5 on 5-element symbol array");

        // 4 symbols should NOT trigger
        let source2 = b"x = [:a, :b, :c, :d]\n";
        let diags2 = run_cop_full_with_config(&SymbolArray, source2, config);
        assert!(diags2.is_empty(), "Should not fire on 4-element symbol array with MinSize:5");
    }

    #[test]
    fn brackets_style_allows_bracket_arrays() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("brackets".into())),
            ]),
            ..CopConfig::default()
        };
        let source = b"x = [:a, :b, :c]\n";
        let diags = run_cop_full_with_config(&SymbolArray, source, config);
        assert!(diags.is_empty(), "Should not flag brackets with brackets style");
    }
}
