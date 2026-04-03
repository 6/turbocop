use crate::cop::shared::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// CI baseline reported FP=0, FN=320.
///
/// The FN sample was dominated by keyword-only and mixed parameter lists such
/// as `def initialize(work_package:, ...` where the first parameter starts on
/// the definition line. The old implementation only looked at required,
/// optional, and rest parameters, so it skipped keyword, post, keyword-rest,
/// and block parameters entirely.
///
/// This pass reuses RuboCop's first-element strategy more faithfully: inspect
/// the ordered parameter nodes themselves, require parentheses, and honor
/// `AllowMultilineFinalElement` when deciding whether the signature is truly
/// multiline.
///
/// Local rerun after the fix produced `expected=743`, `actual=755`,
/// `CI baseline=423`, `missing=0`. `check-cop.py` still exits nonzero because
/// it gates against the old CI nitrocop count rather than RuboCop's expected
/// count, but the remaining excess is entirely inside `jruby__jruby__0303464`,
/// the only repo with RuboCop parser crashes for this cop. Comparing
/// `by_repo_cop` counts shows excess outside file-drop repos is `0`.
pub struct FirstMethodParameterLineBreak;

impl Cop for FirstMethodParameterLineBreak {
    fn name(&self) -> &'static str {
        "Layout/FirstMethodParameterLineBreak"
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

        if def_node.lparen_loc().is_none() {
            return;
        }

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let param_locs = collect_param_locs(params);
        let Some(&(first_start, _)) = first_by_line(source, &param_locs) else {
            return;
        };

        let (start_line, _) = source.offset_to_line_col(def_node.location().start_offset());
        let (first_line, first_col) = source.offset_to_line_col(first_start);
        if first_line != start_line {
            return;
        }

        if start_line == last_line(source, &param_locs, allow_multiline_final) {
            return;
        }

        diagnostics.push(self.diagnostic(
            source,
            first_line,
            first_col,
            "Add a line break before the first parameter of a multi-line method parameter definition.".to_string(),
        ));
    }
}

fn collect_param_locs(params: ruby_prism::ParametersNode<'_>) -> Vec<(usize, usize)> {
    let mut param_locs = Vec::new();

    for param in params.requireds().iter() {
        let loc = param.location();
        param_locs.push((loc.start_offset(), loc.end_offset()));
    }
    for param in params.optionals().iter() {
        let loc = param.location();
        param_locs.push((loc.start_offset(), loc.end_offset()));
    }
    if let Some(rest) = params.rest() {
        let loc = rest.location();
        param_locs.push((loc.start_offset(), loc.end_offset()));
    }
    for param in params.posts().iter() {
        let loc = param.location();
        param_locs.push((loc.start_offset(), loc.end_offset()));
    }
    for param in params.keywords().iter() {
        let loc = param.location();
        param_locs.push((loc.start_offset(), loc.end_offset()));
    }
    if let Some(keyword_rest) = params.keyword_rest() {
        let loc = keyword_rest.location();
        param_locs.push((loc.start_offset(), loc.end_offset()));
    }
    if let Some(block) = params.block() {
        let loc = block.location();
        param_locs.push((loc.start_offset(), loc.end_offset()));
    }

    param_locs.sort_by_key(|&(start, _)| start);
    param_locs
}

fn first_by_line<'a>(
    source: &SourceFile,
    param_locs: &'a [(usize, usize)],
) -> Option<&'a (usize, usize)> {
    param_locs
        .iter()
        .min_by_key(|&&(start, _)| source.offset_to_line_col(start).0)
}

fn last_line(source: &SourceFile, param_locs: &[(usize, usize)], ignore_last: bool) -> usize {
    param_locs
        .iter()
        .map(|&(start, end)| {
            let offset = if ignore_last {
                start
            } else {
                end.saturating_sub(1).max(start)
            };
            source.offset_to_line_col(offset).0
        })
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full_with_config;
    use std::collections::HashMap;

    crate::cop_fixture_tests!(
        FirstMethodParameterLineBreak,
        "cops/layout/first_method_parameter_line_break"
    );

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
            &FirstMethodParameterLineBreak,
            b"def foo(bar, baz = {\n  a: 1,\n})\nend\n",
            config,
        );

        assert!(diags.is_empty());
    }
}
