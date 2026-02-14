use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct HttpStatus;

fn status_code_to_symbol(code: i64) -> Option<&'static str> {
    match code {
        100 => Some(":continue"),
        200 => Some(":ok"),
        201 => Some(":created"),
        202 => Some(":accepted"),
        204 => Some(":no_content"),
        301 => Some(":moved_permanently"),
        302 => Some(":found"),
        303 => Some(":see_other"),
        304 => Some(":not_modified"),
        307 => Some(":temporary_redirect"),
        400 => Some(":bad_request"),
        401 => Some(":unauthorized"),
        403 => Some(":forbidden"),
        404 => Some(":not_found"),
        405 => Some(":method_not_allowed"),
        406 => Some(":not_acceptable"),
        408 => Some(":request_timeout"),
        409 => Some(":conflict"),
        410 => Some(":gone"),
        422 => Some(":unprocessable_entity"),
        429 => Some(":too_many_requests"),
        500 => Some(":internal_server_error"),
        502 => Some(":bad_gateway"),
        503 => Some(":service_unavailable"),
        _ => None,
    }
}

const STATUS_METHODS: &[&[u8]] = &[b"render", b"head", b"redirect_to"];

impl Cop for HttpStatus {
    fn name(&self) -> &'static str {
        "Rails/HttpStatus"
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
        if !STATUS_METHODS.contains(&call.name().as_slice()) {
            return Vec::new();
        }
        if let Some(status_value) = keyword_arg_value(&call, b"status") {
            if status_value.as_integer_node().is_some() {
                let int_loc = status_value.location();
                let code_text = std::str::from_utf8(int_loc.as_slice()).unwrap_or("");
                if let Ok(code_num) = code_text.parse::<i64>() {
                    if let Some(sym) = status_code_to_symbol(code_num) {
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            format!(
                                "Use symbolic HTTP status `{sym}` instead of `{code_num}`."
                            ),
                        )];
                    }
                }
            }
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HttpStatus, "cops/rails/http_status");
}
