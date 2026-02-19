use crate::cop::util::is_dsl_call;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct Validation;

const OLD_VALIDATORS: &[(&[u8], &str)] = &[
    (b"validates_presence_of", "presence: true"),
    (b"validates_uniqueness_of", "uniqueness: true"),
    (b"validates_format_of", "format: { ... }"),
    (b"validates_length_of", "length: { ... }"),
    (b"validates_inclusion_of", "inclusion: { ... }"),
    (b"validates_exclusion_of", "exclusion: { ... }"),
    (b"validates_numericality_of", "numericality: true"),
    (b"validates_acceptance_of", "acceptance: true"),
    (b"validates_confirmation_of", "confirmation: true"),
    (b"validates_size_of", "length: { ... }"),
    (b"validates_comparison_of", "comparison: { ... }"),
    (b"validates_absence_of", "absence: true"),
];

impl Cop for Validation {
    fn name(&self) -> &'static str {
        "Rails/Validation"
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
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        for &(old_name, replacement) in OLD_VALIDATORS {
            if is_dsl_call(&call, old_name) {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let old_str = String::from_utf8_lossy(old_name);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Use `validates :attr, {replacement}` instead of `{old_str}`."
                    ),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Validation, "cops/rails/validation");
}
