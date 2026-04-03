use crate::cop::shared::node_type::ARRAY_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03):
/// - FP=3: all in vim-node repo, `%W[#@dir/...]` uses `#@var` instance variable
///   interpolation which the cop didn't recognize (only checked for `#{`).
///   Fixed by also detecting `#@` (ivar/cvar) and `#$` (global) interpolation.
/// - FN=1: in rufo repo, `%W()` (empty array, 4 bytes) was skipped by the
///   `src_bytes.len() > 4` guard. Fixed by treating short arrays as no-interpolation.
/// - FP=1: in ankusa repo, `%W(... ain't ...)` contains a single quote. RuboCop's
///   `double_quotes_required?` treats strings containing `'` as requiring double
///   quotes, so it skips the offense. Fixed by also checking for single quotes.
pub struct RedundantCapitalW;

impl Cop for RedundantCapitalW {
    fn name(&self) -> &'static str {
        "Style/RedundantCapitalW"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE]
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
        let loc = node.location();
        let src_bytes = loc.as_slice();

        // Only check array nodes whose source starts with %W
        if !src_bytes.starts_with(b"%W") {
            return;
        }

        // Must be an array node
        if node.as_array_node().is_none() {
            return;
        }

        // Check if any element contains interpolation or special escape sequences
        {
            let needs_capital_w = if src_bytes.len() > 4 {
                let content = &src_bytes[3..src_bytes.len().saturating_sub(1)];
                // Check for all Ruby interpolation forms: #{expr}, #@ivar, #@@cvar, #$gvar
                let has_interpolation = content
                    .windows(2)
                    .any(|w| w[0] == b'#' && (w[1] == b'{' || w[1] == b'@' || w[1] == b'$'));
                let has_escape = content.contains(&b'\\');
                // RuboCop's `double_quotes_required?` also treats single quotes
                // in element source as requiring %W (since the element would need
                // double-quote wrapping if extracted). Match that behavior.
                let has_single_quote = content.contains(&b'\'');
                has_interpolation || has_escape || has_single_quote
            } else {
                false
            };

            if !needs_capital_w {
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not use `%W` unless interpolation is needed. If not, use `%w`.".to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    // Replace %W with %w (just the second byte)
                    corr.push(crate::correction::Correction {
                        start: loc.start_offset() + 1,
                        end: loc.start_offset() + 2,
                        replacement: "w".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diag.corrected = true;
                }
                diagnostics.push(diag);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantCapitalW, "cops/style/redundant_capital_w");
    crate::cop_autocorrect_fixture_tests!(RedundantCapitalW, "cops/style/redundant_capital_w");
}
