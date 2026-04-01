use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Detects `Hash#reject`, `Hash#select`, and `Hash#filter` calls that can be
/// replaced with `Hash#slice`.
///
/// 2026-04 corpus fix:
/// - added missed implicit-receiver `select`/`reject` calls inside hash helpers
/// - handled negated `include?`, `in?`, and `exclude?` membership predicates
/// - supported `eql?` and array-literal formatting for `slice(:a, :b)`
/// - kept the fix narrow by skipping range-backed membership checks and
///   safe-navigation predicates like `cached_methods_params&.include?(key)`,
///   which RuboCop does not flag
pub struct HashSlice;

impl Cop for HashSlice {
    fn name(&self) -> &'static str {
        "Style/HashSlice"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
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

        let (key_name, value_name) = match self.block_param_names(&block_node) {
            Some(names) => names,
            None => return,
        };

        let expr = match self.block_body_expression(&block_node) {
            Some(expr) => expr,
            None => return,
        };

        let (predicate_call, negated) = match self.unwrap_negation(expr) {
            Some(parts) => parts,
            None => return,
        };

        if self.is_safe_navigation_call(&predicate_call) {
            return;
        }

        let slice_arg = self
            .comparison_slice_arg(outer_method, &predicate_call, negated, key_name)
            .or_else(|| {
                self.membership_slice_arg(
                    outer_method,
                    &predicate_call,
                    negated,
                    key_name,
                    value_name,
                )
            });

        let Some(slice_arg) = slice_arg else {
            return;
        };

        let loc = outer_call
            .message_loc()
            .unwrap_or_else(|| outer_call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `slice({})` instead.",
                self.format_slice_arg(source, &slice_arg)
            ),
        ));
    }
}

impl HashSlice {
    fn block_param_names<'a>(
        &self,
        block: &ruby_prism::BlockNode<'a>,
    ) -> Option<(&'a [u8], &'a [u8])> {
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

    fn block_body_expression<'a>(
        &self,
        block: &ruby_prism::BlockNode<'a>,
    ) -> Option<ruby_prism::Node<'a>> {
        self.single_expression(block.body()?)
    }

    fn single_expression<'a>(&self, node: ruby_prism::Node<'a>) -> Option<ruby_prism::Node<'a>> {
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

    fn unwrap_negation<'a>(
        &self,
        expr: ruby_prism::Node<'a>,
    ) -> Option<(ruby_prism::CallNode<'a>, bool)> {
        let call = expr.as_call_node()?;
        if call.name().as_slice() == b"!" {
            let inner = call.receiver()?.as_call_node()?;
            Some((inner, true))
        } else {
            Some((call, false))
        }
    }

    fn comparison_slice_arg<'a>(
        &self,
        outer_method: &[u8],
        predicate_call: &ruby_prism::CallNode<'a>,
        negated: bool,
        key_name: &[u8],
    ) -> Option<ruby_prism::Node<'a>> {
        let predicate_method = predicate_call.name().as_slice();
        let matches_semantics = ((outer_method == b"select" || outer_method == b"filter")
            && ((!negated && (predicate_method == b"==" || predicate_method == b"eql?"))
                || (negated && predicate_method == b"!=")))
            || (outer_method == b"reject"
                && ((!negated && predicate_method == b"!=")
                    || (negated && (predicate_method == b"==" || predicate_method == b"eql?"))));
        if !matches_semantics {
            return None;
        }

        let slice_arg = self.binary_other_side(predicate_call, key_name)?;
        if (predicate_method == b"==" || predicate_method == b"!=")
            && !(slice_arg.as_symbol_node().is_some() || slice_arg.as_string_node().is_some())
        {
            return None;
        }

        Some(slice_arg)
    }

    fn membership_slice_arg<'a>(
        &self,
        outer_method: &[u8],
        predicate_call: &ruby_prism::CallNode<'a>,
        negated: bool,
        key_name: &[u8],
        value_name: &[u8],
    ) -> Option<ruby_prism::Node<'a>> {
        let predicate_method = predicate_call.name().as_slice();
        let matches_semantics = ((outer_method == b"select" || outer_method == b"filter")
            && ((!negated && (predicate_method == b"include?" || predicate_method == b"in?"))
                || (negated && predicate_method == b"exclude?")))
            || (outer_method == b"reject"
                && ((negated && (predicate_method == b"include?" || predicate_method == b"in?"))
                    || (!negated && predicate_method == b"exclude?")));
        if !matches_semantics {
            return None;
        }

        match predicate_method {
            b"include?" | b"exclude?" => {
                self.include_collection(predicate_call, key_name, value_name)
            }
            b"in?" => self.in_collection(predicate_call, key_name, value_name),
            _ => None,
        }
    }

    fn binary_other_side<'a>(
        &self,
        predicate_call: &ruby_prism::CallNode<'a>,
        key_name: &[u8],
    ) -> Option<ruby_prism::Node<'a>> {
        let receiver = predicate_call.receiver()?;
        let mut args = predicate_call.arguments()?.arguments().iter();
        let first_arg = args.next()?;
        if args.next().is_some() {
            return None;
        }

        if self.is_lvar_named(&receiver, key_name) {
            Some(first_arg)
        } else if self.is_lvar_named(&first_arg, key_name) {
            Some(receiver)
        } else {
            None
        }
    }

    fn include_collection<'a>(
        &self,
        predicate_call: &ruby_prism::CallNode<'a>,
        key_name: &[u8],
        value_name: &[u8],
    ) -> Option<ruby_prism::Node<'a>> {
        let mut args = predicate_call.arguments()?.arguments().iter();
        let first_arg = args.next()?;
        if args.next().is_some() || !self.is_lvar_named(&first_arg, key_name) {
            return None;
        }

        let collection = predicate_call.receiver()?;
        if self.is_lvar_named(&collection, value_name) || self.is_range_like(&collection) {
            return None;
        }

        Some(collection)
    }

    fn in_collection<'a>(
        &self,
        predicate_call: &ruby_prism::CallNode<'a>,
        key_name: &[u8],
        value_name: &[u8],
    ) -> Option<ruby_prism::Node<'a>> {
        let receiver = predicate_call.receiver()?;
        if !self.is_lvar_named(&receiver, key_name) {
            return None;
        }

        let mut args = predicate_call.arguments()?.arguments().iter();
        let collection = args.next()?;
        if args.next().is_some() {
            return None;
        }

        if self.is_lvar_named(&collection, value_name) || self.is_range_like(&collection) {
            return None;
        }

        Some(collection)
    }

    fn is_lvar_named(&self, node: &ruby_prism::Node<'_>, expected: &[u8]) -> bool {
        node.as_local_variable_read_node()
            .is_some_and(|lvar| lvar.name().as_slice() == expected)
    }

    fn is_safe_navigation_call(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        call.call_operator_loc()
            .is_some_and(|op| op.as_slice() == b"&.")
    }

    fn is_range_like(&self, node: &ruby_prism::Node<'_>) -> bool {
        if node.as_range_node().is_some() {
            return true;
        }

        let Some(parens) = node.as_parentheses_node() else {
            return false;
        };
        let Some(body) = parens.body() else {
            return false;
        };
        let Some(inner) = self.single_expression(body) else {
            return false;
        };

        inner.as_range_node().is_some()
    }

    fn format_slice_arg(&self, source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
        if let Some(array) = node.as_array_node() {
            return array
                .elements()
                .iter()
                .map(|element| self.format_array_element(source, &element))
                .collect::<Vec<_>>()
                .join(", ");
        }

        if self.is_literal(node) {
            return self.node_source(source, node);
        }

        format!("*{}", self.node_source(source, node))
    }

    fn format_array_element(&self, source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
        let raw = self.node_source(source, node);

        if node.as_interpolated_symbol_node().is_some() {
            format!(":\"{}\"", raw)
        } else if node.as_interpolated_string_node().is_some() {
            format!("\"{}\"", raw)
        } else if node.as_symbol_node().is_some() && !raw.starts_with(':') {
            format!(":{}", raw)
        } else if node.as_string_node().is_some() && !raw.starts_with('"') && !raw.starts_with('\'')
        {
            format!("'{}'", raw)
        } else {
            raw
        }
    }

    fn node_source(&self, source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
        let bytes =
            &source.as_bytes()[node.location().start_offset()..node.location().end_offset()];
        String::from_utf8_lossy(bytes).into_owned()
    }

    fn is_literal(&self, node: &ruby_prism::Node<'_>) -> bool {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashSlice, "cops/style/hash_slice");
}
