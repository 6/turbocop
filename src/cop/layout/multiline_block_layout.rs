use crate::cop::node_type::{CALL_NODE, LAMBDA_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Layout/MultilineBlockLayout
///
/// Checks whether multiline do..end or brace blocks have a newline after the
/// block start. Also checks that block arguments are on the same line as the
/// block opener.
///
/// Handles both regular blocks (`foo { }`, `foo do end`) and lambda literals
/// (`-> { }`, `-> do end`). In Prism, regular blocks are `BlockNode` while
/// lambda literals are `LambdaNode` — both must be checked.
///
/// RuboCop uses `on_block` aliased to `on_numblock` and `on_itblock`, which
/// covers all block variants. In Prism, numbered-parameter blocks and
/// it-blocks are still `BlockNode` (with implicit parameter nodes), so
/// handling `BlockNode` + `LambdaNode` covers all cases.
///
/// Corpus fixes:
/// - 2026-03-29: multiline block-argument FNs in long call sites came from
///   measuring only the `do` line, while a remaining FP came from flattening
///   multiline default values inside a single parameter. Match RuboCop's
///   length check by measuring against the enclosing call/lambda first line and
///   rebuilding the top-level block-argument string with `, ` separators while
///   preserving each parameter's own source.
/// - 2026-03-30: Prism stores block-local vars (`|; x|`, `|x; y|`) on
///   `BlockParametersNode.locals()` instead of `parameters()`. Include those
///   locals in the explicit-args check so multiline block-local declarations
///   are flagged like RuboCop.
pub struct MultilineBlockLayout;

impl Cop for MultilineBlockLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineBlockLayout"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, LAMBDA_NODE]
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
        // Regular Ruby blocks are represented by a CallNode with an attached
        // BlockNode. Use the CallNode location so the line-length exemption
        // sees the full expression, matching RuboCop's block node source.
        if let Some(call_node) = node.as_call_node() {
            let Some(block_node) = call_node.block().and_then(|block| block.as_block_node()) else {
                return;
            };

            self.check_block(
                source,
                call_node.location(),
                block_node.opening_loc(),
                block_node.closing_loc(),
                block_node.parameters(),
                block_node.body(),
                config,
                diagnostics,
            );
        } else if let Some(lambda_node) = node.as_lambda_node() {
            self.check_block(
                source,
                lambda_node.location(),
                lambda_node.opening_loc(),
                lambda_node.closing_loc(),
                lambda_node.parameters(),
                lambda_node.body(),
                config,
                diagnostics,
            );
        }
    }
}

impl MultilineBlockLayout {
    #[allow(clippy::too_many_arguments)]
    fn check_block(
        &self,
        source: &SourceFile,
        node_loc: ruby_prism::Location<'_>,
        opening_loc: ruby_prism::Location<'_>,
        closing_loc: ruby_prism::Location<'_>,
        parameters: Option<ruby_prism::Node<'_>>,
        body: Option<ruby_prism::Node<'_>>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let (open_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let (close_line, _) = source.offset_to_line_col(closing_loc.start_offset());

        // Single line block — no offense
        if open_line == close_line {
            return;
        }

        // Check 1: Block arguments should be on the same line as block start.
        // Skip implicit parameter nodes (`it`, `_1`) and explicit empty params.
        if let Some(params) = parameters.and_then(explicit_params_info) {
            let (params_end_line, _) =
                source.offset_to_line_col(params.expression_end.saturating_sub(1));
            if params_end_line != open_line {
                let line_break_necessary = get_max_line_length(config).is_some_and(|max_len| {
                    line_break_necessary_in_args(source, node_loc, &params, max_len)
                });

                if !line_break_necessary {
                    let (params_line, params_col) =
                        source.offset_to_line_col(params.expression_start);
                    diagnostics.push(
                        self.diagnostic(
                            source,
                            params_line,
                            params_col,
                            "Block argument expression is not on the same line as the block start."
                                .to_string(),
                        ),
                    );
                }
            }
        }

        // Check 2: Block body should NOT be on the same line as block start
        if let Some(body) = body {
            // When the block contains rescue/ensure, Prism wraps the body in a
            // BeginNode whose location spans from the `do`/`{` keyword — not from
            // the first actual statement.  Unwrap to find the real first expression.
            let first_expr_offset = if let Some(begin_node) = body.as_begin_node() {
                if let Some(stmts) = begin_node.statements() {
                    let children: Vec<ruby_prism::Node<'_>> = stmts.body().iter().collect();
                    children.first().map(|n| n.location().start_offset())
                } else {
                    // No statements before rescue/ensure — use rescue clause location
                    begin_node
                        .rescue_clause()
                        .map(|r| r.location().start_offset())
                }
            } else {
                Some(body.location().start_offset())
            };

            if let Some(offset) = first_expr_offset {
                let (body_line, body_col) = source.offset_to_line_col(offset);
                if body_line == open_line {
                    diagnostics.push(self.diagnostic(
                        source,
                        body_line,
                        body_col,
                        "Block body expression is on the same line as the block start.".to_string(),
                    ));
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum ParameterDelimiterStyle {
    None,
    Pipes,
    Parentheses,
}

struct ExplicitParamsInfo {
    expression_start: usize,
    expression_end: usize,
    content_start: usize,
    content_end: usize,
    delimiter_style: ParameterDelimiterStyle,
    param_ranges: Vec<(usize, usize)>,
    regular_param_count: usize,
}

fn explicit_params_info(params: ruby_prism::Node<'_>) -> Option<ExplicitParamsInfo> {
    if params.as_it_parameters_node().is_some() || params.as_numbered_parameters_node().is_some() {
        return None;
    }

    if let Some(block_params) = params.as_block_parameters_node() {
        let expression_loc = block_params.location();
        let delimiter_style = match (
            expression_loc.as_slice().first(),
            expression_loc.as_slice().last(),
        ) {
            (Some(b'|'), Some(b'|')) => ParameterDelimiterStyle::Pipes,
            (Some(b'('), Some(b')')) => ParameterDelimiterStyle::Parentheses,
            _ => ParameterDelimiterStyle::None,
        };
        let regular_param_ranges = block_params
            .parameters()
            .map(collect_ordered_param_ranges)
            .unwrap_or_default();
        let mut param_ranges = regular_param_ranges.clone();
        param_ranges.extend(collect_block_local_ranges(&block_params));

        if param_ranges.is_empty() {
            return None;
        }

        let (content_start, content_end) = if let Some(inner_params) = block_params.parameters() {
            let content_loc = inner_params.location();
            (content_loc.start_offset(), content_loc.end_offset())
        } else {
            (
                param_ranges.first().unwrap().0,
                param_ranges.last().unwrap().1,
            )
        };

        return Some(ExplicitParamsInfo {
            expression_start: expression_loc.start_offset(),
            expression_end: expression_loc.end_offset(),
            content_start,
            content_end,
            delimiter_style,
            param_ranges,
            regular_param_count: regular_param_ranges.len(),
        });
    }

    if let Some(inner_params) = params.as_parameters_node() {
        let loc = inner_params.location();
        let param_ranges = collect_ordered_param_ranges(inner_params);
        return Some(ExplicitParamsInfo {
            expression_start: loc.start_offset(),
            expression_end: loc.end_offset(),
            content_start: loc.start_offset(),
            content_end: loc.end_offset(),
            delimiter_style: ParameterDelimiterStyle::None,
            regular_param_count: param_ranges.len(),
            param_ranges,
        });
    }

    let loc = params.location();
    Some(ExplicitParamsInfo {
        expression_start: loc.start_offset(),
        expression_end: loc.end_offset(),
        content_start: loc.start_offset(),
        content_end: loc.end_offset(),
        delimiter_style: ParameterDelimiterStyle::None,
        param_ranges: vec![(loc.start_offset(), loc.end_offset())],
        regular_param_count: 1,
    })
}

fn collect_ordered_param_ranges(params: ruby_prism::ParametersNode<'_>) -> Vec<(usize, usize)> {
    let mut param_ranges = Vec::new();

    for param in params.requireds().iter() {
        let loc = param.location();
        param_ranges.push((loc.start_offset(), loc.end_offset()));
    }
    for param in params.optionals().iter() {
        let loc = param.location();
        param_ranges.push((loc.start_offset(), loc.end_offset()));
    }
    if let Some(rest) = params.rest() {
        let loc = rest.location();
        param_ranges.push((loc.start_offset(), loc.end_offset()));
    }
    for param in params.posts().iter() {
        let loc = param.location();
        param_ranges.push((loc.start_offset(), loc.end_offset()));
    }
    for param in params.keywords().iter() {
        let loc = param.location();
        param_ranges.push((loc.start_offset(), loc.end_offset()));
    }
    if let Some(keyword_rest) = params.keyword_rest() {
        let loc = keyword_rest.location();
        param_ranges.push((loc.start_offset(), loc.end_offset()));
    }
    if let Some(block) = params.block() {
        let loc = block.location();
        param_ranges.push((loc.start_offset(), loc.end_offset()));
    }

    param_ranges.sort_by_key(|&(start, _)| start);
    param_ranges
}

fn collect_block_local_ranges(
    block_params: &ruby_prism::BlockParametersNode<'_>,
) -> Vec<(usize, usize)> {
    let mut local_ranges = Vec::new();

    for local in block_params.locals().iter() {
        let loc = local.location();
        local_ranges.push((loc.start_offset(), loc.end_offset()));
    }

    local_ranges
}

/// Get the max line length from config. Checks for a cross-cop injected
/// MaxLineLength key, falling back to a default of 120.
fn get_max_line_length(config: &CopConfig) -> Option<usize> {
    // Check for explicitly configured MaxLineLength on this cop
    if let Some(val) = config.options.get("MaxLineLength") {
        return val.as_u64().map(|v| v as usize);
    }
    // Default: use 120 (RuboCop's default Layout/LineLength Max)
    Some(120)
}

fn char_len(source: &[u8]) -> usize {
    source.iter().filter(|&&b| (b & 0xC0) != 0x80).count()
}

fn trimmed_ends_with_comma(source: &[u8]) -> bool {
    source
        .iter()
        .rfind(|&&b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
        .is_some_and(|&b| b == b',')
}

fn block_arg_string_len(source: &SourceFile, params: &ExplicitParamsInfo) -> usize {
    let bytes = source.as_bytes();
    let joined_len = params
        .param_ranges
        .iter()
        .enumerate()
        .map(|(index, &(start, end))| {
            let separator_len = if index == 0 { 0 } else { 2 };
            separator_len + char_len(&bytes[start..end])
        })
        .sum::<usize>();

    let trailing_comma_len = usize::from(
        params.regular_param_count == 1
            && trimmed_ends_with_comma(&bytes[params.content_start..params.content_end]),
    );

    joined_len + trailing_comma_len
}

fn line_break_necessary_in_args(
    source: &SourceFile,
    node_loc: ruby_prism::Location<'_>,
    params: &ExplicitParamsInfo,
    max_len: usize,
) -> bool {
    let bytes = source.as_bytes();
    let first_line_end = bytes[node_loc.start_offset()..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|idx| node_loc.start_offset() + idx)
        .unwrap_or(bytes.len());
    let first_line = &bytes[node_loc.start_offset()..first_line_end];
    let first_line_len = char_len(first_line);
    let open_col = source.offset_to_line_col(node_loc.start_offset()).1;

    let extra_chars = match params.delimiter_style {
        ParameterDelimiterStyle::Pipes => {
            if first_line.ends_with(b"|") {
                1
            } else {
                3
            }
        }
        ParameterDelimiterStyle::Parentheses => {
            if first_line.ends_with(b"(") {
                1
            } else {
                2
            }
        }
        ParameterDelimiterStyle::None => 1,
    };

    open_col + first_line_len + extra_chars + block_arg_string_len(source, params) > max_len
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(MultilineBlockLayout, "cops/layout/multiline_block_layout");

    #[test]
    fn flags_multiline_block_args_at_exact_line_length_boundary() {
        let diags = run_cop_full(
            &MultilineBlockLayout,
            br#"define_deprecated_method_by_hash_args :initialize,
    "title, parent, action, back, *buttons",
    ":title => nil, :parent => nil, :action => :open, :buttons => nil" do
    |_self, title, parent, action, back, *buttons|
  options = {}
end
"#,
        );

        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 4);
        assert_eq!(diags[0].location.column, 4);
    }

    #[test]
    fn allows_multiline_block_args_when_joined_line_would_be_too_long() {
        let diags = run_cop_full(
            &MultilineBlockLayout,
            br#"define_command(:grep) do
  |cmd = read_from_minibuffer("Grep: ",
                              initial_value: CONFIG[:grep_command] + " ")|
  shell_execute(cmd, buffer_name: "*grep*", mode: BacktraceMode)
end
"#,
        );

        assert!(diags.is_empty());
    }
}
