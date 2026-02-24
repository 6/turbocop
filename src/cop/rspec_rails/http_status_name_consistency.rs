use crate::cop::node_type::{CALL_NODE, SYMBOL_NODE};
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // requires_gem 'rack', '>= 3.1.0' — only fire in Rails projects.
        // Non-Rails projects won't have TargetRailsVersion set.
        if !config.has_target_rails_version() {
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"have_http_status" {
            return;
        }

        if call.receiver().is_some() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let arg = &arg_list[0];
        let sym = match arg.as_symbol_node() {
            Some(s) => s,
            None => return,
        };

        let sym_name = sym.unescaped();
        let current = std::str::from_utf8(sym_name).unwrap_or("");

        if let Some(preferred) = preferred_status(sym_name) {
            let loc = arg.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Prefer `:{preferred}` over `:{current}`."),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_rails_fixture_tests!(
        HttpStatusNameConsistency,
        "cops/rspecrails/http_status_name_consistency",
        7.0
    );
}
