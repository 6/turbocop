use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/Documentation cop — checks for missing top-level documentation on classes/modules.
///
/// ## Investigation findings (2026-03-24)
///
/// **FP root cause:** `is_include_statement_only` only recursed into `StatementsNode`, not into
/// `SingletonClassNode` (`class << self`). RuboCop's `include_statement_only?` uses
/// `body.respond_to?(:children) && body.children.all? { ... }` which recurses into ANY node
/// with children. Classes like `class Cache; class << self; prepend Mixin; end; end` were
/// falsely flagged because nitrocop didn't recognize the singleton class body as include-only.
///
/// **FN root cause:** `is_include_extend_prepend` matched ANY `include`/`extend`/`prepend` call
/// regardless of argument type. RuboCop's pattern is `(send nil? {:include :extend :prepend} const)`
/// — the argument must be a constant reference. Calls like `include Dry::Types()` or
/// `include Foo.bar.baz` were incorrectly exempted, hiding modules/classes that should be flagged.
///
/// **Fix:** (1) Added `SingletonClassNode` recursion in `is_include_statement_only`.
/// (2) Restricted `is_include_extend_prepend` to require a single constant argument
/// (ConstantReadNode or ConstantPathNode), matching RuboCop's pattern.
///
/// ## Investigation findings (2026-04-01)
///
/// **FN root cause:** nitrocop treated `# Note: ...` as documentation because its annotation
/// keyword matching was case-sensitive, and it missed RuboCop's special handling for a lone
/// top-of-file Emacs-style magic comment like `# -*- encoding : utf-8 -*-` immediately above a
/// non-empty class or module.
///
/// **Fix:** reuse RuboCop-like case-insensitive annotation keyword matching and special-case the
/// line-2 top-of-file Emacs-style magic-comment pattern without suppressing files where that
/// comment is followed by another preceding comment line.
///
/// ## Investigation findings (2026-04-02)
///
/// **FN root cause:** nitrocop treated shebangs and several encoding magic-comment variants as
/// real documentation. That hid offenses for files like `#!/usr/bin/env ruby` on line 1,
/// `#coding : utf-8`, and wrapped forms like `# ~*~ encoding: utf-8 ~*~`.
///
/// **Fix:** treat shebangs and RuboCop-style interpreter directives as non-documentation comment
/// lines, including relaxed `coding`/`encoding` spacing and wrapped magic-comment variants.
///
/// ## Investigation findings (2026-04-02, empty singleton class)
///
/// **FN root cause:** the include-only exemption treated `class << self` with no body as
/// include-only because `is_include_statement_only` returned `true` for `SingletonClassNode`
/// without a body. RuboCop walks the singleton class children, so `self` plus an empty body does
/// not satisfy `include_statement_only?`.
///
/// **Fix:** keep recursing into non-empty singleton-class bodies, but stop exempting empty
/// `class << self` blocks. That restores offenses for classes like
/// `class Foo; class << self; end; end` without regressing `prepend/include`-only singleton
/// classes.
///
/// ## Investigation findings (2026-04-03)
///
/// **FN root cause:** nitrocop treated nearby comments and same-line `:nodoc:` markers as if they
/// were always attached to the current class/module node. RuboCop only counts comments actually
/// associated with the definition. That difference hid offenses for:
/// - inline nested definitions like `module Foo; class Bar`
/// - definitions closed with statement modifiers like `end unless defined?(Foo)`
/// - definitions inside `begin ... rescue`
/// - cbase definitions like `class ::Object #:nodoc:`
/// - while also over-reporting class expressions used as assignment values
///
/// **Fix:** only treat preceding comments as documentation when the definition starts the line,
/// is not inside a rescue-style `begin`, and its `end` line has no trailing code. Also stop
/// honoring same-line `:nodoc:` when the definition is inline after other code or starts with
/// `::`, and skip `class` definitions used as `=` assignment values, matching RuboCop's
/// comment association and statement-position behavior.
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
/// RuboCop uses `body.respond_to?(:children) && body.children.all? { ... }` which recurses
/// into any node with children, including `class << self` (singleton class) nodes.
fn is_include_statement_only(node: &ruby_prism::Node<'_>) -> bool {
    if is_include_extend_prepend(node) {
        return true;
    }
    if let Some(stmts) = node.as_statements_node() {
        if stmts.body().is_empty() {
            return false;
        }
        return stmts
            .body()
            .iter()
            .all(|child| is_include_statement_only(&child));
    }
    // Recurse into singleton class nodes (`class << self; prepend Foo; end`)
    // RuboCop's check walks into any node with children, so `class << self`
    // containing only include/extend/prepend is treated as include-only.
    if let Some(sclass) = node.as_singleton_class_node() {
        if let Some(ref body) = sclass.body() {
            return is_include_statement_only(body);
        }
        return false;
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

/// Check if a node is an include/extend/prepend call with a constant argument.
/// RuboCop's pattern is `(send nil? {:include :extend :prepend} const)` — the argument
/// must be a constant reference (e.g., `include Bar`), not a method call
/// (e.g., `include Dry::Types()` or `include Foo.bar`).
fn is_include_extend_prepend(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if !matches!(name, "include" | "extend" | "prepend") {
            return false;
        }
        // Must have no explicit receiver (implicit self / nil receiver)
        if call.receiver().is_some() {
            return false;
        }
        // Must have exactly one argument that is a constant reference
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() == 1 {
                let arg = &arg_list[0];
                // Accept ConstantReadNode (e.g., `Bar`) or ConstantPathNode (e.g., `Foo::Bar`)
                return arg.as_constant_read_node().is_some()
                    || arg.as_constant_path_node().is_some();
            }
        }
    }
    false
}

/// Check if the line containing the class/module keyword has a `:nodoc:` annotation.
/// Returns `(has_nodoc, has_nodoc_all)`.
fn check_nodoc(
    source: &SourceFile,
    keyword_offset: usize,
    allow_inline_nodoc: bool,
) -> (bool, bool) {
    if !allow_inline_nodoc {
        return (false, false);
    }

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

fn line_has_code_before_offset(source: &SourceFile, offset: usize) -> bool {
    let (line_num, column) = source.offset_to_line_col(offset);
    let lines: Vec<&[u8]> = source.lines().collect();
    lines.get(line_num - 1).is_some_and(|line| {
        line[..column.min(line.len())]
            .iter()
            .any(|b| !b.is_ascii_whitespace())
    })
}

fn line_has_code_after_offset(source: &SourceFile, offset: usize) -> bool {
    let (line_num, column) = source.offset_to_line_col(offset);
    let lines: Vec<&[u8]> = source.lines().collect();
    let Some(line) = lines.get(line_num - 1) else {
        return false;
    };

    let mut trailing = trim_bytes(&line[column.min(line.len())..]);
    while let Some(rest) = trailing.strip_prefix(b";") {
        trailing = trim_bytes(rest);
    }

    !trailing.is_empty() && !trailing.starts_with(b"#")
}

fn can_attach_preceding_comment(
    source: &SourceFile,
    keyword_offset: usize,
    end_offset: usize,
    inside_rescue_begin: bool,
) -> bool {
    !inside_rescue_begin
        && !line_has_code_before_offset(source, keyword_offset)
        && !line_has_code_after_offset(source, end_offset)
}

fn previous_significant_byte_on_line(source: &SourceFile, offset: usize) -> Option<u8> {
    let (line_num, column) = source.offset_to_line_col(offset);
    let lines: Vec<&[u8]> = source.lines().collect();
    let line = lines.get(line_num - 1)?;
    let prefix = &line[..column.min(line.len())];
    prefix
        .iter()
        .rev()
        .copied()
        .find(|b| !b.is_ascii_whitespace())
}

fn has_cbase_prefix(path: &ruby_prism::Node<'_>) -> bool {
    path.as_constant_path_node()
        .is_some_and(|cpath| cpath.location().as_slice().starts_with(b"::"))
}

fn has_documentation_comment_in_context(
    source: &SourceFile,
    keyword_offset: usize,
    allow_preceding_comment: bool,
) -> bool {
    if !allow_preceding_comment {
        return false;
    }

    let (node_line, _) = source.offset_to_line_col(keyword_offset);
    if node_line <= 1 {
        return false;
    }
    let lines: Vec<&[u8]> = source.lines().collect();

    // RuboCop still requires documentation for a non-empty class/module on line 2 when the only
    // preceding line is an Emacs-style magic comment like `# -*- encoding : utf-8 -*-`.
    if node_line == 2 {
        if let Some(line) = lines.first() {
            let trimmed = trim_bytes(line);
            if trimmed.starts_with(b"#") {
                let comment_text = std::str::from_utf8(trimmed).unwrap_or("");
                let text = comment_text.trim_start_matches('#').trim();
                if is_emacs_style_magic_comment(text) {
                    return false;
                }
            }
        }
    }

    // Walk backward from the line before the keyword.
    // RuboCop associates all preceding comments (even across blank lines) with the
    // node via `ast_with_comments`, then checks if ANY is real documentation. To
    // match this, when the block immediately above the keyword is all directives
    // (e.g., `# rubocop:disable ...`), we skip one blank line and continue looking
    // for real doc comments above it.
    let mut line_idx = node_line - 2; // 0-indexed previous line
    let mut found_doc_comment = false;
    let mut seen_any_comment = false;

    while let Some(line) = lines.get(line_idx) {
        let trimmed = trim_bytes(line);

        if trimmed.is_empty() {
            if found_doc_comment {
                break;
            }
            if seen_any_comment {
                // First block was all directives — skip blank and look above
                seen_any_comment = false;
                if line_idx == 0 {
                    break;
                }
                line_idx -= 1;
                continue;
            }
            break;
        }

        if !trimmed.starts_with(b"#") {
            // Non-comment, non-blank line — stop
            break;
        }

        // It's a comment line — check if it's a "real" documentation comment
        seen_any_comment = true;
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
pub(crate) fn is_annotation_or_directive(comment: &str) -> bool {
    let text = comment.trim_start_matches('#').trim();

    // Shebang / interpreter directive comments do not count as documentation.
    if text.starts_with('!') {
        return true;
    }

    if is_interpreter_directive_comment(text) {
        return true;
    }
    // RuboCop directives
    if text.starts_with("rubocop:") {
        return true;
    }

    if is_annotation_comment(text) {
        return true;
    }

    false
}

pub fn is_annotation_comment(text: &str) -> bool {
    const DEFAULT_ANNOTATION_KEYWORDS: &[&str] =
        &["TODO", "FIXME", "OPTIMIZE", "HACK", "REVIEW", "NOTE"];

    for keyword in DEFAULT_ANNOTATION_KEYWORDS {
        if !text
            .get(..keyword.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(keyword))
        {
            continue;
        }

        let keyword_text = &text[..keyword.len()];
        let mut rest = &text[keyword.len()..];

        if let Some(next_byte) = rest.as_bytes().first() {
            if next_byte.is_ascii_alphanumeric() || *next_byte == b'_' {
                continue;
            }
        }

        let has_colon = {
            let trimmed = rest.trim_start();
            if let Some(after_colon) = trimmed.strip_prefix(':') {
                rest = after_colon;
                true
            } else {
                false
            }
        };

        let has_space = if !rest.is_empty() && rest.as_bytes()[0].is_ascii_whitespace() {
            rest = rest.trim_start();
            true
        } else {
            false
        };

        let has_note = !rest.is_empty();

        if !has_colon && !has_space {
            continue;
        }

        if just_keyword_of_sentence(keyword_text, has_colon, has_space, has_note) {
            continue;
        }

        return true;
    }

    false
}

fn just_keyword_of_sentence(
    keyword_text: &str,
    has_colon: bool,
    has_space: bool,
    has_note: bool,
) -> bool {
    if has_colon || !has_space || !has_note {
        return false;
    }

    let mut chars = keyword_text.chars();
    match chars.next() {
        Some(c) if c.is_ascii_uppercase() => {}
        _ => return false,
    }

    chars.all(|c| c.is_ascii_lowercase())
}

fn is_interpreter_directive_comment(text: &str) -> bool {
    has_magic_comment_key(text, "frozen_string_literal")
        || has_magic_comment_key(text, "shareable_constant_value")
        || has_magic_comment_key(text, "warn_indent")
        || has_magic_comment_key(text, "coding")
        || has_magic_comment_key(text, "encoding")
}

fn is_emacs_style_magic_comment(text: &str) -> bool {
    wrapped_magic_comment_inner(text).is_some()
}

fn wrapped_magic_comment_inner(text: &str) -> Option<&str> {
    let text = text.trim();
    if text.starts_with("-*-") && text.ends_with("-*-") {
        return Some(
            text.trim_start_matches("-*-")
                .trim_end_matches("-*-")
                .trim(),
        );
    }
    if text.starts_with("~*~") && text.ends_with("~*~") {
        return Some(
            text.trim_start_matches("~*~")
                .trim_end_matches("~*~")
                .trim(),
        );
    }
    None
}

fn has_magic_comment_key(text: &str, key: &str) -> bool {
    text.strip_prefix(key)
        .is_some_and(|rest| rest.trim_start().starts_with(':'))
}

pub(crate) fn trim_bytes(line: &[u8]) -> &[u8] {
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
            rescue_begin_depth: 0,
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
    /// >0 while visiting the body of a `begin ... rescue/ensure/else` wrapper.
    rescue_begin_depth: usize,
}

impl<'pr> Visit<'pr> for DocumentationVisitor<'_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let name = extract_short_name(&node.constant_path());
        let kw_loc = node.class_keyword_loc();
        let start = kw_loc.start_offset();
        let assigned_definition =
            previous_significant_byte_on_line(self.source, start) == Some(b'=');
        let allow_inline_nodoc = !line_has_code_before_offset(self.source, start)
            && !has_cbase_prefix(&node.constant_path());
        let (has_nodoc, has_nodoc_all) = check_nodoc(self.source, start, allow_inline_nodoc);
        let allow_preceding_comment = can_attach_preceding_comment(
            self.source,
            start,
            node.end_keyword_loc().end_offset(),
            self.rescue_begin_depth > 0,
        );

        // Check documentation requirement (only if not inside a :nodoc: all parent)
        if self.nodoc_all_depth == 0
            && !self.allowed_constants.iter().any(|c| c == &name)
            && !assigned_definition
            && !has_nodoc
            && !is_namespace_only(&node.body(), true)
            && !is_include_only(&node.body())
            && !has_documentation_comment_in_context(self.source, start, allow_preceding_comment)
        {
            let (line, column) = self.source.offset_to_line_col(start);
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Missing top-level documentation comment for `class`.".to_string(),
            ));
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
        let allow_inline_nodoc = !line_has_code_before_offset(self.source, start)
            && !has_cbase_prefix(&node.constant_path());
        let (has_nodoc, has_nodoc_all) = check_nodoc(self.source, start, allow_inline_nodoc);
        let allow_preceding_comment = can_attach_preceding_comment(
            self.source,
            start,
            node.end_keyword_loc().end_offset(),
            self.rescue_begin_depth > 0,
        );

        // Check documentation requirement (only if not inside a :nodoc: all parent)
        if self.nodoc_all_depth == 0
            && !self.allowed_constants.iter().any(|c| c == &name)
            && !has_nodoc
            && !is_namespace_only(&node.body(), false)
            && !is_include_only(&node.body())
            && !has_documentation_comment_in_context(self.source, start, allow_preceding_comment)
        {
            let (line, column) = self.source.offset_to_line_col(start);
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Missing top-level documentation comment for `module`.".to_string(),
            ));
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

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        let masks_preceding_comments = node.rescue_clause().is_some()
            || node.else_clause().is_some()
            || node.ensure_clause().is_some();

        if masks_preceding_comments {
            self.rescue_begin_depth += 1;
            if let Some(statements) = node.statements() {
                self.visit_statements_node(&statements);
            }
            self.rescue_begin_depth -= 1;

            if let Some(rescue_clause) = node.rescue_clause() {
                self.visit_rescue_node(&rescue_clause);
            }
            if let Some(else_clause) = node.else_clause() {
                self.visit_else_node(&else_clause);
            }
            if let Some(ensure_clause) = node.ensure_clause() {
                self.visit_ensure_node(&ensure_clause);
            }
        } else {
            ruby_prism::visit_begin_node(self, node);
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
    fn shebang_not_documentation() {
        let source =
            b"#!/usr/bin/env ruby\nclass SnippetsExample\n  def say_hello(name)\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(diags.len(), 1, "Shebang should not count as documentation");
    }

    #[test]
    fn shebang_then_encoding_not_documentation() {
        let source = b"#!/bin/env ruby\n# encoding: utf-8\nclass CreateWkAccounting < ActiveRecord::Migration[4.2]\n  def change\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Shebang plus encoding comment should not count as documentation"
        );
    }

    #[test]
    fn coding_comment_with_space_before_colon_not_documentation() {
        let source =
            b"#coding : utf-8\nmodule NoticesHelper\n  def mobile?(call_number)\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "`#coding : utf-8` should not count as documentation"
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
    fn emacs_style_encoding_comment_not_documentation() {
        let source = b"# -*- encoding : utf-8 -*-\nclass Foo\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Emacs-style encoding magic comment is not documentation"
        );
    }

    #[test]
    fn emacs_style_encoding_comment_not_documentation_for_module() {
        let source = b"# -*- encoding : utf-8 -*-\nmodule Foo\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Emacs-style encoding magic comment is not documentation for modules either"
        );
    }

    #[test]
    fn emacs_style_comment_followed_by_other_comment_counts_as_documentation() {
        let source =
            b"# -*- encoding : utf-8 -*-\n#coding: utf-8\nclass Foo\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Emacs-style magic comments should still count as documentation when another comment line follows"
        );
    }

    #[test]
    fn tilde_wrapped_encoding_comment_not_documentation() {
        let source =
            b"# ~*~ encoding: utf-8 ~*~\nclass WikiFactory\n  def self.build\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Wrapped encoding magic comment should not count as documentation"
        );
    }

    #[test]
    fn note_comment_not_documentation() {
        let source =
            b"# Note: named Address2 to avoid conflicting with other samples if loaded together\nclass Foo\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Note comments should not count as documentation"
        );
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

    // FN: compact path class definitions like `class Foo::Bar` should be flagged
    #[test]
    fn compact_path_class_needs_docs() {
        let source = b"class Foo::Bar\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Compact path class should need documentation"
        );
    }

    // FN: compact path module definitions like `module A::B` should be flagged
    #[test]
    fn compact_path_module_needs_docs() {
        let source = b"module A::B\n  C = 1\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Compact path module should need documentation"
        );
    }

    // FN: compact path class with documentation should NOT be flagged
    #[test]
    fn compact_path_class_with_docs_no_offense() {
        let source = b"# Documented\nclass Foo::Bar\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Compact path class with documentation should not be flagged"
        );
    }

    // FN: compact path with nodoc should NOT be flagged
    #[test]
    fn compact_path_with_nodoc_no_offense() {
        let source = b"class A::B::Test #:nodoc:\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Compact path with :nodoc: should not be flagged"
        );
    }

    // FN: cbase class like `class ::MyClass` should be flagged
    #[test]
    fn cbase_class_needs_docs() {
        let source = b"class ::MyClass\n  def method\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Cbase class (::MyClass) should need documentation"
        );
    }

    #[test]
    fn inline_nested_class_does_not_inherit_outer_docs() {
        let source = b"# outer docs\nmodule Foo; class Bar\n  def method\n  end\nend; end\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Inline nested class should not inherit the outer module's docs"
        );
    }

    #[test]
    fn modifier_wrapped_module_comment_is_not_documentation() {
        let source = b"# real doc\nmodule UserVars\n  class << self\n    attr_accessor :autostart_scripts\n  end\n  self.autostart_scripts = []\nend unless defined?(UserVars)\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Definitions closed with an end modifier should still need docs"
        );
    }

    #[test]
    fn class_inside_rescue_begin_comment_is_not_documentation() {
        let source = b"begin\n  # comment\n  class Tester\n    def method\n    end\n  end\nrescue LoadError\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Comment inside begin/rescue should not document the class"
        );
    }

    #[test]
    fn cbase_nodoc_does_not_suppress() {
        let source =
            b"class ::Object #:nodoc:\n  def meta_class\n    class << self; self end\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Cbase definitions should not honor same-line :nodoc:"
        );
    }

    #[test]
    fn documented_class_inside_unless_block_no_offense() {
        let source = b"unless defined?(ScopedDocumented)\n  # Real doc\n  class ScopedDocumented\n    def method\n    end\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "A normal unless block should still allow documentation comments"
        );
    }

    #[test]
    fn assigned_class_expression_no_offense() {
        let source = b"describe Foo do\n  before do\n    # Namespace docs\n    module Testing; end\n    @memory_class = class Testing::MyMemory < Parent\n      self\n    end\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Class expressions used as assignment values should not require documentation"
        );
    }

    // FP: deeply nested module inside a method should still be flagged per RuboCop
    // (RuboCop fires on_module for all modules in the AST)
    #[test]
    fn nested_module_inside_namespace_with_nodoc() {
        // Module inside a :nodoc: parent (without all) should still need docs
        let source = b"module TestModule #:nodoc:\n  TEST = 20\n  class Test\n    def method\n    end\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Nested class inside :nodoc: (without all) parent should still need docs"
        );
    }

    // RuboCop: class inside documented module A with inline comment still needs docs
    #[test]
    fn class_inside_commented_module_needs_docs() {
        let source =
            b"module A # The A Module\n  class B\n    C = 1\n    def method\n    end\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Class B inside module A should still need documentation"
        );
    }

    // Empty class with compact path should not need docs (no body)
    #[test]
    fn compact_path_empty_class_no_offense() {
        let source = b"class Foo::Bar\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Empty compact path class should not need documentation"
        );
    }

    // Compact path namespace module should not need docs
    #[test]
    fn compact_path_namespace_module_no_offense() {
        let source = b"module A::B\n  class C; end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Compact path namespace module should not need documentation"
        );
    }

    // Deeply nested class with docs
    #[test]
    fn deeply_nested_class_with_docs_no_offense() {
        let source = b"module A::B\n  module C\n    # Documented\n    class D\n      def method\n      end\n    end\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        // A::B is namespace (contains only C module), C is namespace (contains only D class), D has docs
        assert!(
            diags.is_empty(),
            "All documented/namespace classes should not be flagged"
        );
    }

    // FP fix: class with `class << self; prepend Foo; end` should not need docs
    // (RuboCop's include_statement_only? recurses into singleton class)
    #[test]
    fn singleton_class_with_prepend_no_offense() {
        let source = b"class Cache\n  class << self\n    prepend Mixin\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Class with only class << self with prepend should not need docs"
        );
    }

    // FP fix: module with `class << self; include Foo; end` should not need docs
    #[test]
    fn module_singleton_class_with_include_no_offense() {
        let source =
            b"module Marshal\n  class << self\n    include MarshalAutoloader\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Module with only class << self with include should not need docs"
        );
    }

    #[test]
    fn empty_singleton_class_needs_docs() {
        let source = b"class Foo\n  class << self\n  end\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Empty class << self should not count as include-only"
        );
    }

    // FN fix: include with non-const argument should NOT exempt from docs
    // RuboCop pattern: (send nil? {:include :extend :prepend} const)
    #[test]
    fn include_with_method_call_arg_needs_docs() {
        let source = b"module Types\n  include Dry::Types()\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Module with include of method call (not const) should need docs"
        );
    }

    // FN fix: include with method chain argument should need docs
    #[test]
    fn include_with_method_chain_needs_docs() {
        let source =
            b"class Base\n  include ActionDispatch::Routing::RouteSet.new.url_helpers\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Class with include of method chain should need docs"
        );
    }

    // Include with simple constant argument should still exempt
    #[test]
    fn include_with_const_arg_no_offense() {
        let source = b"module Mixin\n  include Comparable\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Module with include of simple const should not need docs"
        );
    }

    // Include with constant path argument should still exempt
    #[test]
    fn include_with_const_path_arg_no_offense() {
        let source = b"module Mixin\n  include ActiveSupport::Concern\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert!(
            diags.is_empty(),
            "Module with include of constant path should not need docs"
        );
    }

    // Extend with method call should need docs
    #[test]
    fn extend_with_method_call_needs_docs() {
        let source = b"module Foo\n  extend Dry.Types\nend\n";
        let diags = run_cop_full(&Documentation, source);
        assert_eq!(
            diags.len(),
            1,
            "Module with extend of method call should need docs"
        );
    }
}
