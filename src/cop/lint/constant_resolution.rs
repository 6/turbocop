use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks that certain constants are fully qualified.
/// Disabled by default; useful for gems to avoid conflicts.
///
/// ## Investigation notes
/// - FN fix: `ConstantPathWriteNode` targets (e.g., `Foo::Bar = 42`) should NOT
///   suppress the root constant. RuboCop's `defined_module` only returns truthy
///   for `casgn` nodes where the RHS is `Class.new` or `Module.new`, not plain
///   assignments. So `Foo` in `Foo::Bar = 42` IS flagged by RuboCop.
/// - FP fix: In RuboCop's Parser AST, a single-statement class/module body makes
///   that statement a direct child of the class/module node. `defined_module`
///   returns truthy for the class node, so a bare constant as the sole body
///   expression (e.g., `class Foo; Bar; end`) is skipped by RuboCop.
/// - `ConstantWriteNode` with `Class.new`/`Module.new` RHS matches RuboCop's
///   `defined_module` for `casgn` — the target constant is a definition name.
/// - FP fix: `ConstantPathWriteNode` with `Class.new`/`Module.new` RHS — the
///   target path is treated as a module definition (e.g.,
///   `ProblemCheck::TestCheck = Class.new(ProblemCheck)`). `Struct.new` does NOT
///   trigger `defined_module` — the root constant IS still flagged.
/// - FN fix: Directive regex in `directives.rs` was not anchored, causing nested
///   comments like `#   # rubocop:disable all` (YARD doc examples) to suppress
///   offenses for entire files.
///
/// ## Corpus investigation (2026-03-23)
///
/// Corpus oracle reported FP=0, FN=1.
///
/// FN=1: github-linguist/linguist `samples/Ruby/rexpl:4` — a Ruby script without
/// the `.rb` extension. nitrocop discovers files by `.rb` extension, so this file
/// is never processed. This is a file discovery limitation, not a cop logic bug.
/// No code change needed for the cop itself.
///
/// - FN fix (3 FNs in thor encoding_other.thor): Non-UTF-8 files with encoding
///   magic comments (e.g., `# encoding: iso-8859-7`) were skipping `check_source`
///   and `check_node` because the linter gated these on `code_map` being `Some`
///   (which was `None` for non-UTF8 files). Fixed in `linter.rs` to always build
///   `CodeMap` and run all cop phases for files that pass the encoding check.
/// - FN (1 FN in linguist rexpl): File `samples/Ruby/rexpl` has no `.rb`
///   extension (shebang script). This is a file-discovery issue — nitrocop may not
///   process extensionless files — not a cop logic bug. Deferred.
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

    /// In RuboCop's Parser AST, a class/module body with a single statement
    /// has that statement as a direct child of the class/module node (no
    /// wrapping `begin` node). This means `node.parent.defined_module` returns
    /// truthy for that sole statement. We match this by marking a sole
    /// ConstantReadNode body as a def name range.
    fn mark_sole_body_constant(&mut self, body: Option<ruby_prism::Node<'_>>) -> bool {
        if let Some(body_node) = body {
            if let Some(stmts) = body_node.as_statements_node() {
                let body_stmts: Vec<_> = stmts.body().iter().collect();
                if body_stmts.len() == 1 {
                    if let Some(const_read) = body_stmts[0].as_constant_read_node() {
                        let loc = const_read.location();
                        self.def_name_ranges
                            .push(loc.start_offset()..loc.end_offset());
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Check if a node is a `Class.new` or `Module.new` call.
/// RuboCop's `defined_module` returns truthy for `casgn` where the RHS is one of
/// these, treating the target constant as a module definition name.
/// Note: `Struct.new` does NOT trigger `defined_module` — RuboCop flags the root.
fn is_class_or_module_new(node: &ruby_prism::Node<'_>) -> bool {
    let Some(call) = node.as_call_node() else {
        return false;
    };
    let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
    if method_name != "new" {
        return false;
    }
    let Some(receiver) = call.receiver() else {
        return false;
    };
    // Handle both `Class.new` (ConstantReadNode) and `::Class.new` (ConstantPathNode).
    let receiver_name = if let Some(const_read) = receiver.as_constant_read_node() {
        std::str::from_utf8(const_read.name().as_slice()).unwrap_or("")
    } else if let Some(const_path) = receiver.as_constant_path_node() {
        // `::Class` has parent=None and name=Class
        if const_path.parent().is_none() {
            const_path
                .name()
                .map(|n| std::str::from_utf8(n.as_slice()).unwrap_or(""))
                .unwrap_or("")
        } else {
            return false;
        }
    } else {
        return false;
    };
    matches!(receiver_name, "Class" | "Module")
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

        // In RuboCop's Parser AST, a single-statement class body makes the
        // statement a direct child of the class node (no wrapping `begin`).
        // So `node.parent.defined_module` returns truthy for that statement.
        // We match this by marking the sole body constant as a def name.
        let sole_body_marked = self.mark_sole_body_constant(node.body());

        ruby_prism::visit_class_node(self, node);

        if sole_body_marked {
            self.pop_def_name_range();
        }
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

        let sole_body_marked = self.mark_sole_body_constant(node.body());

        ruby_prism::visit_module_node(self, node);

        if sole_body_marked {
            self.pop_def_name_range();
        }
        if is_simple {
            self.pop_def_name_range();
        }
    }

    fn visit_constant_read_node(&mut self, node: &ruby_prism::ConstantReadNode<'pr>) {
        let loc = node.location();

        // Skip constants that are class/module definition names.
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
        // unqualified root constant (like Foo in Foo::Bar).
        ruby_prism::visit_constant_path_node(self, node);
    }

    fn visit_constant_path_write_node(&mut self, node: &ruby_prism::ConstantPathWriteNode<'pr>) {
        // RuboCop's `defined_module` returns truthy for `casgn` nodes where the
        // RHS is `Class.new` or `Module.new` — the target path is treated as a
        // module definition name. For plain assignments like `Foo::Bar = 42`,
        // `defined_module` returns nil, so the root constant `Foo` IS flagged.
        // Note: `Struct.new` does NOT trigger `defined_module` in RuboCop.
        let is_module_def = is_class_or_module_new(&node.value());
        if is_module_def {
            let target = node.target();
            let loc = target.location();
            self.def_name_ranges
                .push(loc.start_offset()..loc.end_offset());
        }

        ruby_prism::visit_constant_path_write_node(self, node);

        if is_module_def {
            self.pop_def_name_range();
        }
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
    fn constant_path_write_flags_root() {
        // `Config::Setting = 42` — the root `Config` should be flagged.
        let diags = crate::testutil::run_cop_full(&ConstantResolution, b"Config::Setting = 42\n");
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for Config, got: {:?}",
            diags
        );
    }

    #[test]
    fn constant_path_write_after_class() {
        // Ensure ConstantPathWriteNode works after class definitions
        let source = b"class MyService < Base::Service\nend\nConfig::Setting = 42\n";
        let diags = crate::testutil::run_cop_full(&ConstantResolution, source);
        // Should flag: Base (root of Base::Service) and Config (root of Config::Setting)
        assert_eq!(diags.len(), 2, "Expected 2 offenses, got: {:?}", diags);
    }

    #[test]
    fn single_statement_class_body_suppressed() {
        // `class Foo; Bar; end` — Bar is the sole body statement, suppressed.
        let diags = crate::testutil::run_cop_full(&ConstantResolution, b"class Foo\n  Bar\nend\n");
        assert_eq!(diags.len(), 0, "Expected 0 offenses, got: {:?}", diags);
    }

    #[test]
    fn constant_path_write_class_new_suppresses_target() {
        // `ProblemCheck::TestCheck = Class.new(ProblemCheck)` — target path is
        // a module definition; root of target should NOT be flagged.
        // But `Class` (receiver) and `ProblemCheck` (argument) ARE flagged.
        let diags = crate::testutil::run_cop_full(
            &ConstantResolution,
            b"ProblemCheck::TestCheck = Class.new(ProblemCheck)\n",
        );
        assert_eq!(
            diags.len(),
            2,
            "Expected 2 offenses (Class + ProblemCheck arg), got: {:?}",
            diags
        );
    }

    #[test]
    fn constant_path_write_module_new_suppresses_target() {
        // `Validators::Custom = Module.new` — the target is a module definition.
        // `Module` (receiver of .new) is still an unqualified constant and IS flagged.
        let diags = crate::testutil::run_cop_full(
            &ConstantResolution,
            b"Validators::Custom = Module.new\n",
        );
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for Module, got: {:?}",
            diags
        );
    }

    #[test]
    fn constant_path_write_struct_new_flags_root() {
        // `Parent::Child = Struct.new(:name)` — Struct.new does NOT trigger
        // defined_module in RuboCop. Both `Parent` and `Struct` are flagged.
        let diags = crate::testutil::run_cop_full(
            &ConstantResolution,
            b"Parent::Child = Struct.new(:name)\n",
        );
        assert_eq!(
            diags.len(),
            2,
            "Expected 2 offenses (Parent + Struct), got: {:?}",
            diags
        );
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
    fn non_utf8_source_with_encoding_comment() {
        // Non-UTF-8 files (e.g., ISO-8859-7) with encoding magic comments should
        // still have constants flagged. This exercises the cop logic; the linter-level
        // fix ensures check_source is called for such files.
        let mut source = b"# encoding: iso-8859-7\n".to_vec();
        source.extend_from_slice(b"SOME_STRING = \"data\"\n");
        source.extend_from_slice(b"puts SOME_STRING\n");
        let diags = crate::testutil::run_cop_full(&ConstantResolution, &source);
        // SOME_STRING at line 2 is a ConstantWriteNode (definition) — not flagged.
        // SOME_STRING at line 3 is a ConstantReadNode — flagged.
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for read of SOME_STRING, got: {:?}",
            diags
        );
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
