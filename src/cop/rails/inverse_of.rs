use crate::cop::util::{class_body_calls, has_keyword_arg, is_dsl_call};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct InverseOf;

impl Cop for InverseOf {
    fn name(&self) -> &'static str {
        "Rails/InverseOf"
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

        let mut diagnostics = Vec::new();
        let calls = class_body_calls(&class);

        for call in &calls {
            let is_assoc = is_dsl_call(call, b"has_many")
                || is_dsl_call(call, b"has_one")
                || is_dsl_call(call, b"belongs_to");

            if !is_assoc {
                continue;
            }

            // Only flag when :foreign_key or :as is specified without :inverse_of
            let has_foreign_key = has_keyword_arg(call, b"foreign_key");
            let has_as = has_keyword_arg(call, b"as");

            if (has_foreign_key || has_as) && !has_keyword_arg(call, b"inverse_of") {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Specify an `:inverse_of` option.".to_string(),
                ));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InverseOf, "cops/rails/inverse_of");
}
