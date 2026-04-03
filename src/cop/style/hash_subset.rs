//! Shared infrastructure for Style/HashExcept and Style/HashSlice.
//!
//! Mirrors RuboCop's `HashSubset` mixin. Both cops detect `Hash#reject`,
//! `Hash#select`, and `Hash#filter` calls with block predicates that can be
//! replaced by `Hash#except` or `Hash#slice`. The only difference is which
//! combinations of (outer_method, predicate_method, negation) indicate
//! "remove these keys" (except) vs "keep these keys" (slice).

use crate::cop::Cop;
use crate::cop::shared::util;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Which hash subset operation to suggest.
#[derive(Clone, Copy)]
pub enum HashSubsetMode {
    Except,
    Slice,
}

impl HashSubsetMode {
    fn method_name(self) -> &'static str {
        match self {
            Self::Except => "except",
            Self::Slice => "slice",
        }
    }
}

/// Shared check_node implementation for both HashExcept and HashSlice.
pub fn check_hash_subset(
    cop: &dyn Cop,
    mode: HashSubsetMode,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let outer_call = match node.as_call_node() {
        Some(call) => call,
        None => return,
    };

    let outer_method = outer_call.name().as_slice();
    if outer_method != b"reject" && outer_method != b"select" && outer_method != b"filter" {
        return;
    }

    let block_node = match outer_call.block().and_then(|block| block.as_block_node()) {
        Some(block) => block,
        None => return,
    };

    let (key_name, value_name) = match block_param_names(&block_node) {
        Some(names) => names,
        None => return,
    };

    let expr = match block_body_expression(&block_node) {
        Some(expr) => expr,
        None => return,
    };

    let (predicate_call, negated) = match unwrap_negation(expr) {
        Some(parts) => parts,
        None => return,
    };

    if util::is_safe_navigation_call(&predicate_call) {
        return;
    }

    let result_arg = comparison_arg(mode, outer_method, &predicate_call, negated, key_name)
        .or_else(|| {
            membership_arg(
                mode,
                outer_method,
                &predicate_call,
                negated,
                key_name,
                value_name,
            )
        });

    let Some(result_arg) = result_arg else {
        return;
    };

    let loc = outer_call
        .message_loc()
        .unwrap_or_else(|| outer_call.location());
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    let method = mode.method_name();
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        format!(
            "Use `{method}({})` instead.",
            format_arg(source, &result_arg)
        ),
    ));
}

// ── Shared helpers ─────────────────────────────────────────────────────

fn block_param_names<'a>(block: &ruby_prism::BlockNode<'a>) -> Option<(&'a [u8], &'a [u8])> {
    let params = block.parameters()?.as_block_parameters_node()?;
    let parameters = params.parameters()?;
    let requireds: Vec<_> = parameters.requireds().iter().collect();
    if requireds.len() != 2 {
        return None;
    }

    let key_name = requireds[0].as_required_parameter_node()?.name().as_slice();
    let value_name = requireds[1].as_required_parameter_node()?.name().as_slice();
    Some((key_name, value_name))
}

fn block_body_expression<'a>(block: &ruby_prism::BlockNode<'a>) -> Option<ruby_prism::Node<'a>> {
    single_expression(block.body()?)
}

fn single_expression<'a>(node: ruby_prism::Node<'a>) -> Option<ruby_prism::Node<'a>> {
    if let Some(statements) = node.as_statements_node() {
        let mut body = statements.body().iter();
        let expr = body.next()?;
        if body.next().is_some() {
            return None;
        }
        Some(expr)
    } else {
        Some(node)
    }
}

fn unwrap_negation<'a>(expr: ruby_prism::Node<'a>) -> Option<(ruby_prism::CallNode<'a>, bool)> {
    let call = expr.as_call_node()?;
    if call.name().as_slice() == b"!" {
        let inner = call.receiver()?.as_call_node()?;
        Some((inner, true))
    } else {
        Some((call, false))
    }
}

fn binary_other_side<'a>(
    predicate_call: &ruby_prism::CallNode<'a>,
    key_name: &[u8],
) -> Option<ruby_prism::Node<'a>> {
    let receiver = predicate_call.receiver()?;
    let mut args = predicate_call.arguments()?.arguments().iter();
    let first_arg = args.next()?;
    if args.next().is_some() {
        return None;
    }

    if is_lvar_named(&receiver, key_name) {
        Some(first_arg)
    } else if is_lvar_named(&first_arg, key_name) {
        Some(receiver)
    } else {
        None
    }
}

fn include_collection<'a>(
    predicate_call: &ruby_prism::CallNode<'a>,
    key_name: &[u8],
    value_name: &[u8],
) -> Option<ruby_prism::Node<'a>> {
    let mut args = predicate_call.arguments()?.arguments().iter();
    let first_arg = args.next()?;
    if args.next().is_some() || !is_lvar_named(&first_arg, key_name) {
        return None;
    }

    let collection = predicate_call.receiver()?;
    if is_lvar_named(&collection, value_name) || is_range_like(&collection) {
        return None;
    }

    Some(collection)
}

fn in_collection<'a>(
    predicate_call: &ruby_prism::CallNode<'a>,
    key_name: &[u8],
    value_name: &[u8],
) -> Option<ruby_prism::Node<'a>> {
    let receiver = predicate_call.receiver()?;
    if !is_lvar_named(&receiver, key_name) {
        return None;
    }

    let mut args = predicate_call.arguments()?.arguments().iter();
    let collection = args.next()?;
    if args.next().is_some() {
        return None;
    }

    if is_lvar_named(&collection, value_name) || is_range_like(&collection) {
        return None;
    }

    Some(collection)
}

fn is_lvar_named(node: &ruby_prism::Node<'_>, expected: &[u8]) -> bool {
    node.as_local_variable_read_node()
        .is_some_and(|lvar| lvar.name().as_slice() == expected)
}

fn is_range_like(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_range_node().is_some() {
        return true;
    }

    let Some(parens) = node.as_parentheses_node() else {
        return false;
    };
    let Some(body) = parens.body() else {
        return false;
    };
    let Some(inner) = single_expression(body) else {
        return false;
    };

    inner.as_range_node().is_some()
}

// ── Semantic matching (the only thing that differs between cops) ───────

/// Check if a comparison predicate matches the target semantics.
///
/// For `except`: reject + (==|eql?) or (select|filter) + !=
/// For `slice`:  (select|filter) + (==|eql?) or reject + !=
///               plus negated inversions of these
fn comparison_arg<'a>(
    mode: HashSubsetMode,
    outer_method: &[u8],
    predicate_call: &ruby_prism::CallNode<'a>,
    negated: bool,
    key_name: &[u8],
) -> Option<ruby_prism::Node<'a>> {
    let predicate_method = predicate_call.name().as_slice();

    let matches_semantics = match mode {
        HashSubsetMode::Except => {
            !negated
                && ((outer_method == b"reject"
                    && (predicate_method == b"==" || predicate_method == b"eql?"))
                    || ((outer_method == b"select" || outer_method == b"filter")
                        && predicate_method == b"!="))
        }
        HashSubsetMode::Slice => {
            ((outer_method == b"select" || outer_method == b"filter")
                && ((!negated && (predicate_method == b"==" || predicate_method == b"eql?"))
                    || (negated && predicate_method == b"!=")))
                || (outer_method == b"reject"
                    && ((!negated && predicate_method == b"!=")
                        || (negated && (predicate_method == b"==" || predicate_method == b"eql?"))))
        }
    };
    if !matches_semantics {
        return None;
    }

    let arg = binary_other_side(predicate_call, key_name)?;
    if (predicate_method == b"==" || predicate_method == b"!=")
        && !(arg.as_symbol_node().is_some() || arg.as_string_node().is_some())
    {
        return None;
    }

    Some(arg)
}

/// Check if a membership predicate matches the target semantics.
///
/// For `except`: reject + !negated + (include?|in?) or (select|filter) + negated + (include?|in?)
/// For `slice`:  (select|filter) + !negated + (include?|in?) or reject + negated + (include?|in?)
///               plus exclude? handling (slice only)
fn membership_arg<'a>(
    mode: HashSubsetMode,
    outer_method: &[u8],
    predicate_call: &ruby_prism::CallNode<'a>,
    negated: bool,
    key_name: &[u8],
    value_name: &[u8],
) -> Option<ruby_prism::Node<'a>> {
    let predicate_method = predicate_call.name().as_slice();

    let matches_semantics = match mode {
        HashSubsetMode::Except => {
            (outer_method == b"reject"
                && !negated
                && (predicate_method == b"include?" || predicate_method == b"in?"))
                || ((outer_method == b"select" || outer_method == b"filter")
                    && negated
                    && (predicate_method == b"include?" || predicate_method == b"in?"))
        }
        HashSubsetMode::Slice => {
            ((outer_method == b"select" || outer_method == b"filter")
                && ((!negated && (predicate_method == b"include?" || predicate_method == b"in?"))
                    || (negated && predicate_method == b"exclude?")))
                || (outer_method == b"reject"
                    && ((negated
                        && (predicate_method == b"include?" || predicate_method == b"in?"))
                        || (!negated && predicate_method == b"exclude?")))
        }
    };
    if !matches_semantics {
        return None;
    }

    match predicate_method {
        b"include?" | b"exclude?" => include_collection(predicate_call, key_name, value_name),
        b"in?" => in_collection(predicate_call, key_name, value_name),
        _ => None,
    }
}

// ── Formatting helpers ─────────────────────────────────────────────────

fn format_arg(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    if let Some(array) = node.as_array_node() {
        return array
            .elements()
            .iter()
            .map(|element| format_array_element(source, &element))
            .collect::<Vec<_>>()
            .join(", ");
    }

    if is_literal(node) {
        return node_source(source, node);
    }

    format!("*{}", node_source(source, node))
}

fn format_array_element(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    let raw = node_source(source, node);

    if node.as_interpolated_symbol_node().is_some() {
        format!(":\"{}\"", raw)
    } else if node.as_interpolated_string_node().is_some() {
        format!("\"{}\"", raw)
    } else if node.as_symbol_node().is_some() && !raw.starts_with(':') {
        format!(":{}", raw)
    } else if node.as_string_node().is_some() && !raw.starts_with('"') && !raw.starts_with('\'') {
        format!("'{}'", raw)
    } else {
        raw
    }
}

fn node_source(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    let bytes = &source.as_bytes()[node.location().start_offset()..node.location().end_offset()];
    String::from_utf8_lossy(bytes).into_owned()
}

fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_interpolated_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_array_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
        || node.as_range_node().is_some()
}
