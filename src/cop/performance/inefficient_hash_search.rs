use crate::cop::shared::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Extended corpus investigation (2026-03-24)
///
/// Extended corpus reported FP=3, FN=0. All 3 FPs from files containing
/// invalid multibyte regex escapes that crash RuboCop's parser, causing all
/// other cops to be skipped. Not a cop logic issue. Fixed by adding the
/// affected files to `repo_excludes.json`.
pub struct InefficientHashSearch;

impl Cop for InefficientHashSearch {
    fn name(&self) -> &'static str {
        "Performance/InefficientHashSearch"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        if chain.outer_method != b"include?" {
            return;
        }

        // inner_call must have an explicit receiver (e.g. `hash.keys`, not bare `keys`)
        // Bare `keys`/`values` without a receiver are often methods on non-Hash classes.
        if chain.inner_call.receiver().is_none() {
            return;
        }

        // inner_call must have no arguments (just `.keys` or `.values`)
        if chain.inner_call.arguments().is_some() {
            return;
        }

        let message = if chain.inner_method == b"keys" {
            "Use `key?` instead of `keys.include?`."
        } else if chain.inner_method == b"values" {
            "Use `value?` instead of `values.include?`."
        } else {
            return;
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, message.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        InefficientHashSearch,
        "cops/performance/inefficient_hash_search"
    );
}
