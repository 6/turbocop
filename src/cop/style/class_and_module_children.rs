use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ClassAndModuleChildren;

impl Cop for ClassAndModuleChildren {
    fn name(&self) -> &'static str {
        "Style/ClassAndModuleChildren"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "nested").to_string();
        let enforced_for_classes = config.get_str("EnforcedStyleForClasses", "").to_string();
        let enforced_for_modules = config.get_str("EnforcedStyleForModules", "").to_string();

        let mut visitor = ChildrenVisitor {
            source,
            enforced_style,
            enforced_for_classes,
            enforced_for_modules,
            nesting_depth: 0,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }

    fn diagnostic(
        &self,
        source: &SourceFile,
        line: usize,
        column: usize,
        message: String,
    ) -> Diagnostic {
        Diagnostic {
            path: source.path_str().to_string(),
            location: crate::diagnostic::Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message,
        }
    }
}

struct ChildrenVisitor<'a> {
    source: &'a SourceFile,
    enforced_style: String,
    enforced_for_classes: String,
    enforced_for_modules: String,
    nesting_depth: usize, // 0 = top-level, 1 = inside one class/module, etc.
    diagnostics: Vec<Diagnostic>,
}

impl<'a> ChildrenVisitor<'a> {
    fn style_for_class(&self) -> &str {
        if !self.enforced_for_classes.is_empty() {
            &self.enforced_for_classes
        } else {
            &self.enforced_style
        }
    }

    fn style_for_module(&self) -> &str {
        if !self.enforced_for_modules.is_empty() {
            &self.enforced_for_modules
        } else {
            &self.enforced_style
        }
    }

    fn add_diagnostic(&mut self, offset: usize, message: String) {
        let (line, column) = self.source.offset_to_line_col(offset);
        self.diagnostics.push(Diagnostic {
            path: self.source.path_str().to_string(),
            location: crate::diagnostic::Location { line, column },
            severity: crate::diagnostic::Severity::Convention,
            cop_name: "Style/ClassAndModuleChildren".to_string(),
            message,
        });
    }
}

impl<'a> Visit<'a> for ChildrenVisitor<'a> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'a>) {
        let style = self.style_for_class().to_string();
        let constant_path = node.constant_path();
        let is_compact = constant_path.as_constant_path_node().is_some();
        let kw_offset = node.class_keyword_loc().start_offset();

        if style == "nested" && is_compact {
            // Compact style used but nested is enforced — always an offense
            self.add_diagnostic(
                kw_offset,
                "Use nested module/class definitions instead of compact style.".to_string(),
            );
        } else if style == "compact" && !is_compact && self.nesting_depth > 0 {
            // Non-compact (simple name) inside another class/module — should use compact
            self.add_diagnostic(
                kw_offset,
                "Use compact module/class definition instead of nested style.".to_string(),
            );
        }

        // Visit children with increased nesting depth
        self.nesting_depth += 1;
        ruby_prism::visit_class_node(self, node);
        self.nesting_depth -= 1;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'a>) {
        let style = self.style_for_module().to_string();
        let constant_path = node.constant_path();
        let is_compact = constant_path.as_constant_path_node().is_some();
        let kw_offset = node.module_keyword_loc().start_offset();

        if style == "nested" && is_compact {
            // Compact style used but nested is enforced — always an offense
            self.add_diagnostic(
                kw_offset,
                "Use nested module/class definitions instead of compact style.".to_string(),
            );
        } else if style == "compact" && !is_compact && self.nesting_depth > 0 {
            // Non-compact (simple name) inside another class/module — should use compact
            self.add_diagnostic(
                kw_offset,
                "Use compact module/class definition instead of nested style.".to_string(),
            );
        }

        // Visit children with increased nesting depth
        self.nesting_depth += 1;
        ruby_prism::visit_module_node(self, node);
        self.nesting_depth -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(ClassAndModuleChildren, "cops/style/class_and_module_children");

    #[test]
    fn config_compact_style_only_flags_nested() {
        use std::collections::HashMap;
        use crate::testutil::{run_cop_full_with_config, assert_cop_no_offenses_full_with_config};

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("compact".into())),
            ]),
            ..CopConfig::default()
        };
        // Top-level non-compact class — should NOT trigger (nothing to compact into)
        let source = b"class Foo\nend\n";
        assert_cop_no_offenses_full_with_config(&ClassAndModuleChildren, source, config.clone());

        // Nested non-compact class — SHOULD trigger
        let source2 = b"module A\n  class Foo\n  end\nend\n";
        let diags = run_cop_full_with_config(&ClassAndModuleChildren, source2, config.clone());
        assert_eq!(diags.len(), 1, "Should fire for nested class with compact style");
        assert!(diags[0].message.contains("compact"));

        // Compact style should be clean
        let source3 = b"class Foo::Bar\nend\n";
        assert_cop_no_offenses_full_with_config(&ClassAndModuleChildren, source3, config);
    }

    #[test]
    fn top_level_module_no_offense_with_compact() {
        use std::collections::HashMap;
        use crate::testutil::assert_cop_no_offenses_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("compact".into())),
            ]),
            ..CopConfig::default()
        };
        let source = b"module Foo\nend\n";
        assert_cop_no_offenses_full_with_config(&ClassAndModuleChildren, source, config);
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
        // Nested class inside module should be flagged (compact for classes)
        let source = b"module A\n  class Foo\n  end\nend\n";
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
        // Module nested inside another module should be flagged (compact for modules)
        let source = b"module A\n  module Foo\n  end\nend\n";
        let diags = run_cop_full_with_config(&ClassAndModuleChildren, source, config);
        assert_eq!(diags.len(), 1, "Module should be flagged with compact style");
        assert!(diags[0].message.contains("compact"));
    }
}
