use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use crate::cop::node_type::{IF_NODE, PARENTHESES_NODE, UNLESS_NODE};

pub struct IfUnlessModifier;

/// Check if a node (or any descendant) contains a heredoc.
/// Heredoc locations in Prism only cover the delimiter, so the actual
/// source spans more lines than the node location suggests.
fn node_contains_heredoc(node: &ruby_prism::Node<'_>) -> bool {
    let mut finder = HeredocFinder { found: false };
    finder.visit(node);
    finder.found
}

struct HeredocFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for HeredocFinder {
    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        if let Some(open) = node.opening_loc() {
            if open.as_slice().starts_with(b"<<") {
                self.found = true;
                return;
            }
        }
        ruby_prism::visit_interpolated_string_node(self, node);
    }

    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        if let Some(open) = node.opening_loc() {
            if open.as_slice().starts_with(b"<<") {
                self.found = true;
                return;
            }
        }
        ruby_prism::visit_string_node(self, node);
    }
}

/// Check if a node (or any descendant) contains a `defined?()` call.
///
/// RuboCop skips `if defined?(x)` when the argument is a local variable
/// or method call that might be undefined — converting to modifier form
/// changes the semantics of `defined?` with respect to local variable
/// scoping.  We conservatively skip any condition that contains `defined?`.
fn condition_contains_defined(node: &ruby_prism::Node<'_>) -> bool {
    let mut finder = DefinedFinder { found: false };
    finder.visit(node);
    finder.found
}

struct DefinedFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for DefinedFinder {
    fn visit_defined_node(&mut self, _node: &ruby_prism::DefinedNode<'pr>) {
        self.found = true;
    }
}

/// Check if a node (or any descendant) contains a local variable assignment (lvasgn).
///
/// RuboCop's `non_eligible_condition?` skips conditions that assign local
/// variables, because the modifier form may change scoping semantics.
fn condition_contains_lvasgn(node: &ruby_prism::Node<'_>) -> bool {
    let mut finder = LvasgnFinder { found: false };
    finder.visit(node);
    finder.found
}

struct LvasgnFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for LvasgnFinder {
    fn visit_local_variable_write_node(&mut self, _node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.found = true;
    }
}

/// Check if a node (or any descendant) contains a nested conditional
/// (if/unless/ternary). RuboCop's `nested_conditional?` on IfNode checks
/// whether any branch contains a nested `:if` node (which includes ternaries).
/// We check the body for any descendant IfNode or UnlessNode.
fn body_contains_nested_conditional(node: &ruby_prism::Node<'_>) -> bool {
    let mut finder = NestedConditionalFinder { found: false };
    finder.visit(node);
    finder.found
}

struct NestedConditionalFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for NestedConditionalFinder {
    fn visit_if_node(&mut self, _node: &ruby_prism::IfNode<'pr>) {
        self.found = true;
    }
    fn visit_unless_node(&mut self, _node: &ruby_prism::UnlessNode<'pr>) {
        self.found = true;
    }
}

impl Cop for IfUnlessModifier {
    fn name(&self) -> &'static str {
        "Style/IfUnlessModifier"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, PARENTHESES_NODE, UNLESS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Extract keyword location, predicate, statements, has_else, and keyword name
        // from either IfNode or UnlessNode
        let (kw_loc, predicate, statements, has_else, keyword) =
            if let Some(if_node) = node.as_if_node() {
                let kw_loc = match if_node.if_keyword_loc() {
                    Some(loc) => loc,
                    None => return, // ternary
                };
                // Skip elsif nodes — they are visited as IfNode but can't be
                // converted to modifier form independently
                if kw_loc.as_slice() == b"elsif" {
                    return;
                }
                (
                    kw_loc,
                    if_node.predicate(),
                    if_node.statements(),
                    if_node.subsequent().is_some(),
                    "if",
                )
            } else if let Some(unless_node) = node.as_unless_node() {
                (
                    unless_node.keyword_loc(),
                    unless_node.predicate(),
                    unless_node.statements(),
                    unless_node.else_clause().is_some(),
                    "unless",
                )
            } else {
                return;
            };

        // Must not have an else clause
        if has_else {
            return;
        }

        let body = match statements {
            Some(stmts) => stmts,
            None => return,
        };

        let body_stmts = body.body();

        // Must have exactly one statement
        if body_stmts.len() != 1 {
            return;
        }

        let body_node = match body_stmts.iter().next() {
            Some(n) => n,
            None => return,
        };

        // Check if already modifier form: keyword starts after body
        if kw_loc.start_offset() > body_node.location().start_offset() {
            return;
        }

        // Skip if the condition is a parenthesized assignment — these need the
        // full if/end form to capture the assignment value used in the body
        if predicate.as_parentheses_node().is_some() {
            return;
        }

        // Skip if the condition contains `defined?()` — converting to modifier
        // form changes semantics for undefined variables/methods.
        if condition_contains_defined(&predicate) {
            return;
        }

        // Skip if the condition contains a local variable assignment — modifier
        // form may change scoping semantics (RuboCop: non_eligible_condition?).
        if condition_contains_lvasgn(&predicate) {
            return;
        }

        // Skip if the body contains any nested conditional (if/unless/ternary).
        // RuboCop's `nested_conditional?` checks if any branch contains a nested
        // `:if` node, which includes ternaries (e.g., `a = x ? y : z`).
        if body_contains_nested_conditional(&body_node) {
            return;
        }

        // Body must be on a single line to be eligible for modifier form
        let (body_start_line, _) = source.offset_to_line_col(body_node.location().start_offset());
        let body_end_off = body_node.location().end_offset().saturating_sub(1).max(body_node.location().start_offset());
        let (body_end_line, _) = source.offset_to_line_col(body_end_off);
        if body_start_line != body_end_line {
            return;
        }

        // If there are comment lines between keyword and body, don't suggest modifier form.
        // Converting would lose the comments.
        let (kw_line, _) = source.offset_to_line_col(kw_loc.start_offset());
        if body_start_line > kw_line + 1 {
            return;
        }

        // Check if body contains a heredoc argument. Prism's node location for heredoc
        // references only covers the opening delimiter (<<~FOO), not the heredoc content.
        // The actual output would span more lines than the AST suggests.
        if node_contains_heredoc(&body_node) {
            return;
        }

        // Skip if body line has an EOL comment — converting to modifier would lose it
        {
            let lines: Vec<&[u8]> = source.lines().collect();
            if body_start_line > 0 && body_start_line <= lines.len() {
                let body_line = lines[body_start_line - 1];
                let body_end_in_line = body_node.location().end_offset();
                let (_, body_end_col) = source.offset_to_line_col(body_end_in_line);
                // Check if there's a comment after the body on the same line
                if body_end_col < body_line.len() {
                    let after_body = &body_line[body_end_col..];
                    let trimmed = after_body.iter().skip_while(|&&b| b == b' ' || b == b'\t').copied().collect::<Vec<_>>();
                    if trimmed.starts_with(b"#") {
                        return;
                    }
                }
            }
        }

        // Skip if there's a comment before `end` on its own line
        {
            let end_offset: Option<usize> = if let Some(if_node) = node.as_if_node() {
                if_node.end_keyword_loc().map(|loc| loc.start_offset())
            } else if let Some(unless_node) = node.as_unless_node() {
                unless_node.end_keyword_loc().map(|loc| loc.start_offset())
            } else {
                None
            };
            if let Some(end_off) = end_offset {
                let (end_line, _) = source.offset_to_line_col(end_off);
                if end_line > body_start_line + 1 {
                    // There are lines between body and end — check for comments
                    let lines: Vec<&[u8]> = source.lines().collect();
                    for line_num in (body_start_line + 1)..end_line {
                        if line_num > 0 && line_num <= lines.len() {
                            let line = lines[line_num - 1];
                            let trimmed: Vec<u8> = line.iter().skip_while(|&&b| b == b' ' || b == b'\t').copied().collect();
                            if trimmed.starts_with(b"#") {
                                return;
                            }
                        }
                    }
                }
            }
        }

        let max_line_length = config.get_usize("MaxLineLength", 120);
        // When MaxLineLength is 0, Layout/LineLength is disabled — skip line length check
        // (matches RuboCop's behavior: return true unless max_line_length)
        let line_length_enabled = config.get_bool("LineLengthEnabled", max_line_length > 0);

        // Estimate modifier line length: body + " " + keyword + " " + condition
        let body_text = &source.as_bytes()
            [body_node.location().start_offset()..body_node.location().end_offset()];
        let cond_text = &source.as_bytes()
            [predicate.location().start_offset()..predicate.location().end_offset()];

        // Include indentation in the modifier line length estimate.
        // The modifier form `body keyword condition` would be placed at the
        // indentation level of the original `if`/`unless` keyword, not at the
        // body's (deeper) indentation.
        let (_, kw_col) = source.offset_to_line_col(kw_loc.start_offset());

        // When the if/unless is used as the value of an assignment (e.g.,
        // `x = if cond; body; end`), RuboCop's `parenthesize?` wraps the modifier
        // form in parens: `x = (body if cond)`. This adds 2 chars to the line.
        // Check if the line before the keyword contains an assignment operator.
        let parens_overhead = {
            let kw_line_start = kw_loc.start_offset() - kw_col;
            let before_kw = &source.as_bytes()[kw_line_start..kw_loc.start_offset()];
            // Check if the content before keyword on the same line is just whitespace;
            // if not, it might contain assignment context. But the real case is when
            // the assignment is on the PREVIOUS line (multi-line assignment).
            // We check the previous non-blank line for a trailing `=`.
            let before_kw_trimmed = before_kw.iter().copied()
                .filter(|&b| b != b' ' && b != b'\t')
                .count();
            if before_kw_trimmed == 0 && kw_line_start > 0 {
                // Check the previous line for trailing `=`
                let lines: Vec<&[u8]> = source.lines().collect();
                let (kw_line_num, _) = source.offset_to_line_col(kw_loc.start_offset());
                if kw_line_num >= 2 {
                    let prev_line = lines[kw_line_num - 2];
                    let trimmed = prev_line.iter().copied()
                        .rev()
                        .skip_while(|&b| b == b' ' || b == b'\t')
                        .collect::<Vec<_>>();
                    if trimmed.first() == Some(&b'=') {
                        2 // add 2 for parentheses: "(" and ")"
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            }
        };

        let modifier_len = kw_col + parens_overhead + body_text.len() + 1 + keyword.len() + 1 + cond_text.len();

        if !line_length_enabled || modifier_len <= max_line_length {
            let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
            diagnostics.push(self.diagnostic(source, line, column, format!(
                "Favor modifier `{keyword}` usage when having a single-line body."
            )));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(IfUnlessModifier, "cops/style/if_unless_modifier");

    #[test]
    fn config_max_line_length() {
        use std::collections::HashMap;
        use crate::testutil::{run_cop_full_with_config, assert_cop_no_offenses_full_with_config};

        let config = CopConfig {
            options: HashMap::from([("MaxLineLength".into(), serde_yml::Value::Number(40.into()))]),
            ..CopConfig::default()
        };
        // Short body + condition fits in 40 chars as modifier => should suggest modifier
        let source = b"if x\n  y\nend\n";
        let diags = run_cop_full_with_config(&IfUnlessModifier, source, config.clone());
        assert!(!diags.is_empty(), "Should fire with MaxLineLength:40 on short if");

        // Longer body that would exceed 40 chars as modifier => should NOT suggest
        let source2 = b"if some_very_long_condition_variable_name\n  do_something_important_here\nend\n";
        assert_cop_no_offenses_full_with_config(&IfUnlessModifier, source2, config);
    }

    #[test]
    fn config_line_length_disabled() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        // When LineLengthEnabled is false (Layout/LineLength disabled),
        // modifier form should always be suggested regardless of line length.
        // This matches RuboCop behavior where `max_line_length` returns nil
        // when the cop is disabled.
        let config = CopConfig {
            options: HashMap::from([
                ("LineLengthEnabled".into(), serde_yml::Value::Bool(false)),
                ("MaxLineLength".into(), serde_yml::Value::Number(40.into())),
            ]),
            ..CopConfig::default()
        };
        // This body + condition would exceed 40 chars, but since line length is
        // disabled, it should still suggest modifier form.
        let source = b"if some_very_long_condition_variable_name\n  do_something_important_here\nend\n";
        let diags = run_cop_full_with_config(&IfUnlessModifier, source, config);
        assert!(!diags.is_empty(), "Should fire when LineLengthEnabled is false regardless of line length");
    }
}
