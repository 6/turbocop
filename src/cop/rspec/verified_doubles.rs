use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct VerifiedDoubles;

/// Flags `double("Name")` and `spy("Name")` — prefer verified doubles
/// like `instance_double`, `class_double`, etc.
impl Cop for VerifiedDoubles {
    fn name(&self) -> &'static str {
        "RSpec/VerifiedDoubles"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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

        let method_name = call.name().as_slice();
        if method_name != b"double" && method_name != b"spy" {
            return Vec::new();
        }

        // Must be receiverless
        if call.receiver().is_some() {
            return Vec::new();
        }

        // Default: ignore nameless doubles (no args, or only keyword args)
        let has_name_arg = if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.is_empty() {
                false
            } else {
                // First positional arg should be a string or symbol (the name)
                let first = &arg_list[0];
                first.as_string_node().is_some() || first.as_symbol_node().is_some()
            }
        } else {
            false
        };

        if !has_name_arg {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Prefer using verifying doubles over normal doubles.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(VerifiedDoubles, "cops/rspec/verified_doubles");
}
