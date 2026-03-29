use crate::cop::Cop;
use crate::diagnostic::Severity;

/// Checks for `# rubocop:disable` comments that can be removed.
///
/// This cop is special: it requires post-processing knowledge of which cops
/// actually fired offenses, so the detection logic lives in `lint_source_inner`
/// in `src/linter.rs`. This struct exists so the cop name is registered and
/// can be referenced in configuration (enabled/disabled/excluded).
///
/// ## Corpus investigation (2026-03-08)
///
/// FP=19 regressed because moved legacy directives like
/// `# rubocop:disable Style/MethodName` and `# rubocop:disable Metrics/LineLength`
/// stopped suppressing their current cops, so nitrocop started reporting the
/// directives themselves as redundant. Fixed centrally in `parse/directives.rs`
/// by honoring moved legacy names whose short name is unchanged.
///
/// ## Corpus investigation (2026-03-24)
///
/// FN=1102: The conservative approach in `is_directive_redundant` never flags
/// unused directives for enabled cops. Attempted aggressive approach (flag ALL
/// unused directives for known+enabled cops, matching RuboCop). This caused
/// the corpus smoke test to drop to 0% match rate — nitrocop's detection gaps
/// mean many directives that ARE needed (because nitrocop misses the offense
/// RuboCop catches) get incorrectly flagged as redundant. Reverted.
///
/// This FN is structural: it decreases naturally as nitrocop's cop coverage
/// improves. No per-cop fix is feasible without a "perfect cop" allowlist.
///
/// ## Corpus investigation (2026-03-29)
///
/// Two improvements:
///
/// 1. **Unknown cop flagging**: Cops not in registry and not renamed are now
///    flagged with "(unknown cop)" suffix, matching RuboCop. These directives
///    can never suppress anything. Added ~650 new matches.
///
/// 2. **Removed `--only` guard**: The `args.only.is_empty()` guard was removed
///    because `is_directive_redundant()` is conservative enough on its own —
///    it never flags directives for enabled cops that "didn't fire", only
///    disabled/excluded/renamed/unknown cops. This makes `check_cop.py --rerun`
///    work for this cop.
///
/// 3. **Renamed cop guard for `--only` mode**: For renamed cops (e.g.,
///    `Style/MethodName` → `Naming/MethodName`), the function now checks the
///    new-name cop's config state. In `--only` mode, if the new-name cop is
///    enabled, the old-name directive might be suppressing its offenses, so
///    we skip (conservative). In normal mode, `check_and_mark_used()` already
///    handles this correctly.
pub struct RedundantCopDisableDirective;

impl Cop for RedundantCopDisableDirective {
    fn name(&self) -> &'static str {
        "Lint/RedundantCopDisableDirective"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    // This cop is intentionally a no-op in check_lines/check_node/check_source.
    // The actual detection happens in lint_source_inner after all cops have run,
    // where we can determine which disable directives actually suppressed an offense.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cop_name() {
        assert_eq!(
            RedundantCopDisableDirective.name(),
            "Lint/RedundantCopDisableDirective"
        );
    }

    #[test]
    fn default_severity_is_warning() {
        assert_eq!(
            RedundantCopDisableDirective.default_severity(),
            Severity::Warning
        );
    }

    // Full-pipeline tests for this cop live in tests/integration.rs because
    // they need the complete linter pipeline (all cops running + post-processing).
}
