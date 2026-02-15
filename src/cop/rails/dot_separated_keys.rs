use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DotSeparatedKeys;

impl Cop for DotSeparatedKeys {
    fn name(&self) -> &'static str {
        "Rails/DotSeparatedKeys"
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

        let method_name = call.name().as_slice();
        if method_name != b"t" && method_name != b"translate" {
            return Vec::new();
        }

        // Receiver can be I18n or absent (Rails helper `t`)
        if let Some(recv) = call.receiver() {
            let is_i18n = recv
                .as_constant_read_node()
                .is_some_and(|c| c.name().as_slice() == b"I18n");
            if !is_i18n {
                return Vec::new();
            }
        }

        // Look for a `scope:` keyword argument — this cop flags scope-based keys
        // and suggests using dot-separated string keys instead.
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        for arg in args.arguments().iter() {
            let hash = if let Some(h) = arg.as_keyword_hash_node() {
                h.elements()
            } else {
                continue;
            };
            for elem in hash.iter() {
                let assoc = match elem.as_assoc_node() {
                    Some(a) => a,
                    None => continue,
                };
                let is_scope_key = if let Some(sym) = assoc.key().as_symbol_node() {
                    sym.unescaped() == b"scope"
                } else {
                    false
                };
                if is_scope_key {
                    let loc = assoc.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use dot-separated keys instead of the `:scope` option.".to_string(),
                    )];
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DotSeparatedKeys, "cops/rails/dot_separated_keys");
}
