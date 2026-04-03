use crate::cop::shared::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, IT_LOCAL_VARIABLE_READ_NODE, IT_PARAMETERS_NODE,
    LOCAL_VARIABLE_READ_NODE, NUMBERED_PARAMETERS_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Detects identity `sort_by` blocks that RuboCop rewrites to `sort`.
///
/// Prism represents trailing-comma block params like `|name,|` as one required
/// parameter plus an `ImplicitRestNode`. The earlier port rejected any
/// `rest()` entry, which missed valid offenses such as
/// `sort_by { |name,| name }.each`. This cop now allows only that Prism
/// trailing-comma shape while still rejecting real rest params like
/// `|name, *rest|`.
pub struct RedundantSortBy;

impl Cop for RedundantSortBy {
    fn name(&self) -> &'static str {
        "Style/RedundantSortBy"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            IT_LOCAL_VARIABLE_READ_NODE,
            IT_PARAMETERS_NODE,
            LOCAL_VARIABLE_READ_NODE,
            NUMBERED_PARAMETERS_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
        ]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be `sort_by` method
        if call_node.name().as_slice() != b"sort_by" {
            return;
        }

        // Must have a receiver
        if call_node.receiver().is_none() {
            return;
        }

        // Must have a block
        let block = match call_node.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Block must have parameters and body that just returns the parameter
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };

        // Body must be a single statement
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        // Get the single body expression (either wrapped in StatementsNode or direct)
        let body_expr = if let Some(stmts) = body.as_statements_node() {
            let stmts_body: Vec<_> = stmts.body().iter().collect();
            if stmts_body.len() != 1 {
                return;
            }
            stmts_body.into_iter().next().unwrap()
        } else {
            body
        };

        // Determine the message based on parameter style
        let message = if let Some(bp) = params.as_block_parameters_node() {
            // Regular block: { |x| x }
            let inner_params = match bp.parameters() {
                Some(p) => p,
                None => return,
            };

            let requireds: Vec<_> = inner_params.requireds().iter().collect();
            if requireds.len() != 1 {
                return;
            }

            let has_explicit_rest = inner_params
                .rest()
                .is_some_and(|rest| rest.as_implicit_rest_node().is_none());

            if !inner_params.optionals().is_empty()
                || has_explicit_rest
                || !inner_params.posts().is_empty()
                || !inner_params.keywords().is_empty()
                || inner_params.keyword_rest().is_some()
                || inner_params.block().is_some()
            {
                return;
            }

            let param_name = match requireds[0].as_required_parameter_node() {
                Some(p) => p.name(),
                None => return,
            };

            let lvar = match body_expr.as_local_variable_read_node() {
                Some(l) => l,
                None => return,
            };

            if lvar.name().as_slice() != param_name.as_slice() {
                return;
            }

            let var_name = std::str::from_utf8(param_name.as_slice()).unwrap_or("x");
            format!(
                "Use `sort` instead of `sort_by {{ |{}| {} }}`.",
                var_name, var_name
            )
        } else if params.as_numbered_parameters_node().is_some() {
            // Numbered params (Ruby 2.7+): { _1 }
            let lvar = match body_expr.as_local_variable_read_node() {
                Some(l) => l,
                None => return,
            };
            if lvar.name().as_slice() != b"_1" {
                return;
            }
            "Use `sort` instead of `sort_by { _1 }`.".to_string()
        } else if params.as_it_parameters_node().is_some() {
            // Ruby 3.4+ `it` implicit parameter: { it }
            if body_expr.as_it_local_variable_read_node().is_none() {
                return;
            }
            "Use `sort` instead of `sort_by { it }`.".to_string()
        } else {
            return;
        };

        let msg_loc = call_node
            .message_loc()
            .unwrap_or_else(|| call_node.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        let mut diag = self.diagnostic(source, line, column, message);
        // Autocorrect: replace `sort_by { |x| x }` with `sort`
        if let Some(ref mut corr) = corrections {
            // Replace from `sort_by` to end of block with just `sort`
            corr.push(crate::correction::Correction {
                start: msg_loc.start_offset(),
                end: node.location().end_offset(),
                replacement: "sort".to_string(),
                cop_name: self.name(),
                cop_index: 0,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantSortBy, "cops/style/redundant_sort_by");
    crate::cop_autocorrect_fixture_tests!(RedundantSortBy, "cops/style/redundant_sort_by");
}
