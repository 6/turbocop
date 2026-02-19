use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, CLASS_NODE, DEF_NODE, MODULE_NODE, STATEMENTS_NODE};

pub struct IndentationConsistency;

/// Check if a node is a bare access modifier call (private, protected, public with no args).
fn is_bare_access_modifier(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    // Must be a bare call: no receiver, no arguments, no block
    if call.receiver().is_some() || call.arguments().is_some() || call.block().is_some() {
        return false;
    }
    matches!(call.name().as_slice(), b"private" | b"protected" | b"public")
}

impl IndentationConsistency {
    fn check_body_consistency(
        &self,
        source: &SourceFile,
        keyword_offset: usize,
        body: Option<ruby_prism::Node<'_>>,
        indented_internal_methods: bool,
    ) -> Vec<Diagnostic> {
        let body = match body {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let children: Vec<_> = stmts.body().iter().collect();
        if children.len() < 2 {
            return Vec::new();
        }

        let (kw_line, _) = source.offset_to_line_col(keyword_offset);

        // Check if first statement is on the same line as keyword
        let first_loc = children[0].location();
        let (first_line, _) = source.offset_to_line_col(first_loc.start_offset());
        if first_line == kw_line {
            return Vec::new();
        }

        if indented_internal_methods {
            self.check_sections(source, &children)
        } else {
            self.check_flat(source, &children, kw_line)
        }
    }

    /// Normal style: all children must have the same indentation.
    fn check_flat(
        &self,
        source: &SourceFile,
        children: &[ruby_prism::Node<'_>],
        kw_line: usize,
    ) -> Vec<Diagnostic> {
        let first_loc = children[0].location();
        let (first_line, first_col) = source.offset_to_line_col(first_loc.start_offset());

        let mut diagnostics = Vec::new();
        let mut prev_line = first_line;

        for child in &children[1..] {
            let loc = child.location();
            let (child_line, child_col) = source.offset_to_line_col(loc.start_offset());

            // Skip semicolon-separated statements on the same line as previous sibling
            if child_line == prev_line || child_line == kw_line {
                prev_line = child_line;
                continue;
            }
            prev_line = child_line;

            if child_col != first_col {
                diagnostics.push(self.diagnostic(
                    source,
                    child_line,
                    child_col,
                    "Inconsistent indentation detected.".to_string(),
                ));
            }
        }

        diagnostics
    }

    /// indented_internal_methods style: access modifiers act as section dividers.
    /// Consistency is checked only within each section.
    fn check_sections(
        &self,
        source: &SourceFile,
        children: &[ruby_prism::Node<'_>],
    ) -> Vec<Diagnostic> {
        // Split children into sections separated by bare access modifiers.
        // Each section's children must have consistent indentation within the section,
        // but different sections can have different indentation levels.
        let mut sections: Vec<Vec<&ruby_prism::Node<'_>>> = vec![vec![]];

        for child in children {
            if is_bare_access_modifier(child) {
                // Start a new section (the modifier itself is not checked)
                sections.push(vec![]);
            } else {
                sections.last_mut().unwrap().push(child);
            }
        }

        let mut diagnostics = Vec::new();

        for section in &sections {
            if section.len() < 2 {
                continue;
            }

            let first_loc = section[0].location();
            let (first_line, first_col) = source.offset_to_line_col(first_loc.start_offset());
            let mut prev_line = first_line;

            for child in &section[1..] {
                let loc = child.location();
                let (child_line, child_col) = source.offset_to_line_col(loc.start_offset());

                // Skip semicolon-separated statements on same line as previous sibling
                if child_line == prev_line {
                    prev_line = child_line;
                    continue;
                }
                prev_line = child_line;

                if child_col != first_col {
                    diagnostics.push(self.diagnostic(
                        source,
                        child_line,
                        child_col,
                        "Inconsistent indentation detected.".to_string(),
                    ));
                }
            }
        }

        diagnostics
    }
}

impl Cop for IndentationConsistency {
    fn name(&self) -> &'static str {
        "Layout/IndentationConsistency"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, CLASS_NODE, DEF_NODE, MODULE_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let style = config.get_str("EnforcedStyle", "normal");
        let indented = style == "indented_internal_methods";

        if let Some(class_node) = node.as_class_node() {
            diagnostics.extend(self.check_body_consistency(
                source,
                class_node.class_keyword_loc().start_offset(),
                class_node.body(),
                indented,
            ));
            return;
        }

        if let Some(module_node) = node.as_module_node() {
            diagnostics.extend(self.check_body_consistency(
                source,
                module_node.module_keyword_loc().start_offset(),
                module_node.body(),
                indented,
            ));
            return;
        }

        if let Some(def_node) = node.as_def_node() {
            diagnostics.extend(self.check_body_consistency(
                source,
                def_node.def_keyword_loc().start_offset(),
                def_node.body(),
                false, // indented_internal_methods only applies to class/module bodies
            ));
            return;
        }

        if let Some(block_node) = node.as_block_node() {
            diagnostics.extend(self.check_body_consistency(
                source,
                block_node.opening_loc().start_offset(),
                block_node.body(),
                indented, // indented_internal_methods applies to block bodies too (class_methods do, etc.)
            ));
            return;
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(IndentationConsistency, "cops/layout/indentation_consistency");

    #[test]
    fn single_statement_body() {
        let source = b"def foo\n  x = 1\nend\n";
        let diags = run_cop_full(&IndentationConsistency, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn enforced_style_is_read() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("indented_internal_methods".into())),
            ]),
            ..CopConfig::default()
        };
        // In indented_internal_methods, methods in the same section before any
        // access modifier must be consistent. Two defs at different indentation
        // in the same section are flagged.
        let src = b"class Foo\n  def bar; end\n    def baz; end\nend\n";
        let diags = run_cop_full_with_config(&IndentationConsistency, src, config);
        assert!(!diags.is_empty(), "indented_internal_methods should still flag inconsistency within a section");
    }

    #[test]
    fn indented_internal_methods_allows_extra_indent_after_private() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("indented_internal_methods".into())),
            ]),
            ..CopConfig::default()
        };
        let src = b"class Foo\n  def bar\n  end\n\n  private\n\n    def baz\n    end\n\n    def qux\n    end\nend\n";
        let diags = run_cop_full_with_config(&IndentationConsistency, src, config);
        assert!(diags.is_empty(), "indented_internal_methods should allow extra indent after private: {:?}", diags);
    }

    #[test]
    fn indented_internal_methods_flags_inconsistency_within_private_section() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("indented_internal_methods".into())),
            ]),
            ..CopConfig::default()
        };
        // Two methods after private at different indentation levels
        let src = b"class Foo\n  private\n\n    def bar\n    end\n\n      def baz\n      end\nend\n";
        let diags = run_cop_full_with_config(&IndentationConsistency, src, config);
        assert!(!diags.is_empty(), "should flag inconsistency within private section");
    }
}
