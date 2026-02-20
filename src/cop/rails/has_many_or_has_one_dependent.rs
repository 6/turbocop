use crate::cop::util::{class_body_calls, has_keyword_arg, is_dsl_call};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CLASS_NODE;

pub struct HasManyOrHasOneDependent;

impl Cop for HasManyOrHasOneDependent {
    fn name(&self) -> &'static str {
        "Rails/HasManyOrHasOneDependent"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE]
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
        let class = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        let calls = class_body_calls(&class);

        for call in &calls {
            let is_has_many = is_dsl_call(call, b"has_many");
            let is_has_one = is_dsl_call(call, b"has_one");

            if !is_has_many && !is_has_one {
                continue;
            }

            // Skip if :through is specified (through associations don't need :dependent)
            if has_keyword_arg(call, b"through") {
                continue;
            }

            // Flag if :dependent is missing
            if !has_keyword_arg(call, b"dependent") {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Specify a `:dependent` option.".to_string(),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        HasManyOrHasOneDependent,
        "cops/rails/has_many_or_has_one_dependent"
    );
}
