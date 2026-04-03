use crate::cop::shared::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/FindBy
///
/// ## Investigation (2026-03-14): FP=10, FN=1
///
/// FPs were caused by two issues:
/// 1. `where(...).take(1)` was flagged — RuboCop only flags `take` with zero
///    arguments (checks `node.arguments.empty?`). Fixed by skipping when the
///    outer call has arguments.
/// 2. Location mismatch — nitrocop reported at the start of the entire chain
///    expression while RuboCop reports at the `take`/`first` keyword location
///    (RESTRICT_ON_SEND fires on the `take`/`first` node itself). Fixed by
///    using `message_loc()` on the outer call.
///
/// The remaining FN was a location mismatch in a multi-line chain (same offense, different line).
///
/// ## Investigation (2026-03-15): FP=3, FN=3
///
/// FP and FN root cause: RuboCop's `offense_range` reports from `where` to `take`/`first`,
/// starting at the `where` keyword line. For multiline `where(\n...\n).take`, nitrocop was
/// reporting at the `take` line (using outer call's message_loc), but RuboCop reports at
/// the `where` line. This caused FP at the `take` line and FN at the `where` line.
/// Fixed by reporting at `chain.inner_call.message_loc()` (the `where` keyword location).
pub struct FindBy;

impl Cop for FindBy {
    fn name(&self) -> &'static str {
        "Rails/FindBy"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let ignore_where_first = config.get_bool("IgnoreWhereFirst", true);

        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        let is_first = chain.outer_method == b"first";
        let is_take = chain.outer_method == b"take";

        if !is_first && !is_take {
            return;
        }

        if chain.inner_method != b"where" {
            return;
        }

        // IgnoreWhereFirst: when true, skip `where(...).first`
        if ignore_where_first && is_first {
            return;
        }

        // Skip `take(n)` / `first(n)` — RuboCop only flags zero-argument calls.
        let outer_call = node.as_call_node().expect("validated by as_method_chain");
        if outer_call
            .arguments()
            .is_some_and(|a| !a.arguments().is_empty())
        {
            return;
        }

        let method_name = if is_first { "first" } else { "take" };
        // RuboCop's offense_range goes from `where` to `take`/`first`, starting at `where`.
        // Report at the `where` keyword location to match RuboCop's line number.
        // For single-line calls this is the same line; for multiline `where(\n...\n).take`
        // this correctly reports at the `where` line instead of the `take` line.
        let where_call = &chain.inner_call;
        let loc = where_call
            .message_loc()
            .unwrap_or_else(|| where_call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `find_by` instead of `where.{method_name}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FindBy, "cops/rails/find_by");

    #[test]
    fn ignore_where_first_true_skips_first() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;

        // Default config (IgnoreWhereFirst: true) should NOT flag where.first
        let config = CopConfig::default();
        let source = b"User.where(name: 'foo').first\n";
        let diags = run_cop_full_with_config(&FindBy, source, config);
        assert!(
            diags.is_empty(),
            "IgnoreWhereFirst:true should skip where.first"
        );
    }

    #[test]
    fn ignore_where_first_true_flags_take() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;

        // Default config should still flag where.take
        let config = CopConfig::default();
        let source = b"User.where(name: 'foo').take\n";
        let diags = run_cop_full_with_config(&FindBy, source, config);
        assert!(
            !diags.is_empty(),
            "IgnoreWhereFirst:true should still flag where.take"
        );
    }

    #[test]
    fn ignore_where_first_false_flags_first() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "IgnoreWhereFirst".to_string(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        let source = b"User.where(name: 'foo').first\n";
        let diags = run_cop_full_with_config(&FindBy, source, config);
        assert!(
            !diags.is_empty(),
            "IgnoreWhereFirst:false should flag where.first"
        );
    }
}
