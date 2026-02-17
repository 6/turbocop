use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedirectBackOrTo;

impl Cop for RedirectBackOrTo {
    fn name(&self) -> &'static str {
        "Rails/RedirectBackOrTo"
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

        // Must be receiverless `redirect_back`
        if call.receiver().is_some() || call.name().as_slice() != b"redirect_back" {
            return Vec::new();
        }

        // Must have `fallback_location:` keyword argument
        if keyword_arg_value(&call, b"fallback_location").is_none() {
            return Vec::new();
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `redirect_back_or_to` instead of `redirect_back` with `:fallback_location` keyword argument.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedirectBackOrTo, "cops/rails/redirect_back_or_to");
}
