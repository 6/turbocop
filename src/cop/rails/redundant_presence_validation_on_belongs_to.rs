use crate::cop::util::{class_body_calls, has_keyword_arg, is_dsl_call};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantPresenceValidationOnBelongsTo;

impl Cop for RedundantPresenceValidationOnBelongsTo {
    fn name(&self) -> &'static str {
        "Rails/RedundantPresenceValidationOnBelongsTo"
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
    ) -> Vec<Diagnostic> {
        let class = match node.as_class_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let calls = class_body_calls(&class);

        // Collect belongs_to association names
        let mut belongs_to_names: Vec<Vec<u8>> = Vec::new();
        for call in &calls {
            if is_dsl_call(call, b"belongs_to") {
                if let Some(name) = extract_first_symbol_arg(call) {
                    belongs_to_names.push(name);
                }
            }
        }

        if belongs_to_names.is_empty() {
            return Vec::new();
        }

        // Check validates calls for presence on belongs_to associations
        let mut diagnostics = Vec::new();
        for call in &calls {
            if !is_dsl_call(call, b"validates") {
                continue;
            }

            if !has_keyword_arg(call, b"presence") {
                continue;
            }

            // Get the first symbol arg (the validated attribute name)
            if let Some(name) = extract_first_symbol_arg(call) {
                if belongs_to_names.contains(&name) {
                    let loc = call.message_loc().unwrap_or(call.location());
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let name_str = String::from_utf8_lossy(&name);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Remove explicit `presence` validation for `{name_str}` `belongs_to` association (validated by default since Rails 5)."),
                    ));
                }
            }
        }

        diagnostics
    }
}

fn extract_first_symbol_arg(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let first_arg = args.arguments().iter().next()?;
    let sym = first_arg.as_symbol_node()?;
    Some(sym.unescaped().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantPresenceValidationOnBelongsTo,
        "cops/rails/redundant_presence_validation_on_belongs_to"
    );
}
