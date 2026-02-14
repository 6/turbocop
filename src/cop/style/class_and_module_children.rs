use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ClassAndModuleChildren;

impl Cop for ClassAndModuleChildren {
    fn name(&self) -> &'static str {
        "Style/ClassAndModuleChildren"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "nested");

        if let Some(class_node) = node.as_class_node() {
            let constant_path = class_node.constant_path();
            let is_compact = constant_path.as_constant_path_node().is_some();

            if enforced_style == "nested" && is_compact {
                let kw_loc = class_node.class_keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                return vec![self.diagnostic(source, line, column, "Use nested module/class definitions instead of compact style.".to_string())];
            } else if enforced_style == "compact" && !is_compact {
                let kw_loc = class_node.class_keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                return vec![self.diagnostic(source, line, column, "Use compact module/class definition instead of nested style.".to_string())];
            }
        } else if let Some(module_node) = node.as_module_node() {
            let constant_path = module_node.constant_path();
            let is_compact = constant_path.as_constant_path_node().is_some();

            if enforced_style == "nested" && is_compact {
                let kw_loc = module_node.module_keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                return vec![self.diagnostic(source, line, column, "Use nested module/class definitions instead of compact style.".to_string())];
            } else if enforced_style == "compact" && !is_compact {
                let kw_loc = module_node.module_keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                return vec![self.diagnostic(source, line, column, "Use compact module/class definition instead of nested style.".to_string())];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(ClassAndModuleChildren, "cops/style/class_and_module_children");

    #[test]
    fn config_compact_style() {
        use std::collections::HashMap;
        use crate::testutil::{run_cop_full_with_config, assert_cop_no_offenses_full_with_config};

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("compact".into())),
            ]),
            ..CopConfig::default()
        };
        // Nested style should trigger with compact enforced
        let source = b"class Foo\nend\n";
        let diags = run_cop_full_with_config(&ClassAndModuleChildren, source, config.clone());
        assert!(!diags.is_empty(), "Should fire with EnforcedStyle:compact on nested class");
        assert!(diags[0].message.contains("compact"));

        // Compact style should be clean
        let source2 = b"class Foo::Bar\nend\n";
        assert_cop_no_offenses_full_with_config(&ClassAndModuleChildren, source2, config);
    }
}
