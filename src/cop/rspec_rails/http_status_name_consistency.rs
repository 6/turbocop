use crate::cop::rspec_rails::RSPEC_RAILS_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct HttpStatusNameConsistency;

/// Mapping of deprecated status names to their preferred replacements.
fn preferred_status(sym: &[u8]) -> Option<&'static str> {
    match sym {
        b"unprocessable_entity" => Some("unprocessable_content"),
        b"payload_too_large" => Some("content_too_large"),
        _ => None,
    }
}

impl Cop for HttpStatusNameConsistency {
    fn name(&self) -> &'static str {
        "RSpecRails/HttpStatusNameConsistency"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_RAILS_DEFAULT_INCLUDE
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

        if call.name().as_slice() != b"have_http_status" {
            return Vec::new();
        }

        if call.receiver().is_some() {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        let arg = &arg_list[0];
        let sym = match arg.as_symbol_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let sym_name = sym.unescaped();
        let current = std::str::from_utf8(sym_name.as_ref()).unwrap_or("");

        if let Some(preferred) = preferred_status(&sym_name) {
            let loc = arg.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("Prefer `:{preferred}` over `:{current}`."),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        HttpStatusNameConsistency,
        "cops/rspec_rails/http_status_name_consistency"
    );
}
