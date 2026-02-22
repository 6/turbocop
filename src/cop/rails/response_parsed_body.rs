use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE};
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ResponseParsedBody;

impl Cop for ResponseParsedBody {
    fn name(&self) -> &'static str {
        "Rails/ResponseParsedBody"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &[
            "spec/controllers/**/*.rb",
            "spec/requests/**/*.rb",
            "test/controllers/**/*.rb",
            "test/integration/**/*.rb",
        ]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE]
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

        if call.name().as_slice() != b"parse" {
            return;
        }

        // Must have exactly 1 argument (response.body) — no keyword args or extra args.
        // RuboCop's node pattern requires exactly one argument:
        //   (send (const {nil? cbase} :JSON) :parse (send (send nil? :response) :body))
        // If there are additional arguments (e.g., symbolize_names: true), it does NOT match.
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let arg_call = match arg_list[0].as_call_node() {
            Some(c) => c,
            None => return,
        };
        if arg_call.name().as_slice() != b"body" {
            return;
        }

        // The receiver of .body should be `response`
        let body_recv = match arg_call.receiver() {
            Some(r) => r,
            None => return,
        };
        let body_recv_call = match body_recv.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if body_recv_call.name().as_slice() != b"response" {
            return;
        }

        // Receiver must be constant `JSON` or `Nokogiri::HTML`/`Nokogiri::HTML5`
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Check for JSON.parse(response.body)
        if util::constant_name(&recv) == Some(b"JSON") {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer `response.parsed_body` to `JSON.parse(response.body)`.".to_string(),
            ));
        }

        // Check for Nokogiri::HTML.parse(response.body) / Nokogiri::HTML5.parse(response.body)
        if let Some(cp) = recv.as_constant_path_node() {
            if let Some(name) = cp.name() {
                let name_bytes = name.as_slice();
                if name_bytes == b"HTML" || name_bytes == b"HTML5" {
                    if let Some(parent) = cp.parent() {
                        if util::constant_name(&parent) == Some(b"Nokogiri") {
                            let const_name = std::str::from_utf8(name_bytes).unwrap_or("HTML");
                            let loc = node.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!("Prefer `response.parsed_body` to `Nokogiri::{const_name}.parse(response.body)`."),
                            ));
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(ResponseParsedBody, "cops/rails/response_parsed_body");
}
