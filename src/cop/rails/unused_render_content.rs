use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, INTEGER_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UnusedRenderContent;

const NON_CONTENT_SYMBOLS: &[&[u8]] = &[
    b"continue",
    b"switching_protocols",
    b"processing",
    b"no_content",
    b"reset_content",
    b"not_modified",
];

fn is_non_content_code(code: i64) -> bool {
    (100..=199).contains(&code) || code == 204 || code == 205 || code == 304
}

const BODY_OPTIONS: &[&[u8]] = &[
    b"action",
    b"body",
    b"content_type",
    b"file",
    b"html",
    b"inline",
    b"json",
    b"js",
    b"layout",
    b"plain",
    b"raw",
    b"template",
    b"text",
    b"xml",
];

impl Cop for UnusedRenderContent {
    fn name(&self) -> &'static str {
        "Rails/UnusedRenderContent"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ASSOC_NODE,
            CALL_NODE,
            INTEGER_NODE,
            KEYWORD_HASH_NODE,
            SYMBOL_NODE,
        ]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"render" {
            return;
        }

        if call.receiver().is_some() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let mut has_non_content_status = false;
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
                    // Check symbol status
                    if let Some(sym) = assoc.value().as_symbol_node() {
                        if NON_CONTENT_SYMBOLS.contains(&sym.unescaped()) {
                            has_non_content_status = true;
                        }
                    }
                    // Check numeric status
                    if let Some(_int) = assoc.value().as_integer_node() {
                        let int_loc = assoc.value().location();
                        let code_text = std::str::from_utf8(int_loc.as_slice()).unwrap_or("");
                        if let Ok(code_num) = code_text.parse::<i64>() {
                            if is_non_content_code(code_num) {
                                has_non_content_status = true;
                            }
                        }
                    }
                } else if BODY_OPTIONS.contains(&key_name) {
                    has_content_keys = true;
                }
            }
        }

        if has_non_content_status && has_content_keys {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(
                self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not specify body content for a response with a non-content status code"
                        .to_string(),
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnusedRenderContent, "cops/rails/unused_render_content");
}
