use crate::cop::shared::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE,
    HASH_NODE, INTERPOLATED_REGULAR_EXPRESSION_NODE, KEYWORD_HASH_NODE, LOCAL_VARIABLE_READ_NODE,
    REGULAR_EXPRESSION_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Detects `select`/`filter`/`find_all`/`reject` blocks whose sole predicate is
/// a direct regexp match on the block variable.
///
/// Matches RuboCop's broader direct-usage rules for:
/// - explicit single block params (`|item| item.match?(pattern)`, `pattern =~ item`)
/// - numbered params (`_1`)
/// - implicit `it` params
/// - regexp sources that are locals/constants/method calls, not just literals
///
/// Also preserves RuboCop's exclusions for:
/// - safe-navigation predicates like `text&.match?(...)`
/// - blocks with destructuring or extra params such as `|name, *_attrs|`
pub struct SelectByRegexp;

enum BlockArg<'a> {
    Named(&'a [u8]),
    Numbered,
    It,
}

impl SelectByRegexp {
    fn has_safe_navigation(call: &ruby_prism::CallNode<'_>) -> bool {
        call.call_operator_loc()
            .is_some_and(|loc| loc.as_slice() == b"&.")
    }

    fn single_body_call<'a>(body: &'a ruby_prism::Node<'a>) -> Option<ruby_prism::CallNode<'a>> {
        if let Some(stmts) = body.as_statements_node() {
            let mut body_nodes = stmts.body().iter();
            let first = body_nodes.next()?;
            if body_nodes.next().is_some() {
                return None;
            }
            return first.as_call_node();
        }

        body.as_call_node()
    }

    fn block_arg<'a>(block_node: &'a ruby_prism::BlockNode<'a>) -> Option<BlockArg<'a>> {
        let params = block_node.parameters()?;

        if let Some(numbered) = params.as_numbered_parameters_node() {
            return if numbered.maximum() == 1 {
                Some(BlockArg::Numbered)
            } else {
                None
            };
        }

        if params.as_it_parameters_node().is_some() {
            return Some(BlockArg::It);
        }

        let block_params = params.as_block_parameters_node()?;
        let inner = block_params.parameters()?;
        let requireds: Vec<_> = inner.requireds().iter().collect();

        let has_explicit_rest = inner
            .rest()
            .is_some_and(|rest| rest.as_implicit_rest_node().is_none());

        if requireds.len() != 1
            || !inner.optionals().is_empty()
            || has_explicit_rest
            || !inner.posts().is_empty()
            || !inner.keywords().is_empty()
            || inner.keyword_rest().is_some()
            || inner.block().is_some()
        {
            return None;
        }

        let required = requireds[0].as_required_parameter_node()?;
        Some(BlockArg::Named(required.name().as_slice()))
    }

    fn matches_block_arg(node: &ruby_prism::Node<'_>, block_arg: &BlockArg<'_>) -> bool {
        match block_arg {
            BlockArg::Named(name) => node
                .as_local_variable_read_node()
                .is_some_and(|lvar| lvar.name().as_slice() == *name),
            BlockArg::Numbered => node
                .as_local_variable_read_node()
                .is_some_and(|lvar| lvar.name().as_slice() == b"_1"),
            BlockArg::It => node.as_it_local_variable_read_node().is_some(),
        }
    }

    fn extract_match_call<'a>(
        body: &'a ruby_prism::Node<'a>,
        block_arg: &BlockArg<'a>,
    ) -> Option<ruby_prism::CallNode<'a>> {
        let call = Self::single_body_call(body)?;

        if Self::has_safe_navigation(&call) {
            return None;
        }

        let name = call.name().as_slice();
        if !matches!(name, b"match?" | b"=~" | b"!~") {
            return None;
        }

        let receiver = call.receiver()?;
        let args = call.arguments()?;
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return None;
        }

        if Self::matches_block_arg(&receiver, block_arg)
            || Self::matches_block_arg(&arg_list[0], block_arg)
        {
            return Some(call);
        }

        None
    }

    fn replacement(method_name: &[u8], body_method_name: &[u8]) -> Option<&'static str> {
        let mismatch = body_method_name == b"!~";

        match (method_name, mismatch) {
            (b"select" | b"filter" | b"find_all", false) => Some("grep"),
            (b"reject", false) => Some("grep_v"),
            (b"select" | b"filter" | b"find_all", true) => Some("grep_v"),
            (b"reject", true) => Some("grep"),
            _ => None,
        }
    }

    fn is_hash_receiver(node: &ruby_prism::Node<'_>) -> bool {
        if node.as_hash_node().is_some() || node.as_keyword_hash_node().is_some() {
            return true;
        }
        if let Some(call) = node.as_call_node() {
            let name = call.name();
            let name_bytes = name.as_slice();
            if matches!(name_bytes, b"to_h" | b"to_hash") {
                return true;
            }
            if matches!(name_bytes, b"new" | b"[]") {
                if let Some(recv) = call.receiver() {
                    if let Some(cr) = recv.as_constant_read_node() {
                        if cr.name().as_slice() == b"Hash" {
                            return true;
                        }
                    }
                    if let Some(cp) = recv.as_constant_path_node() {
                        if cp.location().as_slice().ends_with(b"Hash") {
                            return true;
                        }
                    }
                }
            }
        }
        if let Some(cr) = node.as_constant_read_node() {
            if cr.name().as_slice() == b"ENV" {
                return true;
            }
        }
        if let Some(cp) = node.as_constant_path_node() {
            if cp.location().as_slice().ends_with(b"ENV") {
                return true;
            }
        }
        false
    }
}

impl Cop for SelectByRegexp {
    fn name(&self) -> &'static str {
        "Style/SelectByRegexp"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            HASH_NODE,
            INTERPOLATED_REGULAR_EXPRESSION_NODE,
            KEYWORD_HASH_NODE,
            LOCAL_VARIABLE_READ_NODE,
            REGULAR_EXPRESSION_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
        ]
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
        // We check the CallNode; its block() gives us the BlockNode
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        // Must be select, filter, find_all, or reject
        if !matches!(
            method_bytes,
            b"select" | b"filter" | b"find_all" | b"reject"
        ) {
            return;
        }

        // Must not be called on a hash-like receiver
        if let Some(receiver) = call.receiver() {
            if Self::is_hash_receiver(&receiver) {
                return;
            }
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let block_arg = match Self::block_arg(&block_node) {
            Some(arg) => arg,
            None => return,
        };

        // Block body must be a single expression that matches regexp
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let body_call = match Self::extract_match_call(&body, &block_arg) {
            Some(call) => call,
            None => return,
        };

        let replacement = match Self::replacement(method_bytes, body_call.name().as_slice()) {
            Some(replacement) => replacement,
            None => return,
        };

        let method_str = std::str::from_utf8(method_bytes).unwrap_or("select");
        // Report on the whole call including block
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Prefer `{}` to `{}` with a regexp match.",
                replacement, method_str
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SelectByRegexp, "cops/style/select_by_regexp");
}
