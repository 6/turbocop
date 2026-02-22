use crate::cop::node_type::{CALL_NODE, DEF_NODE, HASH_NODE, KEYWORD_HASH_NODE};
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ClosingParenthesisIndentation;

impl Cop for ClosingParenthesisIndentation {
    fn name(&self) -> &'static str {
        "Layout/ClosingParenthesisIndentation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, DEF_NODE, HASH_NODE, KEYWORD_HASH_NODE]
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
                    diagnostics.extend(check_parens(
                        source,
                        self,
                        open_loc,
                        close_loc,
                        call.arguments(),
                        config,
                    ));
                    return;
                }
            }
            return;
        }

        // Handle method definitions with parenthesized parameters
        if let Some(def_node) = node.as_def_node() {
            let params = def_node.parameters();
            if let Some(params) = params {
                let lparen = def_node.lparen_loc();
                let rparen = def_node.rparen_loc();
                if let (Some(open_loc), Some(close_loc)) = (lparen, rparen) {
                    diagnostics.extend(check_def_parens(
                        source, self, open_loc, close_loc, params, config,
                    ));
                    return;
                }
            }
            return;
        }
    }
}

fn check_parens(
    source: &SourceFile,
    cop: &ClosingParenthesisIndentation,
    open_loc: ruby_prism::Location<'_>,
    close_loc: ruby_prism::Location<'_>,
    arguments: Option<ruby_prism::ArgumentsNode<'_>>,
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
        None => return Vec::new(),
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
            Some(line) => util::indentation_of(line),
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

        let all_aligned = element_columns.iter().all(|&c| c == element_columns[0]);

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
                Some(line) => util::indentation_of(line),
                None => 0,
            };
            let first_arg_line_indent = match util::line_at(source, first_arg_line) {
                Some(line) => util::indentation_of(line),
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
    params: ruby_prism::ParametersNode<'_>,
    config: &CopConfig,
) -> Vec<Diagnostic> {
    let (open_line, _open_col) = source.offset_to_line_col(open_loc.start_offset());
    let (close_line, close_col) = source.offset_to_line_col(close_loc.start_offset());

    if !util::begins_its_line(source, close_loc.start_offset()) {
        return Vec::new();
    }

    if close_line == open_line {
        return Vec::new();
    }

    // Get first parameter
    let first_param = params
        .requireds()
        .iter()
        .next()
        .or_else(|| params.optionals().iter().next().map(ruby_prism::Node::from));

    let first_param = match first_param {
        Some(p) => p,
        None => return Vec::new(),
    };

    let (first_param_line, _) = source.offset_to_line_col(first_param.location().start_offset());

    let indent_width = config.get_usize("IndentationWidth", 2);

    if first_param_line > open_line {
        let first_param_line_indent = match util::line_at(source, first_param_line) {
            Some(line) => util::indentation_of(line),
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
