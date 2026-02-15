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
        let enforced_for_classes = config.get_str("EnforcedStyleForClasses", "");
        let enforced_for_modules = config.get_str("EnforcedStyleForModules", "");

        if let Some(class_node) = node.as_class_node() {
            // Use class-specific override if set, otherwise fall back to EnforcedStyle
            let style = if !enforced_for_classes.is_empty() {
                enforced_for_classes
            } else {
                enforced_style
            };
            let constant_path = class_node.constant_path();
            let is_compact = constant_path.as_constant_path_node().is_some();

            if style == "nested" && is_compact {
                let kw_loc = class_node.class_keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                return vec![self.diagnostic(source, line, column, "Use nested module/class definitions instead of compact style.".to_string())];
            } else if style == "compact" && !is_compact {
                let kw_loc = class_node.class_keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                return vec![self.diagnostic(source, line, column, "Use compact module/class definition instead of nested style.".to_string())];
            }
        } else if let Some(module_node) = node.as_module_node() {
            let style = if !enforced_for_modules.is_empty() {
                enforced_for_modules
            } else {
                enforced_style
            };
            let constant_path = module_node.constant_path();
            let is_compact = constant_path.as_constant_path_node().is_some();

            if style == "nested" && is_compact {
                let kw_loc = module_node.module_keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                return vec![self.diagnostic(source, line, column, "Use nested module/class definitions instead of compact style.".to_string())];
            } else if style == "compact" && !is_compact {
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

    #[test]
    fn enforced_style_for_classes_overrides() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("nested".into())),
                ("EnforcedStyleForClasses".into(), serde_yml::Value::String("compact".into())),
            ]),
            ..CopConfig::default()
        };
        // Class should use compact (overridden), module should use nested (default)
        let source = b"class Foo\nend\n";
        let diags = run_cop_full_with_config(&ClassAndModuleChildren, source, config.clone());
        assert_eq!(diags.len(), 1, "Class should be flagged with compact style");
        assert!(diags[0].message.contains("compact"));

        // Module should still use nested style
        let source2 = b"module Foo::Bar\nend\n";
        let diags2 = run_cop_full_with_config(&ClassAndModuleChildren, source2, config);
        assert_eq!(diags2.len(), 1, "Module should be flagged with nested style");
        assert!(diags2[0].message.contains("nested"));
    }

    #[test]
    fn enforced_style_for_modules_overrides() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("nested".into())),
                ("EnforcedStyleForModules".into(), serde_yml::Value::String("compact".into())),
            ]),
            ..CopConfig::default()
        };
        // Module should use compact (overridden)
        let source = b"module Foo\nend\n";
        let diags = run_cop_full_with_config(&ClassAndModuleChildren, source, config);
        assert_eq!(diags.len(), 1, "Module should be flagged with compact style");
        assert!(diags[0].message.contains("compact"));
    }
}
