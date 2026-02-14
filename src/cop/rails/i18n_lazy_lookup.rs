use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct I18nLazyLookup;

impl Cop for I18nLazyLookup {
    fn name(&self) -> &'static str {
        "Rails/I18nLazyLookup"
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

        let method = call.name().as_slice();

        // Match I18n.t("key") or bare t("key")
        let is_i18n_t = if method == b"t" {
            if let Some(recv) = call.receiver() {
                // I18n.t
                recv.as_constant_read_node()
                    .is_some_and(|c| c.name().as_slice() == b"I18n")
            } else {
                // bare t()
                true
            }
        } else {
            false
        };

        if !is_i18n_t {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // Check if first arg is a string with 3+ dot-separated segments
        if let Some(s) = arg_list[0].as_string_node() {
            let key = s.unescaped();
            let dot_count = key.iter().filter(|&&b| b == b'.').count();
            if dot_count < 2 {
                return Vec::new();
            }
        } else {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use lazy lookup for i18n keys.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(I18nLazyLookup, "cops/rails/i18n_lazy_lookup");
}
