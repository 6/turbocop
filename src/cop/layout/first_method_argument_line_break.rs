use crate::cop::shared::node_type::{CALL_NODE, SUPER_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// CI baseline reported FP=38, FN=149.
///
/// The FN sample was dominated by explicit `super(...)` calls and
/// config-sensitive cases. The old implementation only inspected `CallNode`,
/// ignored `AllowedMethods`, and always treated a multiline final argument as
/// an offense even when `AllowMultilineFinalElement: true`.
///
/// This pass mirrors RuboCop's first-element check more closely for
/// parenthesized calls: inspect both `CallNode` and `SuperNode`, honor
/// `AllowedMethods`, and decide multiline-ness from the argument nodes
/// themselves instead of the closing delimiter line.
///
/// Acceptance gate after the fix:
/// `expected=69,729`, `actual=72,210`, `CI baseline=69,618`, `missing=0`.
/// The raw delta (`+2,592`) stayed within `jruby`'s parser-crash file-drop
/// noise (`4,141`), so the rerun passed unchanged.
///
/// ## Corpus investigation (2026-03-14)
///
/// CI baseline: FP=0, FN=62. All 62 FNs were multi-line calls where the
/// `BlockArgumentNode` (`&block`, `&handler`, `&@callback`) was on a different
/// line from the other arguments. In Prism, `BlockArgumentNode` lives on
/// `call.block()`, not in `call.arguments()`, so `collect_arg_locs` never
/// included it. The `last_line` check saw all regular args on the same line
/// and returned early (no offense). Fix: include `call.block()` when it is a
/// `BlockArgumentNode` in the arg_locs vector.
///
/// ## Corpus investigation (2026-03-15)
///
/// CI baseline: FP=0, FN=5. The remaining FNs were `super(arg, &block)` calls
/// where `&block` was on a different line. The `SuperNode` branch was missing
/// the same `block()` handling that was added for `CallNode` in the previous
/// fix. Fix: include `super_node.block()` when it is a `BlockArgumentNode`
/// in the arg_locs vector, mirroring the `CallNode` logic.
///
/// ## Corpus investigation (2026-03-16)
///
/// CI baseline: FP=0, FN=5. All 5 remaining FNs had `&block_arg` as the SOLE
/// argument inside parentheses (e.g., `configure(&configuration_block(...))`).
/// In Prism, `BlockArgumentNode` lives on `call.block()`, not in
/// `call.arguments()`, so when no other arguments exist, `call.arguments()`
/// returns `None`. The old code did `let Some(args) = call.arguments() else {
/// return; }`, bailing before ever checking `call.block()`. Fix: initialize
/// `arg_locs` from `call.arguments()` when present (or empty vec when `None`),
/// then always check `call.block()` for `BlockArgumentNode`. Same fix applied
/// to the `SuperNode` branch.
pub struct FirstMethodArgumentLineBreak;

impl Cop for FirstMethodArgumentLineBreak {
    fn name(&self) -> &'static str {
        "Layout/FirstMethodArgumentLineBreak"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, SUPER_NODE]
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
        let allowed_methods = config
            .get_string_array("AllowedMethods")
            .unwrap_or_default();

        let (start_offset, arg_locs) = if let Some(call) = node.as_call_node() {
            if allowed_methods
                .iter()
                .any(|method| method.as_bytes() == call.name().as_slice())
            {
                return;
            }

            let (Some(open_loc), Some(close_loc)) = (call.opening_loc(), call.closing_loc()) else {
                return;
            };
            if open_loc.as_slice() != b"(" || close_loc.as_slice() != b")" {
                return;
            }

            let mut arg_locs = if let Some(args) = call.arguments() {
                collect_arg_locs(args.arguments().iter().collect())
            } else {
                Vec::new()
            };

            // In Prism, BlockArgumentNode (&block) is on call.block(), not in arguments().
            // Include it so multiline detection accounts for block args on different lines.
            if let Some(block) = call.block() {
                if block.as_block_argument_node().is_some() {
                    let loc = block.location();
                    arg_locs.push((loc.start_offset(), loc.end_offset()));
                }
            }

            (call.location().start_offset(), arg_locs)
        } else if let Some(super_node) = node.as_super_node() {
            if super_node.lparen_loc().is_none() {
                return;
            }

            let mut arg_locs = if let Some(args) = super_node.arguments() {
                collect_arg_locs(args.arguments().iter().collect())
            } else {
                Vec::new()
            };

            // In Prism, BlockArgumentNode (&block) is on super_node.block(), not in arguments().
            // Include it so multiline detection accounts for block args on different lines.
            if let Some(block) = super_node.block() {
                if block.as_block_argument_node().is_some() {
                    let loc = block.location();
                    arg_locs.push((loc.start_offset(), loc.end_offset()));
                }
            }

            (super_node.location().start_offset(), arg_locs)
        } else {
            return;
        };

        let Some(&(first_start, _)) = first_by_line(source, &arg_locs) else {
            return;
        };

        let (start_line, _) = source.offset_to_line_col(start_offset);
        let (first_line, first_col) = source.offset_to_line_col(first_start);
        if first_line != start_line {
            return;
        }

        if start_line == last_line(source, &arg_locs, allow_multiline_final) {
            return;
        }

        diagnostics.push(self.diagnostic(
            source,
            first_line,
            first_col,
            "Add a line break before the first argument of a multi-line method call.".to_string(),
        ));
    }
}

fn collect_arg_locs(args: Vec<ruby_prism::Node<'_>>) -> Vec<(usize, usize)> {
    let mut arg_locs = Vec::new();

    for (index, arg) in args.iter().enumerate() {
        if index == args.len() - 1 {
            if let Some(keyword_hash) = arg.as_keyword_hash_node() {
                for element in keyword_hash.elements().iter() {
                    let loc = element.location();
                    arg_locs.push((loc.start_offset(), loc.end_offset()));
                }
                continue;
            }
        }

        let loc = arg.location();
        arg_locs.push((loc.start_offset(), loc.end_offset()));
    }

    arg_locs
}

fn first_by_line<'a>(
    source: &SourceFile,
    arg_locs: &'a [(usize, usize)],
) -> Option<&'a (usize, usize)> {
    arg_locs
        .iter()
        .min_by_key(|&&(start, _)| source.offset_to_line_col(start).0)
}

fn last_line(source: &SourceFile, arg_locs: &[(usize, usize)], ignore_last: bool) -> usize {
    arg_locs
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
        FirstMethodArgumentLineBreak,
        "cops/layout/first_method_argument_line_break"
    );

    #[test]
    fn allow_multiline_final_element_ignores_multiline_last_hash_argument() {
        let config = CopConfig {
            options: HashMap::from([(
                "AllowMultilineFinalElement".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };

        let diags = run_cop_full_with_config(
            &FirstMethodArgumentLineBreak,
            b"foo(bar, {\n  a: 1,\n})\n",
            config,
        );

        assert!(diags.is_empty());
    }

    #[test]
    fn allowed_methods_skip_configured_calls() {
        let config = CopConfig {
            options: HashMap::from([(
                "AllowedMethods".into(),
                serde_yml::to_value(vec!["something"]).unwrap(),
            )]),
            ..CopConfig::default()
        };

        let diags = run_cop_full_with_config(
            &FirstMethodArgumentLineBreak,
            b"something(bar,\n  baz)\n",
            config,
        );

        assert!(diags.is_empty());
    }
}
