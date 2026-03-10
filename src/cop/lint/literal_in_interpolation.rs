use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for interpolated literals in strings, symbols, regexps, and heredocs.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=40, FN=0 on the March 10, 2026 run.
///
/// The remaining FPs came from interpolated heredoc bodies such as
/// `"#{<<~TEXT}"`. RuboCop's `basic_literal?` check does not treat these heredoc
/// string nodes as plain printable literals for this cop, but the Prism port
/// treated all `StringNode` values alike and flagged them.
///
/// Fix: detect heredoc-backed `StringNode` / `InterpolatedStringNode` values by
/// their `<<` opening and exclude them from literal interpolation offenses.
///
/// Follow-up: `is_literal()` also needs to accept `KeywordHashNode`. These nodes
/// arise for Prism keyword-argument hashes (for example `call(foo: 1)`), and the
/// zero-tolerance `prism_pitfalls` test requires the literal helper to acknowledge
/// the hash/keyword-hash split even though interpolation bodies rarely surface
/// them directly.
///
/// RuboCop considers a node "literal" if it's a basic literal (int, float, string,
/// symbol, nil, true, false) or a composite literal (array, hash, pair/assoc, irange,
/// erange) where ALL children are also literals (recursively).
///
/// Special exclusions:
/// - `__FILE__`, `__LINE__`, `__ENCODING__` (SourceFileNode/SourceLineNode/SourceEncodingNode
///   in Prism — distinct from literal types, so naturally excluded)
/// - Whitespace-only string literals at the end of heredoc lines (deliberate
///   idiom for Layout/TrailingWhitespace preservation)
/// - Array literals inside regexps (handled by Lint/ArrayLiteralInRegexp)
/// - Literals in `%W[]`/`%I[]` whose expanded value contains spaces or is empty
///   (word splitting semantics differ)
///
/// Investigation findings (corpus 24 FP, 202 FN at 39.5%):
/// - FNs: missing range, array, hash composite literal support; missing multi-statement
///   handling (#{foo; 42}); overly broad whitespace exclusion (all contexts, not just
///   heredoc line endings); offense reported on #{} instead of on the literal node
/// - FPs: missing %W/%I percent literal exclusion; missing array-in-regexp exclusion
pub struct LiteralInInterpolation;

impl Cop for LiteralInInterpolation {
    fn name(&self) -> &'static str {
        "Lint/LiteralInInterpolation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = LiteralInterpVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_heredoc: false,
            in_array_percent_literal: false,
            in_regexp: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Recursively checks whether a Prism node is a "literal" in the RuboCop sense.
/// Basic literals: int, float, string, symbol, nil, true, false, rational, imaginary.
/// Composite literals: array (all elements literal), hash (all assoc key/values literal),
/// range (both endpoints literal).
fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    // Basic literals
    if node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_string_node().is_some()
    {
        return true;
    }

    // Composite: array with all-literal elements
    if let Some(array) = node.as_array_node() {
        return array.elements().iter().all(|e| is_literal(&e));
    }

    // Composite: hash with all-literal key/value pairs
    if let Some(hash) = node.as_hash_node() {
        return hash.elements().iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_literal(&assoc.key()) && is_literal(&assoc.value())
            } else {
                false
            }
        });
    }

    if let Some(hash) = node.as_keyword_hash_node() {
        return hash.elements().iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_literal(&assoc.key()) && is_literal(&assoc.value())
            } else {
                false
            }
        });
    }

    // Composite: range with literal endpoints
    if let Some(range) = node.as_range_node() {
        let left_ok = range.left().is_some_and(|l| is_literal(&l));
        let right_ok = range.right().is_some_and(|r| is_literal(&r));
        return left_ok && right_ok;
    }

    false
}

/// Check if a string node contains only whitespace (spaces/tabs) and is non-empty.
fn is_space_literal(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(str_node) = node.as_string_node() {
        let content = str_node.content_loc().as_slice();
        !content.is_empty() && content.iter().all(|&b| b == b' ' || b == b'\t')
    } else {
        false
    }
}

/// Check if an embedded statements node is at the end of a heredoc line.
fn ends_heredoc_line(
    source: &SourceFile,
    embedded: &ruby_prism::EmbeddedStatementsNode<'_>,
) -> bool {
    let end_offset = embedded.location().end_offset();
    let src = source.as_bytes();
    // At end of source or followed by newline means end of heredoc line
    end_offset >= src.len() || src[end_offset] == b'\n'
}

/// Check if the expanded value of a literal would contain whitespace or be empty.
/// Used for %W[] / %I[] exclusion where word splitting makes interpolation significant.
fn expanded_value_has_space_or_empty(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(str_node) = node.as_string_node() {
        let content = str_node.content_loc().as_slice();
        return content.is_empty() || content.iter().any(|&b| b == b' ' || b == b'\t');
    }
    if let Some(sym_node) = node.as_symbol_node() {
        let content = sym_node.unescaped();
        return content.is_empty() || content.iter().any(|&b| b == b' ' || b == b'\t');
    }
    if node.as_nil_node().is_some() {
        // nil.to_s is "", which is empty
        return true;
    }
    // For arrays, check recursively
    if let Some(array) = node.as_array_node() {
        return array.elements().is_empty()
            || array
                .elements()
                .iter()
                .any(|e| expanded_value_has_space_or_empty(&e));
    }
    false
}

fn is_heredoc_literal(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    let bytes = source.as_bytes();

    if let Some(str_node) = node.as_string_node() {
        return str_node
            .opening_loc()
            .is_some_and(|loc| bytes[loc.start_offset()..loc.end_offset()].starts_with(b"<<"));
    }

    if let Some(str_node) = node.as_interpolated_string_node() {
        return str_node
            .opening_loc()
            .is_some_and(|loc| bytes[loc.start_offset()..loc.end_offset()].starts_with(b"<<"));
    }

    false
}

struct LiteralInterpVisitor<'a, 'src> {
    cop: &'a LiteralInInterpolation,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    in_heredoc: bool,
    in_array_percent_literal: bool,
    in_regexp: bool,
}

impl<'a, 'src> LiteralInterpVisitor<'a, 'src> {
    fn check_embedded(&mut self, embedded: &ruby_prism::EmbeddedStatementsNode<'_>) {
        let stmts = match embedded.statements() {
            Some(s) => s,
            None => return,
        };

        let body: Vec<_> = stmts.body().iter().collect();
        // RuboCop checks `begin_node.children.last` — the final expression
        let final_node = match body.last() {
            Some(n) => n,
            None => return,
        };

        if is_heredoc_literal(self.source, final_node) {
            return;
        }

        if !is_literal(final_node) {
            return;
        }

        // Whitespace-only string at end of heredoc line — deliberate idiom
        if is_space_literal(final_node)
            && self.in_heredoc
            && ends_heredoc_line(self.source, embedded)
        {
            return;
        }

        // Array literals inside regexp — handled by Lint/ArrayLiteralInRegexp
        if self.in_regexp && final_node.as_array_node().is_some() {
            return;
        }

        // %W[] / %I[] exclusion: if the expanded value contains spaces or is empty,
        // the interpolation is semantically significant for word splitting
        if self.in_array_percent_literal && expanded_value_has_space_or_empty(final_node) {
            return;
        }

        let loc = final_node.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Literal interpolation detected.".to_string(),
        ));
    }
}

impl<'pr> Visit<'pr> for LiteralInterpVisitor<'_, '_> {
    fn visit_embedded_statements_node(&mut self, node: &ruby_prism::EmbeddedStatementsNode<'pr>) {
        self.check_embedded(node);
        // Don't recurse into embedded statements — nested interpolations in nested
        // strings will be visited when we visit those strings' own parts.
        // Calling the default visit here would recurse into the statements body
        // which we already inspected.
    }

    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        let was_heredoc = self.in_heredoc;
        if let Some(opening) = node.opening_loc() {
            let opening_slice = opening.as_slice();
            if opening_slice.starts_with(b"<<") {
                self.in_heredoc = true;
            }
        }

        ruby_prism::visit_interpolated_string_node(self, node);

        self.in_heredoc = was_heredoc;
    }

    fn visit_interpolated_regular_expression_node(
        &mut self,
        node: &ruby_prism::InterpolatedRegularExpressionNode<'pr>,
    ) {
        let was_regexp = self.in_regexp;
        self.in_regexp = true;

        ruby_prism::visit_interpolated_regular_expression_node(self, node);

        self.in_regexp = was_regexp;
    }

    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        let was_percent = self.in_array_percent_literal;
        if let Some(opening) = node.opening_loc() {
            let opening_slice = opening.as_slice();
            if opening_slice.starts_with(b"%W") || opening_slice.starts_with(b"%I") {
                self.in_array_percent_literal = true;
            }
        }

        ruby_prism::visit_array_node(self, node);

        self.in_array_percent_literal = was_percent;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LiteralInInterpolation, "cops/lint/literal_in_interpolation");

    #[test]
    fn keyword_hash_is_treated_as_literal() {
        let result = ruby_prism::parse(b"call(foo: 1)\n");
        let root = result.node();
        let program = root.as_program_node().unwrap();
        let stmts = program.statements();
        let call = stmts.body().iter().next().unwrap().as_call_node().unwrap();
        let arg = call.arguments().unwrap().arguments().iter().next().unwrap();

        assert!(arg.as_keyword_hash_node().is_some());
        assert!(is_literal(&arg));
    }
}
