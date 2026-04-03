use crate::cop::shared::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// CI baseline reported FP=30, FN=15.
///
/// The FP sample showed definitions like:
/// `def foo(\n  a, b\n)` where all parameters share a single continuation line.
/// RuboCop accepts those because it decides "multiline" from the parameter
/// nodes, not from the closing `)` line.
///
/// The FN sample was the mirror image: multiline definitions without
/// parentheses such as `def build_cache store,\n  logger, notifier`. The old
/// implementation required both `lparen_loc` and `rparen_loc`, so it skipped
/// that entire family.
///
/// This pass ports the RuboCop mixin behavior more directly: inspect the
/// ordered parameter nodes, consider `AllowMultilineFinalElement` only in the
/// initial all-on-same-line check, and flag any parameter whose `first_line`
/// does not advance past the previous accepted parameter's `last_line`.
pub struct MultilineMethodParameterLineBreaks;

impl Cop for MultilineMethodParameterLineBreaks {
    fn name(&self) -> &'static str {
        "Layout/MultilineMethodParameterLineBreaks"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
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
        let allow_multiline_final = config.get_bool("AllowMultilineFinalElement", false);

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let param_locs = collect_param_locs(params);
        if param_locs.is_empty() || all_on_same_line(source, &param_locs, allow_multiline_final) {
            return;
        }

        let mut last_seen_line = 0;
        for &(start, end) in &param_locs {
            let (curr_line, curr_col) = source.offset_to_line_col(start);
            if last_seen_line >= curr_line {
                diagnostics.push(self.diagnostic(
                    source,
                    curr_line,
                    curr_col,
                    "Each parameter in a multi-line method definition must start on a separate line."
                        .to_string(),
                ));
            } else {
                let end_offset = end.saturating_sub(1).max(start);
                last_seen_line = source.offset_to_line_col(end_offset).0;
            }
        }
    }
}

fn collect_param_locs(params: ruby_prism::ParametersNode<'_>) -> Vec<(usize, usize)> {
    let mut param_locs = Vec::new();

    for p in params.requireds().iter() {
        let loc = p.location();
        param_locs.push((loc.start_offset(), loc.end_offset()));
    }
    for p in params.optionals().iter() {
        let loc = p.location();
        param_locs.push((loc.start_offset(), loc.end_offset()));
    }
    if let Some(rest) = params.rest() {
        let loc = rest.location();
        param_locs.push((loc.start_offset(), loc.end_offset()));
    }
    for p in params.posts().iter() {
        let loc = p.location();
        param_locs.push((loc.start_offset(), loc.end_offset()));
    }
    for p in params.keywords().iter() {
        let loc = p.location();
        param_locs.push((loc.start_offset(), loc.end_offset()));
    }
    if let Some(kw_rest) = params.keyword_rest() {
        let loc = kw_rest.location();
        param_locs.push((loc.start_offset(), loc.end_offset()));
    }
    if let Some(block_param) = params.block() {
        let loc = block_param.location();
        param_locs.push((loc.start_offset(), loc.end_offset()));
    }

    param_locs.sort_by_key(|&(start, _)| start);
    param_locs
}

fn all_on_same_line(
    source: &SourceFile,
    param_locs: &[(usize, usize)],
    allow_multiline_final: bool,
) -> bool {
    let Some(&(first_start, _)) = param_locs.first() else {
        return true;
    };
    let Some(&(last_start, last_end)) = param_locs.last() else {
        return true;
    };

    let first_line = source.offset_to_line_col(first_start).0;
    let last_line = if allow_multiline_final {
        source.offset_to_line_col(last_start).0
    } else {
        let end_offset = last_end.saturating_sub(1).max(last_start);
        source.offset_to_line_col(end_offset).0
    };

    first_line == last_line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};
    use std::collections::HashMap;

    crate::cop_fixture_tests!(
        MultilineMethodParameterLineBreaks,
        "cops/layout/multiline_method_parameter_line_breaks"
    );

    #[test]
    fn flags_multiline_no_paren_definitions() {
        let diags = run_cop_full(
            &MultilineMethodParameterLineBreaks,
            b"def build_cache store,\n                logger, notifier\nend\n",
        );

        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
    }

    #[test]
    fn allow_multiline_final_element_ignores_multiline_last_parameter() {
        let config = CopConfig {
            options: HashMap::from([(
                "AllowMultilineFinalElement".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };

        let diags = run_cop_full_with_config(
            &MultilineMethodParameterLineBreaks,
            b"def foo(abc, foo, bar = {\n  a: 1,\n})\nend\n",
            config,
        );

        assert!(diags.is_empty());
    }
}
