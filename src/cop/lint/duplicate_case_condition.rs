use std::collections::HashSet;

use crate::cop::shared::node_type::{CASE_NODE, WHEN_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus FN=5 came from comparing `when` conditions by raw source bytes instead
/// of Prism's semantic literal value. That missed escape-equivalent strings like
/// `"\""` vs `'"'` and `"\C-m"` vs `"\r"`, which RuboCop treats as duplicates.
/// Fix: canonicalize string/symbol condition keys by unescaped bytes and fall
/// back to source text for non-literal expressions to keep the change narrow.
pub struct DuplicateCaseCondition;

impl Cop for DuplicateCaseCondition {
    fn name(&self) -> &'static str {
        "Lint/DuplicateCaseCondition"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CASE_NODE, WHEN_NODE]
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
        let case_node = match node.as_case_node() {
            Some(n) => n,
            None => return,
        };

        let mut seen = HashSet::new();

        for when_node_ref in case_node.conditions().iter() {
            let when_node = match when_node_ref.as_when_node() {
                Some(w) => w,
                None => continue,
            };
            for condition in when_node.conditions().iter() {
                let loc = condition.location();
                if !seen.insert(condition_key(&condition)) {
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Duplicate `when` condition detected.".to_string(),
                    ));
                }
            }
        }
    }
}

fn condition_key(condition: &ruby_prism::Node<'_>) -> Vec<u8> {
    if let Some(string) = condition.as_string_node() {
        let mut key = Vec::with_capacity(4 + string.unescaped().len());
        key.extend_from_slice(b"str:");
        key.extend_from_slice(string.unescaped());
        return key;
    }

    if let Some(symbol) = condition.as_symbol_node() {
        let mut key = Vec::with_capacity(4 + symbol.unescaped().len());
        key.extend_from_slice(b"sym:");
        key.extend_from_slice(symbol.unescaped());
        return key;
    }

    let source_text = condition.location().as_slice();
    let mut key = Vec::with_capacity(4 + source_text.len());
    key.extend_from_slice(b"src:");
    key.extend_from_slice(source_text);
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateCaseCondition, "cops/lint/duplicate_case_condition");
}
