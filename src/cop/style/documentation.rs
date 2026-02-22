use ruby_prism::Visit;

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

/// Check if a class/module body is "namespace-only" — contains only other
/// class/module definitions, constant assignments, and constant visibility declarations.
/// RuboCop exempts these from the documentation requirement.
/// `is_class` distinguishes: empty classes don't need docs, but empty modules do.
fn is_namespace_only(body: &Option<ruby_prism::Node<'_>>, is_class: bool) -> bool {
    let body = match body {
        Some(b) => b,
        None => return is_class, // empty class = namespace-only; empty module = needs docs
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => {
            // Body is a single node (e.g., a begin block)
            return is_constant_declaration(body);
        }
    };
    stmts
        .body()
        .iter()
        .all(|node| is_constant_declaration(&node))
}

/// Check if a class/module body contains only include/extend/prepend statements.
/// RuboCop exempts these from the documentation requirement separately from namespace check.
fn is_include_only(body: &Option<ruby_prism::Node<'_>>) -> bool {
    let body = match body {
        Some(b) => b,
        None => return false,
    };
    is_include_statement_only(body)
}

/// Recursively check if a node (or group of statements) is entirely include/extend/prepend calls.
fn is_include_statement_only(node: &ruby_prism::Node<'_>) -> bool {
    if is_include_extend_prepend(node) {
        return true;
    }
    if let Some(stmts) = node.as_statements_node() {
        return stmts
            .body()
            .iter()
            .all(|child| is_include_statement_only(&child));
    }
    false
}

/// Check if a single statement is a constant definition (class, module, casgn)
/// or a constant visibility declaration (private_constant, public_constant).
fn is_constant_declaration(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_class_node().is_some()
        || node.as_module_node().is_some()
        || node.as_constant_write_node().is_some()
        || node.as_constant_path_write_node().is_some()
    {
        return true;
    }
    // private_constant/public_constant calls
    if let Some(call) = node.as_call_node() {
        let name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if matches!(name, "private_constant" | "public_constant") {
            return true;
        }
    }
    false
}

/// Check if a node is an include/extend/prepend call.
fn is_include_extend_prepend(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        return matches!(name, "include" | "extend" | "prepend");
    }
    false
}

/// Check if the line containing the class/module keyword has a `:nodoc:` annotation.
/// Returns `(has_nodoc, has_nodoc_all)`.
fn check_nodoc(source: &SourceFile, keyword_offset: usize) -> (bool, bool) {
    let (line_num, _) = source.offset_to_line_col(keyword_offset);
    let lines: Vec<&[u8]> = source.lines().collect();
    if let Some(line) = lines.get(line_num - 1) {
        let line_str = String::from_utf8_lossy(line);
        // Look for #:nodoc: or # :nodoc: (with optional spaces)
        if let Some(pos) = line_str.find("#") {
            let comment = &line_str[pos..];
            if comment.contains(":nodoc:") {
                let has_all = comment.contains(":nodoc: all") || comment.contains(":nodoc:all");
                return (true, has_all);
            }
        }
    }
    (false, false)
}

/// Check if the line before the class/module has a proper documentation comment.
/// A documentation comment is a `#` line that:
/// - Is not separated by a blank line
/// - Is not purely a magic/annotation/directive comment (unless followed by a real comment)
fn has_documentation_comment(source: &SourceFile, keyword_offset: usize) -> bool {
    let (node_line, _) = source.offset_to_line_col(keyword_offset);
    if node_line <= 1 {
        return false;
    }
    let lines: Vec<&[u8]> = source.lines().collect();

    // Walk backward from the line before the keyword
    let mut line_idx = node_line - 2; // 0-indexed previous line
    let mut found_doc_comment = false;

    loop {
        let line = match lines.get(line_idx) {
            Some(l) => l,
            None => break,
        };
        let trimmed = trim_bytes(line);

        if trimmed.is_empty() {
            // Blank line — any comments before this don't count as documentation
            break;
        }

        if !trimmed.starts_with(b"#") {
            // Non-comment, non-blank line — stop
            break;
        }

        // It's a comment line — check if it's a "real" documentation comment
        let comment_text = std::str::from_utf8(trimmed).unwrap_or("");
        if !is_annotation_or_directive(comment_text) {
            found_doc_comment = true;
        }

        if line_idx == 0 {
            break;
        }
        line_idx -= 1;
    }

    found_doc_comment
}

/// Check if a comment line is a magic/annotation/directive comment that doesn't count
/// as documentation. These include:
/// - `# frozen_string_literal: true`
/// - `# encoding: ...`
/// - `# rubocop:disable ...`
/// - `# TODO: ...`, `# FIXME: ...`, etc.
fn is_annotation_or_directive(comment: &str) -> bool {
    let text = comment.trim_start_matches('#').trim();

    // Magic comments
    if text.starts_with("frozen_string_literal:")
        || text.starts_with("encoding:")
        || text.starts_with("coding:")
        || text.starts_with("warn_indent:")
        || text.starts_with("shareable_constant_value:")
    {
        return true;
    }

    // RuboCop directives
    if text.starts_with("rubocop:") {
        return true;
    }

    // Annotation keywords (only if the WHOLE comment starts with one)
    let annotation_keywords = ["TODO", "FIXME", "OPTIMIZE", "HACK", "REVIEW", "NOTE"];
    for kw in &annotation_keywords {
        if text.starts_with(kw) {
            // Must be followed by : or whitespace or end of string
            let rest = &text[kw.len()..];
            if rest.is_empty() || rest.starts_with(':') || rest.starts_with(' ') {
                return true;
            }
        }
    }

    false
}

fn trim_bytes(line: &[u8]) -> &[u8] {
    let start = line
        .iter()
        .position(|&b| b != b' ' && b != b'\t')
        .unwrap_or(line.len());
    let end = line
        .iter()
        .rposition(|&b| b != b' ' && b != b'\t' && b != b'\r' && b != b'\n')
        .map_or(start, |e| e + 1);
    if end > start { &line[start..end] } else { &[] }
}

impl Cop for Documentation {
    fn name(&self) -> &'static str {
        "Style/Documentation"
    }

    fn default_exclude(&self) -> &'static [&'static str] {
        &["spec/**/*", "test/**/*"]
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
        let allowed_constants = config
            .get_string_array("AllowedConstants")
            .unwrap_or_default();

        let mut visitor = DocumentationVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            allowed_constants,
            nodoc_all_depth: 0,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct DocumentationVisitor<'a> {
    cop: &'a Documentation,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    allowed_constants: Vec<String>,
    /// Depth counter: >0 means we're inside a `:nodoc: all` parent
    nodoc_all_depth: usize,
}

impl<'pr> Visit<'pr> for DocumentationVisitor<'_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let name = extract_short_name(&node.constant_path());
        let kw_loc = node.class_keyword_loc();
        let start = kw_loc.start_offset();
        let (has_nodoc, has_nodoc_all) = check_nodoc(self.source, start);

        // Check documentation requirement (only if not inside a :nodoc: all parent)
        if self.nodoc_all_depth == 0 {
            if !self.allowed_constants.iter().any(|c| c == &name)
                && !has_nodoc
                && !is_namespace_only(&node.body(), true)
                && !is_include_only(&node.body())
                && !has_documentation_comment(self.source, start)
            {
                let (line, column) = self.source.offset_to_line_col(start);
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Missing top-level documentation comment for `class`.".to_string(),
                ));
            }
        }

        // Recurse into children, tracking nodoc_all depth
        if has_nodoc_all {
            self.nodoc_all_depth += 1;
        }
        ruby_prism::visit_class_node(self, node);
        if has_nodoc_all {
            self.nodoc_all_depth -= 1;
        }
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let name = extract_short_name(&node.constant_path());
        let kw_loc = node.module_keyword_loc();
        let start = kw_loc.start_offset();
        let (has_nodoc, has_nodoc_all) = check_nodoc(self.source, start);

        // Check documentation requirement (only if not inside a :nodoc: all parent)
        if self.nodoc_all_depth == 0 {
            if !self.allowed_constants.iter().any(|c| c == &name)
                && !has_nodoc
                && !is_namespace_only(&node.body(), false)
                && !is_include_only(&node.body())
                && !has_documentation_comment(self.source, start)
            {
                let (line, column) = self.source.offset_to_line_col(start);
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Missing top-level documentation comment for `module`.".to_string(),
                ));
            }
        }

        // Recurse into children, tracking nodoc_all depth
        if has_nodoc_all {
            self.nodoc_all_depth += 1;
        }
        ruby_prism::visit_module_node(self, node);
        if has_nodoc_all {
            self.nodoc_all_depth -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};

    crate::cop_fixture_tests!(Documentation, "cops/style/documentation");

    #[test]
    fn first_line_class_has_no_preceding_comment() {
        let source = b"class Foo\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("class"));
    }

    #[test]
    fn module_without_comment() {
        let source = b"module Bar\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("module"));
    }

    #[test]
    fn empty_class_no_offense() {
        let source = b"class Foo\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Empty class should not need documentation"
        );
    }

    #[test]
    fn empty_module_no_offense() {
        // RuboCop DOES flag empty modules (unlike empty classes)
        // See spec: "registers an offense for empty module without documentation"
        let source = b"module Foo\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Empty module should need documentation per RuboCop spec"
        );
    }

    #[test]
    fn namespace_module_no_offense() {
        let source = b"module Test\n  class A; end\n  class B; end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Namespace module should not need documentation"
        );
    }

    #[test]
    fn namespace_class_no_offense() {
        let source = b"class Test\n  class A; end\n  class B; end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Namespace class should not need documentation"
        );
    }

    #[test]
    fn namespace_with_constants_no_offense() {
        let source = b"class Test\n  A = Class.new\n  B = Class.new\n  D = 1\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Namespace class with constants should not need documentation"
        );
    }

    #[test]
    fn nodoc_suppresses() {
        let source = b"class Test #:nodoc:\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            ":nodoc: should suppress documentation requirement"
        );
    }

    #[test]
    fn nodoc_with_space() {
        let source = b"class Test # :nodoc:\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "# :nodoc: should suppress documentation requirement"
        );
    }

    #[test]
    fn nodoc_all_suppresses_inner_classes() {
        let source =
            b"module Outer #:nodoc: all\n  class Inner\n    def method\n    end\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            ":nodoc: all should suppress inner classes"
        );
    }

    #[test]
    fn nodoc_all_on_class_suppresses_inner() {
        let source =
            b"class Base # :nodoc: all\n  class Helper\n    def method\n    end\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            ":nodoc: all on class should suppress inner classes"
        );
    }

    #[test]
    fn nodoc_all_deeply_nested() {
        let source = b"module Top #:nodoc: all\n  module Mid\n    class Deep\n      def method\n      end\n    end\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            ":nodoc: all should propagate to deeply nested classes"
        );
    }

    #[test]
    fn include_only_module_no_offense() {
        let source = b"module Foo\n  include Bar\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Module with only include should not need documentation"
        );
    }

    #[test]
    fn extend_only_module_no_offense() {
        let source = b"module Foo\n  extend Bar\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Module with only extend should not need documentation"
        );
    }

    #[test]
    fn include_with_methods_needs_docs() {
        let source = b"module Foo\n  include Bar\n  def baz; end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Module with include AND methods should need documentation"
        );
    }

    #[test]
    fn frozen_string_literal_not_documentation() {
        let source = b"# frozen_string_literal: true\nclass Foo\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "frozen_string_literal comment is not documentation"
        );
    }

    #[test]
    fn annotation_not_documentation() {
        let source = b"# TODO: do something\nclass Foo\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(diags.len(), 1, "TODO annotation is not documentation");
    }

    #[test]
    fn comment_after_blank_line_not_documentation() {
        let source = b"# Copyright 2024\n\nclass Foo\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Comment separated by blank line is not documentation"
        );
    }

    #[test]
    fn annotation_followed_by_real_comment_is_documentation() {
        let source = b"# TODO: fix this\n# Class comment.\nclass Foo\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Annotation followed by real comment should count as documentation"
        );
    }

    #[test]
    fn rubocop_directive_not_documentation() {
        let source = b"# rubocop:disable Style/For\nclass Foo\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(diags.len(), 1, "rubocop directive is not documentation");
    }

    #[test]
    fn allowed_constants_exempts_class() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedConstants".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("ClassMethods".into())]),
            )]),
            ..CopConfig::default()
        };
        // ClassMethods should be exempt
        let source = b"module ClassMethods\n  def method\n  end\nend\n";
        let diags = run_cop_full_with_config(&Documentation, source, config);
        assert!(
            diags.is_empty(),
            "AllowedConstants should exempt ClassMethods"
        );
    }

    #[test]
    fn allowed_constants_does_not_exempt_other_names() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedConstants".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("ClassMethods".into())]),
            )]),
            ..CopConfig::default()
        };
        // Foo is NOT in AllowedConstants, should still be flagged
        let source = b"class Foo\n  def method\n  end\nend\n";
        let diags = run_cop_full_with_config(&Documentation, source, config);
        assert_eq!(
            diags.len(),
            1,
            "Non-allowed constant should still be flagged"
        );
    }

    #[test]
    fn private_constant_namespace_no_offense() {
        let source =
            b"module Namespace\n  class Private\n  end\n\n  private_constant :Private\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Module with classes and private_constant should not need documentation"
        );
    }
}
