use crate::cop::node_type::{
    CALL_NODE, DEF_NODE, EMBEDDED_STATEMENTS_NODE, HASH_NODE, KEYWORD_HASH_NODE, PARENTHESES_NODE,
};
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Count leading whitespace characters (spaces and tabs) as columns.
/// Unlike `util::indentation_of()` which only counts spaces, this counts both
/// spaces and tabs as 1 column each, matching `offset_to_line_col()`'s character
/// counting and RuboCop's `processed_source.line_indentation()`.
fn leading_whitespace_columns(line: &[u8]) -> usize {
    line.iter()
        .take_while(|&&b| b == b' ' || b == b'\t')
        .count()
}

/// Corpus investigation (2026-03-16)
///
/// FP root cause #1 (5 FPs from loomio): Tab-indented code. `indentation_of()` only
/// counts spaces, returning 0 for tab-prefixed lines. `offset_to_line_col()` counts
/// tabs as 1 character each. This mismatch caused the cop to compute expected=0 for
/// tab-indented closing parens. Fix: use `leading_whitespace_columns()` which counts
/// both spaces and tabs, matching RuboCop's `line_indentation()`.
///
/// FP root cause #2 (2 FPs from puppetlabs/puppet): When the first argument is an
/// empty hash `{}`, expanding its children produces an empty `element_columns` vec.
/// Rust's `.all()` returns true (vacuously) on empty iterators, so the cop treated
/// it as "all aligned" and required `)` to align with `(`. But RuboCop's `[].uniq.one?`
/// returns false, going to the else branch (line indentation). Fix: check that
/// `element_columns` is non-empty before treating it as "all aligned".
///
/// FN root cause #3 (2026-03-30): grouped expressions whose first operand starts
/// on the same line as `(` were treated more permissively than RuboCop. The Prism
/// port accepted `)` at either the line indentation or the `(` column, but RuboCop
/// only accepts line indentation when the grouped body has multiple unaligned
/// child expressions. A single child expression, including heredoc bodies and
/// multiline conditions like `if ((foo) && ... )`, must align `)` with `(`.
///
/// FN root cause #4 (2026-03-30): multiline `#{...}` string interpolations were
/// skipped entirely. RuboCop reaches those through Parser `begin` nodes and applies
/// the same hanging-close rules, reporting on the closing `}` with the standard
/// `)` messages. Prism exposes them as `EmbeddedStatementsNode`, so the port must
/// check that node type explicitly and reuse the grouped-expression logic.
pub struct ClosingParenthesisIndentation;

impl Cop for ClosingParenthesisIndentation {
    fn name(&self) -> &'static str {
        "Layout/ClosingParenthesisIndentation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            DEF_NODE,
            HASH_NODE,
            KEYWORD_HASH_NODE,
            PARENTHESES_NODE,
            EMBEDDED_STATEMENTS_NODE,
        ]
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
        // Handle method calls with parentheses
        if let Some(call) = node.as_call_node() {
            if let (Some(open_loc), Some(close_loc)) = (call.opening_loc(), call.closing_loc()) {
                if close_loc.as_slice() == b")" {
                    let (_, node_col) = source.offset_to_line_col(node.location().start_offset());
                    diagnostics.extend(check_parens(
                        source,
                        self,
                        open_loc,
                        close_loc,
                        call.arguments(),
                        node_col,
                        config,
                    ));
                    return;
                }
            }
            return;
        }

        // Handle grouped expressions: (expr)
        // In Parser gem these are `begin` nodes; in Prism they are ParenthesesNode.
        if let Some(parens) = node.as_parentheses_node() {
            let open_loc = parens.opening_loc();
            let close_loc = parens.closing_loc();
            if close_loc.as_slice() == b")" {
                diagnostics.extend(check_grouped_body(
                    source,
                    self,
                    open_loc,
                    close_loc,
                    parenthesized_body_elements(parens.body()),
                    config,
                ));
            }
            return;
        }

        // Handle multiline string interpolation bodies: "#{...}"
        if let Some(embedded) = node.as_embedded_statements_node() {
            let open_loc = embedded.opening_loc();
            let close_loc = embedded.closing_loc();
            if close_loc.as_slice() == b"}" {
                diagnostics.extend(check_grouped_body(
                    source,
                    self,
                    open_loc,
                    close_loc,
                    embedded_body_elements(embedded.statements()),
                    config,
                ));
            }
            return;
        }

        // Handle method definitions with parenthesized parameters
        if let Some(def_node) = node.as_def_node() {
            let lparen = def_node.lparen_loc();
            let rparen = def_node.rparen_loc();
            if let (Some(open_loc), Some(close_loc)) = (lparen, rparen) {
                diagnostics.extend(check_def_parens(
                    source,
                    self,
                    open_loc,
                    close_loc,
                    def_node.parameters(),
                    config,
                ));
            }
        }
    }
}

fn check_parens(
    source: &SourceFile,
    cop: &ClosingParenthesisIndentation,
    open_loc: ruby_prism::Location<'_>,
    close_loc: ruby_prism::Location<'_>,
    arguments: Option<ruby_prism::ArgumentsNode<'_>>,
    node_col: usize,
    config: &CopConfig,
) -> Vec<Diagnostic> {
    let (open_line, open_col) = source.offset_to_line_col(open_loc.start_offset());
    let (close_line, close_col) = source.offset_to_line_col(close_loc.start_offset());

    // Closing paren must be on its own line (hanging)
    if !util::begins_its_line(source, close_loc.start_offset()) {
        return Vec::new();
    }

    // Must be multiline
    if close_line == open_line {
        return Vec::new();
    }

    let args = match arguments {
        Some(a) => a,
        None => {
            // No arguments: check_for_no_elements logic.
            // Acceptable columns: line indentation of open paren line, open paren column,
            // and the node column (for sends, same as open_line_indent typically).
            let open_line_indent = match util::line_at(source, open_line) {
                Some(line) => leading_whitespace_columns(line),
                None => 0,
            };
            if close_col != open_line_indent && close_col != open_col && close_col != node_col {
                return vec![cop.diagnostic(
                    source,
                    close_line,
                    close_col,
                    format!(
                        "Indent `)` to column {} (not {}).",
                        open_line_indent, close_col
                    ),
                )];
            }
            return Vec::new();
        }
    };

    let first_arg = match args.arguments().iter().next() {
        Some(a) => a,
        None => return Vec::new(),
    };

    let (first_arg_line, _first_arg_col) =
        source.offset_to_line_col(first_arg.location().start_offset());

    let indent_width = config.get_usize("IndentationWidth", 2);

    // Scenario 1: First param is on its own line (after the opening paren)
    if first_arg_line > open_line {
        let first_arg_line_indent = match util::line_at(source, first_arg_line) {
            Some(line) => leading_whitespace_columns(line),
            None => 0,
        };
        let expected = first_arg_line_indent.saturating_sub(indent_width);
        if close_col != expected {
            return vec![cop.diagnostic(
                source,
                close_line,
                close_col,
                format!("Indent `)` to column {} (not {}).", expected, close_col),
            )];
        }
    } else {
        // Scenario 2: First param is on same line as opening paren
        // When first element is a hash, check alignment of its children (pairs)
        let first_arg = args.arguments().iter().next().unwrap();
        let element_columns: Vec<usize> =
            if first_arg.as_keyword_hash_node().is_some() || first_arg.as_hash_node().is_some() {
                // Expand hash/keyword_hash into its pair columns
                let pairs: Vec<ruby_prism::Node<'_>> =
                    if let Some(kh) = first_arg.as_keyword_hash_node() {
                        kh.elements().iter().collect()
                    } else if let Some(h) = first_arg.as_hash_node() {
                        h.elements().iter().collect()
                    } else {
                        vec![]
                    };
                pairs
                    .iter()
                    .map(|p| {
                        let (_, col) = source.offset_to_line_col(p.location().start_offset());
                        col
                    })
                    .collect()
            } else {
                args.arguments()
                    .iter()
                    .map(|a| {
                        let (_, col) = source.offset_to_line_col(a.location().start_offset());
                        col
                    })
                    .collect()
            };

        let all_aligned =
            !element_columns.is_empty() && element_columns.iter().all(|&c| c == element_columns[0]);

        if all_aligned {
            // All args at same column: `)` aligns with `(`
            if close_col != open_col {
                return vec![cop.diagnostic(
                    source,
                    close_line,
                    close_col,
                    "Align `)` with `(`.".to_string(),
                )];
            }
        } else {
            // Args not aligned: accept first arg line indent or open line indent
            let open_line_indent = match util::line_at(source, open_line) {
                Some(line) => leading_whitespace_columns(line),
                None => 0,
            };
            let first_arg_line_indent = match util::line_at(source, first_arg_line) {
                Some(line) => leading_whitespace_columns(line),
                None => 0,
            };
            if close_col != first_arg_line_indent && close_col != open_line_indent {
                return vec![cop.diagnostic(
                    source,
                    close_line,
                    close_col,
                    format!(
                        "Indent `)` to column {} (not {}).",
                        open_line_indent, close_col
                    ),
                )];
            }
        }
    }

    Vec::new()
}

fn check_def_parens(
    source: &SourceFile,
    cop: &ClosingParenthesisIndentation,
    open_loc: ruby_prism::Location<'_>,
    close_loc: ruby_prism::Location<'_>,
    params: Option<ruby_prism::ParametersNode<'_>>,
    config: &CopConfig,
) -> Vec<Diagnostic> {
    let (open_line, open_col) = source.offset_to_line_col(open_loc.start_offset());
    let (close_line, close_col) = source.offset_to_line_col(close_loc.start_offset());

    if !util::begins_its_line(source, close_loc.start_offset()) {
        return Vec::new();
    }

    if close_line == open_line {
        return Vec::new();
    }

    let params = match params {
        Some(p) => p,
        None => {
            // No parameters: check_for_no_elements logic.
            // In RuboCop, on_def calls check(node.arguments, node.arguments).
            // For no-elements, candidates are [line_indentation, left_paren.column, node.loc.column].
            // For def params, node is the arguments node (the paren range), so
            // node.loc.column == open_col. Candidates collapse to [line_indent, open_col].
            let open_line_indent = match util::line_at(source, open_line) {
                Some(line) => leading_whitespace_columns(line),
                None => 0,
            };
            if close_col != open_line_indent && close_col != open_col {
                return vec![cop.diagnostic(
                    source,
                    close_line,
                    close_col,
                    format!(
                        "Indent `)` to column {} (not {}).",
                        open_line_indent, close_col
                    ),
                )];
            }
            return Vec::new();
        }
    };

    // Get first parameter - check all parameter types
    let first_param = params
        .requireds()
        .iter()
        .next()
        .or_else(|| params.optionals().iter().next())
        .or_else(|| params.posts().iter().next())
        .or_else(|| params.keywords().iter().next());

    // Also check rest, keyword_rest, and block params via their locations
    let first_param_offset = first_param.as_ref().map(|p| p.location().start_offset());

    // Check rest param
    let rest_offset = params.rest().map(|r| r.location().start_offset());
    let keyword_rest_offset = params.keyword_rest().map(|kr| kr.location().start_offset());
    let block_offset = params.block().map(|b| b.location().start_offset());

    // Find the earliest parameter offset
    let earliest_offset = [
        first_param_offset,
        rest_offset,
        keyword_rest_offset,
        block_offset,
    ]
    .into_iter()
    .flatten()
    .min();

    let earliest_offset = match earliest_offset {
        Some(o) => o,
        None => return Vec::new(),
    };

    let (first_param_line, _) = source.offset_to_line_col(earliest_offset);

    let indent_width = config.get_usize("IndentationWidth", 2);

    if first_param_line > open_line {
        // Scenario 1: First param on its own line after `(`
        let first_param_line_indent = match util::line_at(source, first_param_line) {
            Some(line) => leading_whitespace_columns(line),
            None => 0,
        };
        let expected = first_param_line_indent.saturating_sub(indent_width);
        if close_col != expected {
            return vec![cop.diagnostic(
                source,
                close_line,
                close_col,
                format!("Indent `)` to column {} (not {}).", expected, close_col),
            )];
        }
    } else {
        // Scenario 2: First param on same line as `(`
        // RuboCop uses expected_column which checks all_elements_aligned? and then
        // either aligns with `(` or uses line indentation.
        // For def params, the elements are the parameters themselves.
        let param_columns: Vec<usize> = collect_def_param_columns(source, &params);

        let all_aligned =
            !param_columns.is_empty() && param_columns.iter().all(|&c| c == param_columns[0]);

        if all_aligned {
            // All params at same column: `)` aligns with `(`
            if close_col != open_col {
                return vec![cop.diagnostic(
                    source,
                    close_line,
                    close_col,
                    "Align `)` with `(`.".to_string(),
                )];
            }
        } else {
            // Params not aligned: use line indentation of first param line
            let open_line_indent = match util::line_at(source, open_line) {
                Some(line) => leading_whitespace_columns(line),
                None => 0,
            };
            let first_param_line_indent = match util::line_at(source, first_param_line) {
                Some(line) => leading_whitespace_columns(line),
                None => 0,
            };
            if close_col != first_param_line_indent && close_col != open_line_indent {
                return vec![cop.diagnostic(
                    source,
                    close_line,
                    close_col,
                    format!(
                        "Indent `)` to column {} (not {}).",
                        open_line_indent, close_col
                    ),
                )];
            }
        }
    }

    Vec::new()
}

/// Collect column positions for all def parameters.
fn collect_def_param_columns(
    source: &SourceFile,
    params: &ruby_prism::ParametersNode<'_>,
) -> Vec<usize> {
    let mut columns = Vec::new();
    for p in params.requireds().iter() {
        let (_, col) = source.offset_to_line_col(p.location().start_offset());
        columns.push(col);
    }
    for p in params.optionals().iter() {
        let (_, col) = source.offset_to_line_col(p.location().start_offset());
        columns.push(col);
    }
    for p in params.posts().iter() {
        let (_, col) = source.offset_to_line_col(p.location().start_offset());
        columns.push(col);
    }
    for p in params.keywords().iter() {
        let (_, col) = source.offset_to_line_col(p.location().start_offset());
        columns.push(col);
    }
    if let Some(r) = params.rest() {
        let (_, col) = source.offset_to_line_col(r.location().start_offset());
        columns.push(col);
    }
    if let Some(kr) = params.keyword_rest() {
        let (_, col) = source.offset_to_line_col(kr.location().start_offset());
        columns.push(col);
    }
    if let Some(b) = params.block() {
        let (_, col) = source.offset_to_line_col(b.location().start_offset());
        columns.push(col);
    }
    columns
}

fn parenthesized_body_elements(body: Option<ruby_prism::Node<'_>>) -> Vec<ruby_prism::Node<'_>> {
    let Some(body) = body else {
        return Vec::new();
    };

    if let Some(stmts) = body.as_statements_node() {
        stmts.body().iter().collect()
    } else {
        vec![body]
    }
}

fn embedded_body_elements(
    statements: Option<ruby_prism::StatementsNode<'_>>,
) -> Vec<ruby_prism::Node<'_>> {
    let Some(statements) = statements else {
        return Vec::new();
    };

    statements.body().iter().collect()
}

/// Check hanging closing delimiter indentation for grouped-expression-like bodies.
/// RuboCop uses the same logic for `(expr)` and multiline `#{...}` interpolation
/// bodies via `on_begin`.
fn check_grouped_body(
    source: &SourceFile,
    cop: &ClosingParenthesisIndentation,
    open_loc: ruby_prism::Location<'_>,
    close_loc: ruby_prism::Location<'_>,
    elements: Vec<ruby_prism::Node<'_>>,
    config: &CopConfig,
) -> Vec<Diagnostic> {
    let (open_line, open_col) = source.offset_to_line_col(open_loc.start_offset());
    let (close_line, close_col) = source.offset_to_line_col(close_loc.start_offset());

    // Closing paren must be on its own line (hanging)
    if !util::begins_its_line(source, close_loc.start_offset()) {
        return Vec::new();
    }

    // Must be multiline
    if close_line == open_line {
        return Vec::new();
    }

    if elements.is_empty() {
        // Empty delimiters: accept either the opening-line indentation or the opener column.
        let open_line_indent = match util::line_at(source, open_line) {
            Some(line) => leading_whitespace_columns(line),
            None => 0,
        };
        if close_col != open_col && close_col != open_line_indent {
            return vec![cop.diagnostic(
                source,
                close_line,
                close_col,
                format!(
                    "Indent `)` to column {} (not {}).",
                    open_line_indent, close_col
                ),
            )];
        }
        return Vec::new();
    }

    let first_element = &elements[0];
    let (first_elem_line, _) = source.offset_to_line_col(first_element.location().start_offset());

    let indent_width = config.get_usize("IndentationWidth", 2);

    if first_elem_line > open_line {
        // Scenario 1: First element on its own line after `(`
        let first_elem_line_indent = match util::line_at(source, first_elem_line) {
            Some(line) => leading_whitespace_columns(line),
            None => 0,
        };
        let expected = first_elem_line_indent.saturating_sub(indent_width);
        if close_col != expected {
            return vec![cop.diagnostic(
                source,
                close_line,
                close_col,
                format!("Indent `)` to column {} (not {}).", expected, close_col),
            )];
        }
    } else {
        // Scenario 2: First element on same line as `(`
        let element_columns: Vec<usize> = elements
            .iter()
            .map(|element| {
                let (_, col) = source.offset_to_line_col(element.location().start_offset());
                col
            })
            .collect();
        let all_aligned =
            !element_columns.is_empty() && element_columns.iter().all(|&c| c == element_columns[0]);

        if all_aligned {
            if close_col != open_col {
                return vec![cop.diagnostic(
                    source,
                    close_line,
                    close_col,
                    "Align `)` with `(`.".to_string(),
                )];
            }
        } else {
            let first_elem_line_indent = match util::line_at(source, first_elem_line) {
                Some(line) => leading_whitespace_columns(line),
                None => 0,
            };
            if close_col != first_elem_line_indent {
                return vec![cop.diagnostic(
                    source,
                    close_line,
                    close_col,
                    format!(
                        "Indent `)` to column {} (not {}).",
                        first_elem_line_indent, close_col
                    ),
                )];
            }
        }
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        ClosingParenthesisIndentation,
        "cops/layout/closing_parenthesis_indentation"
    );
}
