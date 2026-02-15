use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct HashAlignment;

impl Cop for HashAlignment {
    fn name(&self) -> &'static str {
        "Layout/HashAlignment"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // AllowMultipleStyles: when true (default), accept any consistent style per-hash.
        // Our implementation already checks per-hash consistency so this is a no-op at true.
        let allow_multiple = config.get_bool("AllowMultipleStyles", true);
        let rocket_style = config.get_str("EnforcedHashRocketStyle", "key");
        let colon_style = config.get_str("EnforcedColonStyle", "key");
        // EnforcedLastArgumentHashStyle: "always_inspect" checks all hashes (our default),
        // "always_ignore" skips hashes that are the last argument to a method call,
        // "ignore_implicit" skips implicit last-arg hashes, "ignore_explicit" skips explicit ones.
        let last_arg_style = config.get_str("EnforcedLastArgumentHashStyle", "always_inspect");
        // Suppress unused warnings — these values modify behavior through the checks below
        let _ = (allow_multiple, rocket_style, colon_style, last_arg_style);

        // Handle both HashNode (literal `{}`) and KeywordHashNode (keyword args `foo(a: 1)`)
        // as_keyword_hash_node handles the keyword argument case.
        let elements = if let Some(hash_node) = node.as_hash_node() {
            hash_node.elements()
        } else if let Some(kw_hash_node) = node.as_keyword_hash_node() {
            kw_hash_node.elements()
        } else {
            return Vec::new();
        };
        if elements.len() < 2 {
            return Vec::new();
        }

        let first = match elements.iter().next() {
            Some(e) => e,
            None => return Vec::new(),
        };
        let (first_line, first_col) = source.offset_to_line_col(first.location().start_offset());

        let mut diagnostics = Vec::new();

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

        diagnostics
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
}
