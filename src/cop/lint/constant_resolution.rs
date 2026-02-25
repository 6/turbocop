use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks that certain constants are fully qualified.
/// Disabled by default; useful for gems to avoid conflicts.
pub struct ConstantResolution;

impl Cop for ConstantResolution {
    fn name(&self) -> &'static str {
        "Lint/ConstantResolution"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Check Only/Ignore config.
        // RuboCop uses `cop_config['Only'].blank?` which returns true for both
        // nil and []. So `Only: []` (the default) means "check everything", same
        // as not configuring Only at all. Only a non-empty list restricts checking.
        let only = config.get_string_array("Only").unwrap_or_default();
        let ignore = config.get_string_array("Ignore").unwrap_or_default();

        let mut visitor = ConstantResolutionVisitor {
            cop: self,
            source,
            only,
            ignore,
            def_name_ranges: Vec::new(),
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ConstantResolutionVisitor<'a, 'src> {
    cop: &'a ConstantResolution,
    source: &'src SourceFile,
    only: Vec<String>,
    ignore: Vec<String>,
    /// Byte ranges of constant_path() nodes from class/module definitions.
    /// Any ConstantReadNode falling within these ranges is a definition name
    /// and should not be flagged.
    def_name_ranges: Vec<std::ops::Range<usize>>,
    diagnostics: Vec<Diagnostic>,
}

impl ConstantResolutionVisitor<'_, '_> {
    fn is_in_def_name(&self, offset: usize) -> bool {
        self.def_name_ranges
            .iter()
            .any(|range| range.contains(&offset))
    }

    fn push_def_name_range(&mut self, node: &ruby_prism::Node<'_>) {
        let loc = node.location();
        self.def_name_ranges
            .push(loc.start_offset()..loc.end_offset());
    }

    fn pop_def_name_range(&mut self) {
        self.def_name_ranges.pop();
    }
}

impl<'pr> Visit<'pr> for ConstantResolutionVisitor<'_, '_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        // The constant_path() of a ClassNode is the class name being defined.
        // Only mark it when the constant_path() is a simple ConstantReadNode
        // (e.g. `class Foo`). When it's a ConstantPathNode (e.g. `class Foo::Bar`),
        // the inner ConstantReadNode `Foo` has a ConstantPathNode as its parent,
        // not the ClassNode, so RuboCop still flags it — we match that behavior
        // by NOT marking ConstantPathNode ranges.
        let cp = node.constant_path();
        let is_simple_name = cp.as_constant_read_node().is_some();
        if is_simple_name {
            self.push_def_name_range(&cp);
        }

        // RuboCop's `node.parent&.defined_module` returns truthy for ALL direct
        // children of a class/module node, not just the name. This means the
        // superclass constant in `class Foo < Bar` is also skipped. However, for
        // qualified superclasses like `class Foo < Bar::Baz`, the inner `Bar` has
        // a ConstantPathNode parent (not the ClassNode), so it IS flagged.
        // We match this by marking simple ConstantReadNode superclasses only.
        let is_simple_super = node
            .superclass()
            .is_some_and(|s| s.as_constant_read_node().is_some());
        if let (true, Some(sup)) = (is_simple_super, node.superclass()) {
            self.push_def_name_range(&sup);
        }

        ruby_prism::visit_class_node(self, node);

        if is_simple_super {
            self.pop_def_name_range();
        }
        if is_simple_name {
            self.pop_def_name_range();
        }
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let cp = node.constant_path();
        let is_simple = cp.as_constant_read_node().is_some();
        if is_simple {
            self.push_def_name_range(&cp);
        }
        ruby_prism::visit_module_node(self, node);
        if is_simple {
            self.pop_def_name_range();
        }
    }

    fn visit_constant_read_node(&mut self, node: &ruby_prism::ConstantReadNode<'pr>) {
        let loc = node.location();

        // Skip constants that are class/module definition names.
        // RuboCop checks `node.parent&.defined_module` which returns truthy
        // when the constant's immediate parent is a class/module node and this
        // constant is the defined name. For simple `class Foo`, the ConstantReadNode
        // is the direct constant_path() of the ClassNode.
        // For `class Foo::Bar`, the ConstantPathNode is the constant_path(), and
        // the inner Foo ConstantReadNode's parent is the ConstantPathNode (not the
        // ClassNode), so RuboCop DOES flag the Foo part. We match this by only
        // checking if the ConstantReadNode IS the direct constant_path() node.
        if self.is_in_def_name(loc.start_offset()) {
            return;
        }

        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");

        if !self.only.is_empty() && !self.only.contains(&name.to_string()) {
            return;
        }
        if self.ignore.contains(&name.to_string()) {
            return;
        }

        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Fully qualify this constant to avoid possibly ambiguous resolution.".to_string(),
        ));
    }

    fn visit_constant_path_node(&mut self, node: &ruby_prism::ConstantPathNode<'pr>) {
        // ConstantPathNode itself (e.g., Foo::Bar or ::Foo) is already qualified,
        // so we don't flag it. But we must visit its children in case there's an
        // unqualified root constant (like Foo in Foo::Bar — as_constant_path_node
        // parent holds a ConstantReadNode that should still be checked).
        ruby_prism::visit_constant_path_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_with_config, run_cop_full_with_config};
    use std::collections::HashMap;
    crate::cop_fixture_tests!(ConstantResolution, "cops/lint/constant_resolution");

    fn config_with_only(values: Vec<&str>) -> crate::cop::CopConfig {
        let mut options = HashMap::new();
        options.insert(
            "Only".to_string(),
            serde_yml::Value::Sequence(
                values
                    .into_iter()
                    .map(|s| serde_yml::Value::String(s.to_string()))
                    .collect(),
            ),
        );
        crate::cop::CopConfig {
            options,
            ..crate::cop::CopConfig::default()
        }
    }

    #[test]
    fn empty_only_flags_all_constants() {
        // RuboCop's `Only: []` (the default) uses `.blank?` which returns true
        // for empty arrays, so it flags ALL unqualified constants.
        let config = config_with_only(vec![]);
        let diags = run_cop_full_with_config(&ConstantResolution, b"Foo\nBar\n", config);
        assert_eq!(diags.len(), 2);
    }

    #[test]
    fn only_restricts_to_listed_constants() {
        let config = config_with_only(vec!["Foo"]);
        let diags = run_cop_full_with_config(&ConstantResolution, b"Foo\nBar\n", config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Fully qualify"));
    }

    #[test]
    fn only_with_no_match_produces_no_offenses() {
        let config = config_with_only(vec!["Baz"]);
        assert_cop_no_offenses_with_config(&ConstantResolution, b"Foo\nBar\n", config);
    }

    #[test]
    fn ignore_suppresses_listed_constants() {
        let mut options = HashMap::new();
        options.insert(
            "Ignore".to_string(),
            serde_yml::Value::Sequence(vec![serde_yml::Value::String("Foo".to_string())]),
        );
        let config = crate::cop::CopConfig {
            options,
            ..crate::cop::CopConfig::default()
        };
        let diags = run_cop_full_with_config(&ConstantResolution, b"Foo\nBar\n", config);
        assert_eq!(diags.len(), 1);
    }
}
