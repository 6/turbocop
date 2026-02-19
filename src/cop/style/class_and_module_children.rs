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
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "nested").to_string();
        let enforced_for_classes = config.get_str("EnforcedStyleForClasses", "").to_string();
        let enforced_for_modules = config.get_str("EnforcedStyleForModules", "").to_string();

        let mut visitor = ChildrenVisitor {
            source,
            enforced_style,
            enforced_for_classes,
            enforced_for_modules,
            inside_class_or_module: false,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
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
    inside_class_or_module: bool,
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

    /// Check if the body of a class/module is a single class or module definition
    /// that could be compacted. In Prism, the body is either a StatementsNode
    /// containing a single child, or None.
    fn body_is_single_class_or_module(
        &self,
        body: &Option<ruby_prism::Node<'a>>,
    ) -> bool {
        let Some(body_node) = body else {
            return false;
        };
        // The body is typically a StatementsNode wrapping one or more statements
        if let Some(stmts) = body_node.as_statements_node() {
            let children: Vec<_> = stmts.body().iter().collect();
            if children.len() == 1 {
                let child = &children[0];
                return child.as_class_node().is_some() || child.as_module_node().is_some();
            }
        }
        // If the body is directly a class or module (shouldn't normally happen but handle it)
        body_node.as_class_node().is_some() || body_node.as_module_node().is_some()
    }

    fn check_nested_style(&mut self, is_compact: bool, kw_offset: usize) {
        // For nested style: flag compact-style definitions (with ::) at top level
        if !is_compact {
            return;
        }
        // Skip if inside another class/module (RuboCop: return if node.parent&.type?(:class, :module))
        if self.inside_class_or_module {
            return;
        }
        self.add_diagnostic(
            kw_offset,
            "Use nested module/class definitions instead of compact style.".to_string(),
        );
    }

    fn check_compact_style(
        &mut self,
        body: &Option<ruby_prism::Node<'a>>,
        kw_offset: usize,
    ) {
        // For compact style: flag outer nodes whose body is a single class/module
        // Skip if inside another class/module (RuboCop: return if parent&.type?(:class, :module))
        if self.inside_class_or_module {
            return;
        }
        if !self.body_is_single_class_or_module(body) {
            return;
        }
        self.add_diagnostic(
            kw_offset,
            "Use compact module/class definition instead of nested style.".to_string(),
        );
    }
}

impl<'a> Visit<'a> for ChildrenVisitor<'a> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'a>) {
        let style = self.style_for_class().to_string();
        let constant_path = node.constant_path();
        let is_compact = constant_path.as_constant_path_node().is_some();
        let kw_offset = node.class_keyword_loc().start_offset();

        // RuboCop: return if node.parent_class && style != :nested
        // Skip classes with superclass unless checking nested style
        let has_superclass = node.superclass().is_some();
        if has_superclass && style != "nested" {
            // Still visit children
            let prev = self.inside_class_or_module;
            self.inside_class_or_module = true;
            ruby_prism::visit_class_node(self, node);
            self.inside_class_or_module = prev;
            return;
        }

        if style == "nested" {
            self.check_nested_style(is_compact, kw_offset);
        } else if style == "compact" {
            let body = node.body();
            self.check_compact_style(&body, kw_offset);
        }

        // Visit children inside class/module context
        let prev = self.inside_class_or_module;
        self.inside_class_or_module = true;
        ruby_prism::visit_class_node(self, node);
        self.inside_class_or_module = prev;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'a>) {
        let style = self.style_for_module().to_string();
        let constant_path = node.constant_path();
        let is_compact = constant_path.as_constant_path_node().is_some();
        let kw_offset = node.module_keyword_loc().start_offset();

        if style == "nested" {
            self.check_nested_style(is_compact, kw_offset);
        } else if style == "compact" {
            let body = node.body();
            self.check_compact_style(&body, kw_offset);
        }

        // Visit children inside class/module context
        let prev = self.inside_class_or_module;
        self.inside_class_or_module = true;
        ruby_prism::visit_module_node(self, node);
        self.inside_class_or_module = prev;
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
        // Top-level class with no children — should NOT trigger
        let source = b"class Foo\nend\n";
        assert_cop_no_offenses_full_with_config(&ClassAndModuleChildren, source, config.clone());

        // Module wrapping a single class — SHOULD trigger (on the module)
        let source2 = b"module A\n  class Foo\n  end\nend\n";
        let diags = run_cop_full_with_config(&ClassAndModuleChildren, source2, config.clone());
        assert_eq!(diags.len(), 1, "Should fire for module wrapping a single class");
        assert!(diags[0].message.contains("compact"));

        // Compact style class should be clean
        let source3 = b"class Foo::Bar\nend\n";
        assert_cop_no_offenses_full_with_config(&ClassAndModuleChildren, source3, config.clone());

        // Class wrapping a single class — should NOT trigger (inside_class_or_module
        // is not the issue; the outer class has a child class but classes with children
        // still get checked. However, the outer class has a superclass? No. Let's verify.)
        let source4 = b"class A\n  class Foo\n  end\nend\n";
        let diags4 = run_cop_full_with_config(&ClassAndModuleChildren, source4, config.clone());
        // RuboCop DOES flag this: outer class wraps a single class child.
        // But wait -- does it? Let me check: on_class returns early if parent_class && style != :nested.
        // class A has no parent_class (superclass), so it proceeds to check_compact_style.
        // The body is a single class, so it flags it.
        assert_eq!(diags4.len(), 1, "Module wrapping single class should be flagged");

        // Class with superclass wrapping a class — should NOT trigger
        // (on_class returns early: node.parent_class && style != :nested)
        let source5 = b"class A < Base\n  class Foo\n  end\nend\n";
        assert_cop_no_offenses_full_with_config(&ClassAndModuleChildren, source5, config);
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
    fn compact_style_class_inside_class_with_superclass_no_offense() {
        use std::collections::HashMap;
        use crate::testutil::assert_cop_no_offenses_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("compact".into())),
            ]),
            ..CopConfig::default()
        };
        // Class with superclass wrapping a child class — RuboCop skips this because
        // on_class returns early when parent_class is present and style != :nested.
        // This is the chatwoot pattern (e.g., class InboxPolicy < ApplicationPolicy; class Scope; end; end)
        let source = b"class InboxPolicy < ApplicationPolicy\n  class Scope\n    def resolve\n      super\n    end\n  end\nend\n";
        assert_cop_no_offenses_full_with_config(&ClassAndModuleChildren, source, config.clone());

        // Module wrapping multiple classes — should NOT flag (body is not a single class)
        let source2 = b"module CustomExceptions::Account\n  class InvalidEmail < Base\n    def message; end\n  end\n  class UserExists < Base\n    def message; end\n  end\nend\n";
        assert_cop_no_offenses_full_with_config(&ClassAndModuleChildren, source2, config.clone());

        // Module wrapping a single class — SHOULD flag
        let source3 = b"module Api\n  class SessionsController\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&ClassAndModuleChildren, source3, config);
        assert_eq!(diags.len(), 1, "Module wrapping single class should be flagged with compact style");
    }

    #[test]
    fn compact_style_nested_inside_other_class_module_not_flagged() {
        use std::collections::HashMap;
        use crate::testutil::{run_cop_full_with_config, assert_cop_no_offenses_full_with_config};

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("compact".into())),
            ]),
            ..CopConfig::default()
        };
        // Class (no superclass) wrapping module — RuboCop DOES flag this (body is single module)
        let source = b"class Foo\n  module Bar\n    class Baz\n    end\n  end\nend\n";
        let diags = run_cop_full_with_config(&ClassAndModuleChildren, source, config.clone());
        assert_eq!(diags.len(), 1, "Class wrapping single module should be flagged");

        // But the inner module (Bar wrapping Baz) should NOT be flagged separately
        // because Bar is inside a class/module (Foo). Only the outermost is flagged.
        assert!(diags[0].location.line == 1, "Only the outer class should be flagged");

        // Class with superclass wrapping module — should NOT be flagged
        let source2 = b"class Foo < Base\n  module Bar\n    class Baz\n    end\n  end\nend\n";
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
        // Top-level class wrapping a single class — should be flagged (compact for classes)
        let source = b"class A\n  class Foo\n  end\nend\n";
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
        // Module wrapping a single module — should be flagged (compact for modules)
        let source = b"module A\n  module Foo\n  end\nend\n";
        let diags = run_cop_full_with_config(&ClassAndModuleChildren, source, config);
        assert_eq!(diags.len(), 1, "Module should be flagged with compact style");
        assert!(diags[0].message.contains("compact"));
    }
}
