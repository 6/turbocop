use crate::cop::util::{has_keyword_arg, is_dsl_call};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantAllowNil;

impl Cop for RedundantAllowNil {
    fn name(&self) -> &'static str {
        "Rails/RedundantAllowNil"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if !is_dsl_call(&call, b"validates") {
            return Vec::new();
        }

        if has_keyword_arg(&call, b"presence") && has_keyword_arg(&call, b"allow_nil") {
            let loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Remove redundant `allow_nil` when `presence` validation is also specified."
                    .to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantAllowNil, "cops/rails/redundant_allow_nil");
}
