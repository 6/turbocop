use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-07)
///
/// FP=26, FN=1. FPs from safe navigation (`!arr&.include?(x)`) and multi-arg
/// `include?` calls. RuboCop's pattern `(send (send $!nil? :include? $_) :!)`
/// uses `send` (not `csend`) and `$_` (exactly one arg).
/// Fixed by checking for safe navigation and argument count.
///
/// ## Corpus investigation (2026-03-16)
///
/// FP=0, FN=1. The remaining FN is in `rubocop__rubocop__b210a6e` at
/// `lib/rubocop/cop/lint/cop_directive_syntax.rb:74` —
/// `elsif !DirectiveComment::AVAILABLE_MODES.include?(mode)`. Verified that
/// the cop logic correctly detects `!` calls with constant path receivers in
/// both `if` and `elsif` conditions (test fixtures added). The FN is a
/// corpus config artifact — likely the rubocop repo's config resolution
/// differs from the baseline, causing this cop to not run on that file.
///
/// ## Corpus investigation (2026-03-19)
///
/// FP=3, FN=0. All 3 FPs are `![TkFOR, TkWHILE, TkUNTIL].include?(...)`
/// in vendored gem files:
///   - `heroku/ruby/1.9.1/gems/rdoc-*/lib/rdoc/ruby_lex.rb` (cjstewart88__Tubalr, 2 FPs)
///   - `vendor/bundle/ruby/2.3.0/gems/rdoc-4.3.0/lib/rdoc/ruby_lex.rb` (liaoziyang__stackneveroverflow, 1 FP)
///
/// Root cause: file-exclusion path resolution, NOT cop logic. RuboCop
/// correctly flags `![...].include?(x)` too (verified locally). The corpus
/// oracle runs nitrocop on `repos/REPO_ID/`, producing paths like
/// `repos/REPO_ID/vendor/bundle/...` which don't match the `vendor/**/*`
/// AllCops.Exclude glob because the repo prefix prevents matching. RuboCop
/// uses `--force-exclusion` which handles this correctly. The `heroku/`
/// paths aren't under `vendor/` at all and are likely excluded by RuboCop's
/// file discovery or `.gitignore` handling. No cop-level fix needed.
///
/// ## Corpus investigation (2026-03-24)
///
/// FP=0, FN=4. All 4 FNs are in vendored gem paths:
///   - `heroku/ruby/1.9.1/gems/rdoc-*/lib/rdoc/method_attr.rb` (cjstewart88__Tubalr, 2 FNs)
///   - `vendor/bundle/ruby/2.3.0/gems/rdoc-4.3.0/lib/rdoc/method_attr.rb` (liaoziyang__stackneveroverflow, 1 FN)
///   - `examples/vendored-puppet/vendor/puppet-2.7.8/lib/puppet/util/settings.rb` (pitluga__supply_drop, 1 FN)
///
/// Patterns: `!searched.include?(kernel)` inside `&&` and `! %w{...}.include?(group)`
/// with space after `!`. Both patterns are correctly detected by the cop
/// (verified via fixture tests). Root cause is the same file-exclusion path
/// resolution issue as the 2026-03-19 FPs — RuboCop scans these vendored
/// files via `--force-exclusion`, but nitrocop's corpus path resolution
/// differs. No cop-level fix needed.
pub struct NegateInclude;

impl Cop for NegateInclude {
    fn name(&self) -> &'static str {
        "Rails/NegateInclude"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"!" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if inner_call.name().as_slice() != b"include?" {
            return;
        }

        // RuboCop uses `send` not `csend` — skip safe navigation (&.include?)
        if let Some(op) = inner_call.call_operator_loc() {
            if op.as_slice() == b"&." {
                return;
            }
        }

        // RuboCop: receiver must exist ($!nil?)
        if inner_call.receiver().is_none() {
            return;
        }

        // RuboCop: exactly one argument ($_)
        let arg_count = inner_call
            .arguments()
            .map(|a| a.arguments().len())
            .unwrap_or(0);
        if arg_count != 1 {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `exclude?` instead of `!include?`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NegateInclude, "cops/rails/negate_include");
}
