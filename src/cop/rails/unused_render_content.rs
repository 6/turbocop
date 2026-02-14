use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UnusedRenderContent;

const HEAD_ONLY_STATUSES: &[&[u8]] = &[
    b"no_content", b"not_modified", b"reset_content",
];

impl Cop for UnusedRenderContent {
    fn name(&self) -> &'static str {
        "Rails/UnusedRenderContent"
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

        if call.name().as_slice() != b"render" {
            return Vec::new();
        }

        if call.receiver().is_some() {
            return Vec::new();
        }

        // Check if there's a `status:` keyword arg with a head-only status
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let mut has_head_only_status = false;
        let mut has_content_keys = false;

        for arg in args.arguments().iter() {
            let kw = match arg.as_keyword_hash_node() {
                Some(k) => k,
                None => continue,
            };
            for elem in kw.elements().iter() {
                let assoc = match elem.as_assoc_node() {
                    Some(a) => a,
                    None => continue,
                };
                let key = match assoc.key().as_symbol_node() {
                    Some(s) => s,
                    None => continue,
                };
                let key_name = key.unescaped();
                if key_name == b"status" {
                    // Check if value is a head-only symbol
                    if let Some(sym) = assoc.value().as_symbol_node() {
                        if HEAD_ONLY_STATUSES.contains(&sym.unescaped().as_ref()) {
                            has_head_only_status = true;
                        }
                    }
                } else if key_name == b"json" || key_name == b"html" || key_name == b"body"
                    || key_name == b"plain" || key_name == b"xml"
                {
                    has_content_keys = true;
                }
            }
        }

        if has_head_only_status && has_content_keys {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Do not pass content to `render` with a head-only status.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnusedRenderContent, "cops/rails/unused_render_content");
}
