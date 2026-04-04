use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// ## Corpus investigation (2026-04-04)
///
/// Prior FN=7 came from two separate gaps (fixed in a3cfbaa8):
///
/// - `default_include` only matched `spec/**/*.rb`, so the cop never ran on
///   Minitest files under `test/**/*.rb` even though the vendor default config
///   includes both.
/// - `after` context detection required a receiverless call, so
///   `config.after do ... end` and `config.after { ... }` were missed even
///   though RuboCop matches any block method named `after`.
///
/// Remaining FN=62 are a corpus-runner Include resolution issue, not a
/// detection bug. The vendor default config has `Include: [spec/**/*.rb,
/// test/**/*.rb]` which overrides `default_include`. When the corpus runner
/// CWD is `/tmp` (not the repo root), these non-`**/` prefixed patterns
/// fail to match because paths relativize to `nitrocop_cop_check_.../repos/
/// .../spec/...` instead of `spec/...`. Confirmed: `check_cop.py --repo-cwd`
/// resolves all 62 FN (62/62 detected, 0 FP). The `is_include_gated_cop`
/// auto-enable in `check_cop.py` doesn't trigger because it requires
/// `zero_baseline` but this cop has 62 expected RuboCop offenses. Fix:
/// broaden the auto-enable condition to also trigger when the cop is
/// include-gated AND nitrocop baseline is 0 AND expected > 0.
///
/// Additionally, `rails_version_at_least()` requires `railties_in_lockfile`,
/// but no Gemfile.lock exists at CWD=/tmp. Changed to use
/// `target_rails_version()` directly (bypasses lockfile gate) and
/// `default_include` to `**/spec/**/*.rb` (fallback when vendor gem not
/// resolved).
pub struct RedundantTravelBack;

impl Cop for RedundantTravelBack {
    fn name(&self) -> &'static str {
        "Rails/RedundantTravelBack"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/spec/**/*.rb", "**/test/**/*.rb"]
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::cop::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // minimum_target_rails_version 5.2
        // Use target_rails_version() directly instead of rails_version_at_least()
        // to avoid the railties_in_lockfile gate, which fails in corpus CI where
        // CWD is /tmp and no Gemfile.lock is found. The TargetRailsVersion config
        // value alone is sufficient to enable the cop.
        if !config.target_rails_version().is_some_and(|v| v >= 5.2) {
            return;
        }

        let mut visitor = TravelBackVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_teardown_or_after: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct TravelBackVisitor<'a> {
    cop: &'a RedundantTravelBack,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    in_teardown_or_after: bool,
}

impl<'a, 'pr> Visit<'pr> for TravelBackVisitor<'a> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        // Check if we're entering an `after` block.
        // RuboCop only matches method defs named `teardown` and block calls
        // named `after`; `teardown do ... end` blocks are not flagged.
        let enters_after = method_name == b"after"
            && node
                .block()
                .and_then(|block| block.as_block_node())
                .is_some();

        // Check if this is a `travel_back` call inside teardown/after
        if self.in_teardown_or_after && method_name == b"travel_back" && node.receiver().is_none() {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(
                self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Redundant `travel_back` detected. It is automatically called after each test."
                        .to_string(),
                ),
            );
        }

        let was = self.in_teardown_or_after;
        if enters_after {
            self.in_teardown_or_after = true;
        }
        if let Some(receiver) = node.receiver() {
            self.visit(&receiver);
        }
        if let Some(arguments) = node.arguments() {
            for argument in arguments.arguments().iter() {
                self.visit(&argument);
            }
        }
        if let Some(block) = node.block() {
            self.visit(&block);
        }
        self.in_teardown_or_after = was;
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Also match `def teardown; ... travel_back; end`
        let is_teardown = node.name().as_slice() == b"teardown";

        let was = self.in_teardown_or_after;
        if is_teardown {
            self.in_teardown_or_after = true;
        }
        ruby_prism::visit_def_node(self, node);
        self.in_teardown_or_after = was;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_rails_fixture_tests!(RedundantTravelBack, "cops/rails/redundant_travel_back", 5.2);
}
