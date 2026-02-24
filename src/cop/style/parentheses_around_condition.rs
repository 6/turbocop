use crate::cop::node_type::{IF_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ParenthesesAroundCondition;

/// Check if the content of a parenthesized node is a safe assignment (=).
/// RuboCop allows `if (a = b)` by default (AllowSafeAssignment: true).
fn is_safe_assignment(node: &ruby_prism::ParenthesesNode<'_>) -> bool {
    let inner = match get_single_inner_node(node) {
        Some(n) => n,
        None => return false,
    };
    is_assignment_node(&inner)
}

/// Get the single inner node from a parenthesized expression.
/// Returns None if parens are empty or contain multiple semicolon-separated statements.
fn get_single_inner_node<'a>(
    node: &'a ruby_prism::ParenthesesNode<'a>,
) -> Option<ruby_prism::Node<'a>> {
    let body = node.body()?;
    if let Some(stmts) = body.as_statements_node() {
        let stmts_body = stmts.body();
        if stmts_body.len() == 1 {
            return Some(stmts_body.iter().next().unwrap());
        }
        return None;
    }
    Some(body)
}

fn is_multiline_paren(source: &SourceFile, paren: &ruby_prism::ParenthesesNode<'_>) -> bool {
    let open_loc = paren.opening_loc();
    let close_loc = paren.closing_loc();
    let (open_line, _) = source.offset_to_line_col(open_loc.start_offset());
    let (close_line, _) = source.offset_to_line_col(close_loc.start_offset());
    open_line != close_line
}

fn is_assignment_node(node: &ruby_prism::Node<'_>) -> bool {
    node.as_local_variable_write_node().is_some()
        || node.as_instance_variable_write_node().is_some()
        || node.as_class_variable_write_node().is_some()
        || node.as_global_variable_write_node().is_some()
        || node.as_constant_write_node().is_some()
        || node.as_multi_write_node().is_some()
        || is_setter_call(node)
}

/// Check if a node is a setter method call (e.g., `obj.attr = value`, `obj[0] = value`).
/// RuboCop treats these as safe assignments in parenthesized conditions.
fn is_setter_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name();
        let s = name.as_slice();
        // Setter methods end with `=` but are not ==, !=, <=, >=, <=>
        s.len() >= 2
            && s.last() == Some(&b'=')
            && s != b"=="
            && s != b"!="
            && s != b"<="
            && s != b">="
            && s != b"<=>"
    } else {
        false
    }
}

/// RuboCop's `parens_required?` check: if a lowercase letter immediately precedes
/// the opening `(` or follows the closing `)`, the parens are considered required.
/// This handles `if(cond)` (no space) where the `(` looks like a method call.
fn parens_required(source: &SourceFile, paren: &ruby_prism::ParenthesesNode<'_>) -> bool {
    let src = source.as_bytes();
    let open_offset = paren.opening_loc().start_offset();
    let close_offset = paren.closing_loc().start_offset();

    // Check char before `(`
    if open_offset > 0 {
        let ch = src[open_offset - 1];
        if ch.is_ascii_lowercase() {
            return true;
        }
    }
    // Check char after `)`
    let after = close_offset + 1;
    if after < src.len() {
        let ch = src[after];
        if ch.is_ascii_lowercase() {
            return true;
        }
    }
    false
}

/// RuboCop's `modifier_op?`: the inner expression is a modifier if/unless/while/until
/// or a rescue modifier. These require parens to group correctly.
fn is_modifier_op(node: &ruby_prism::Node<'_>) -> bool {
    // Rescue modifier: `something rescue fallback`
    if node.as_rescue_modifier_node().is_some() {
        return true;
    }
    // Modifier if: in Prism, a modifier `x if cond` is still an IfNode,
    // but the node location starts before the keyword (at the body expression).
    if let Some(if_node) = node.as_if_node() {
        if let Some(kw_loc) = if_node.if_keyword_loc() {
            // Ternary is not a modifier
            if kw_loc.as_slice() == b"?" {
                return false;
            }
            // Modifier form: the overall node starts before the keyword
            return node.location().start_offset() != kw_loc.start_offset();
        }
    }
    if let Some(unless_node) = node.as_unless_node() {
        let kw_loc = unless_node.keyword_loc();
        return node.location().start_offset() != kw_loc.start_offset();
    }
    if let Some(while_node) = node.as_while_node() {
        // Modifier while: `x while cond` — node starts before keyword
        let kw_loc = while_node.keyword_loc();
        return node.location().start_offset() != kw_loc.start_offset();
    }
    if let Some(until_node) = node.as_until_node() {
        let kw_loc = until_node.keyword_loc();
        return node.location().start_offset() != kw_loc.start_offset();
    }
    false
}

/// Check if the parens contain semicolon-separated expressions (multiple statements).
/// RuboCop allows `if (foo; bar)` because the parens serve a grouping purpose.
fn has_semicolon_separated_expressions(
    source: &SourceFile,
    paren: &ruby_prism::ParenthesesNode<'_>,
) -> bool {
    let body = match paren.body() {
        Some(b) => b,
        None => return false,
    };
    if let Some(stmts) = body.as_statements_node() {
        if stmts.body().len() > 1 {
            // Multiple statements — check if separated by semicolons (not newlines only).
            // In the context of a condition, multiple statements in parens always
            // indicate semicolons since newlines in a condition don't start new statements
            // without semicolons (they'd be continuation).
            // Actually, we need to check for semicolons in the source between statements.
            let src = source.as_bytes();
            let open = paren.opening_loc().start_offset();
            let close = paren.closing_loc().start_offset();
            let inner = &src[open + 1..close];
            return inner.contains(&b';');
        }
    }
    false
}

/// Check if the parens are empty: `if ()`.
fn is_empty_parens(paren: &ruby_prism::ParenthesesNode<'_>) -> bool {
    paren.body().is_none()
}

/// For while/until: RuboCop's `require_parentheses?` allows parens when the condition
/// is a method call with a `do..end` block that has keywords.
/// E.g., `while (foo do end)` — removing parens would change parse semantics.
fn requires_parens_for_block(node: &ruby_prism::Node<'_>, is_while_until: bool) -> bool {
    if !is_while_until {
        return false;
    }
    // Check if the inner expression contains a call node with a do..end block
    // that uses keywords. In Prism, a `foo do end` inside parens would be
    // a CallNode whose block is a BlockNode with `do` keyword.
    if let Some(call) = node.as_call_node() {
        if let Some(block) = call.block() {
            if let Some(block_node) = block.as_block_node() {
                // Check for `do` keyword (as opposed to `{` brace block)
                if block_node.opening_loc().as_slice() == b"do" {
                    return true;
                }
            }
        }
    }
    false
}

/// Common check: should we skip flagging this parenthesized condition?
fn should_skip(
    source: &SourceFile,
    paren: &ruby_prism::ParenthesesNode<'_>,
    allow_safe_assignment: bool,
    allow_multiline: bool,
    is_while_until: bool,
) -> bool {
    // Empty parens: `if ()`
    if is_empty_parens(paren) {
        return true;
    }
    // No space between keyword and `(`: `if(cond)` / `while(cond)`
    if parens_required(source, paren) {
        return true;
    }
    // Semicolon-separated expressions: `if (foo; bar)`
    if has_semicolon_separated_expressions(source, paren) {
        return true;
    }
    // Safe assignment: `if (x = something)`
    if allow_safe_assignment && is_safe_assignment(paren) {
        return true;
    }
    // Multiline: `if (x &&\n  y)`
    if allow_multiline && is_multiline_paren(source, paren) {
        return true;
    }
    // Check inner node for modifier ops and block requirements
    if let Some(inner) = get_single_inner_node(paren) {
        // Modifier conditional or rescue inside parens: `if (x rescue nil)`
        if is_modifier_op(&inner) {
            return true;
        }
        // while/until with do..end block: `while (foo do end)`
        if requires_parens_for_block(&inner, is_while_until) {
            return true;
        }
    }
    false
}

impl Cop for ParenthesesAroundCondition {
    fn name(&self) -> &'static str {
        "Style/ParenthesesAroundCondition"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_safe_assignment = config.get_bool("AllowSafeAssignment", true);
        let allow_multiline = config.get_bool("AllowInMultilineConditions", false);

        if let Some(if_node) = node.as_if_node() {
            // Must have `if` keyword (not ternary)
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };

            if let Some(paren) = if_node.predicate().as_parentheses_node() {
                if should_skip(
                    source,
                    &paren,
                    allow_safe_assignment,
                    allow_multiline,
                    false,
                ) {
                    return;
                }
                let keyword = if kw_loc.as_slice() == b"unless" {
                    "unless"
                } else {
                    "if"
                };
                let open_loc = paren.opening_loc();
                let (line, column) = source.offset_to_line_col(open_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Don't use parentheses around the condition of an `{keyword}`."),
                ));
            }
        } else if let Some(unless_node) = node.as_unless_node() {
            if let Some(paren) = unless_node.predicate().as_parentheses_node() {
                if should_skip(
                    source,
                    &paren,
                    allow_safe_assignment,
                    allow_multiline,
                    false,
                ) {
                    return;
                }
                let open_loc = paren.opening_loc();
                let (line, column) = source.offset_to_line_col(open_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Don't use parentheses around the condition of an `unless`.".to_string(),
                ));
            }
        } else if let Some(while_node) = node.as_while_node() {
            if let Some(paren) = while_node.predicate().as_parentheses_node() {
                if should_skip(source, &paren, allow_safe_assignment, allow_multiline, true) {
                    return;
                }
                let open_loc = paren.opening_loc();
                let (line, column) = source.offset_to_line_col(open_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Don't use parentheses around the condition of a `while`.".to_string(),
                ));
            }
        } else if let Some(until_node) = node.as_until_node() {
            if let Some(paren) = until_node.predicate().as_parentheses_node() {
                if should_skip(source, &paren, allow_safe_assignment, allow_multiline, true) {
                    return;
                }
                let open_loc = paren.opening_loc();
                let (line, column) = source.offset_to_line_col(open_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Don't use parentheses around the condition of an `until`.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};

    crate::cop_fixture_tests!(
        ParenthesesAroundCondition,
        "cops/style/parentheses_around_condition"
    );

    #[test]
    fn allow_multiline_conditions() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowInMultilineConditions".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };
        // Multiline condition in parens should be allowed
        let source = b"if (x > 10 &&\n   y > 10)\n  puts 'hi'\nend\n";
        let diags = run_cop_full_with_config(&ParenthesesAroundCondition, source, config);
        assert!(
            diags.is_empty(),
            "Should allow multiline conditions in parens"
        );
    }

    #[test]
    fn still_flags_single_line_with_allow_multiline() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowInMultilineConditions".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };
        // Single-line parens should still be flagged
        let source = b"if (x > 10)\n  puts 'hi'\nend\n";
        let diags = run_cop_full_with_config(&ParenthesesAroundCondition, source, config);
        assert_eq!(diags.len(), 1, "Should still flag single-line parens");
    }

    #[test]
    fn flags_multiline_by_default() {
        // Multiline parens should be flagged with default config
        let source = b"if (x > 10 &&\n   y > 10)\n  puts 'hi'\nend\n";
        let diags = run_cop_full(&ParenthesesAroundCondition, source);
        assert_eq!(diags.len(), 1, "Should flag multiline parens by default");
    }
}
