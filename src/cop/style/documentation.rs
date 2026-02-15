use crate::cop::util::preceding_comment_line;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Documentation;

/// Extract the short (unqualified) name from a constant node.
/// For `Foo::Bar`, returns `"Bar"`. For `Foo`, returns `"Foo"`.
fn extract_short_name(node: &ruby_prism::Node<'_>) -> String {
    if let Some(path) = node.as_constant_path_node() {
        // Qualified name like Foo::Bar — get the last segment
        let name_loc = path.name_loc();
        std::str::from_utf8(name_loc.as_slice())
            .unwrap_or("")
            .to_string()
    } else if let Some(read) = node.as_constant_read_node() {
        std::str::from_utf8(read.name().as_slice())
            .unwrap_or("")
            .to_string()
    } else {
        String::new()
    }
}

impl Cop for Documentation {
    fn name(&self) -> &'static str {
        "Style/Documentation"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allowed_constants = config.get_string_array("AllowedConstants").unwrap_or_default();
        if let Some(class_node) = node.as_class_node() {
            // Check if class name is in AllowedConstants
            let name = extract_short_name(&class_node.constant_path());
            if allowed_constants.iter().any(|c| c == &name) {
                return Vec::new();
            }
            let kw_loc = class_node.class_keyword_loc();
            let start = kw_loc.start_offset();
            if !preceding_comment_line(source, start) {
                let (line, column) = source.offset_to_line_col(start);
                return vec![self.diagnostic(source, line, column, "Missing top-level documentation comment for `class`.".to_string())];
            }
        } else if let Some(module_node) = node.as_module_node() {
            let name = extract_short_name(&module_node.constant_path());
            if allowed_constants.iter().any(|c| c == &name) {
                return Vec::new();
            }
            let kw_loc = module_node.module_keyword_loc();
            let start = kw_loc.start_offset();
            if !preceding_comment_line(source, start) {
                let (line, column) = source.offset_to_line_col(start);
                return vec![self.diagnostic(source, line, column, "Missing top-level documentation comment for `module`.".to_string())];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};

    crate::cop_fixture_tests!(Documentation, "cops/style/documentation");

    #[test]
    fn first_line_class_has_no_preceding_comment() {
        let source = b"class Foo\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("class"));
    }

    #[test]
    fn module_without_comment() {
        let source = b"module Bar\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("module"));
    }

    #[test]
    fn allowed_constants_exempts_class() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("AllowedConstants".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("ClassMethods".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        // ClassMethods should be exempt
        let source = b"module ClassMethods\nend\n";
        let diags = run_cop_full_with_config(&Documentation, source, config);
        assert!(diags.is_empty(), "AllowedConstants should exempt ClassMethods");
    }

    #[test]
    fn allowed_constants_does_not_exempt_other_names() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("AllowedConstants".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("ClassMethods".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        // Foo is NOT in AllowedConstants, should still be flagged
        let source = b"class Foo\nend\n";
        let diags = run_cop_full_with_config(&Documentation, source, config);
        assert_eq!(diags.len(), 1, "Non-allowed constant should still be flagged");
    }
}
