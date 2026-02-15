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

fn symbol_to_status_code(sym: &[u8]) -> Option<i64> {
    match sym {
        b"continue" => Some(100),
        b"ok" => Some(200),
        b"created" => Some(201),
        b"accepted" => Some(202),
        b"no_content" => Some(204),
        b"moved_permanently" => Some(301),
        b"found" => Some(302),
        b"see_other" => Some(303),
        b"not_modified" => Some(304),
        b"temporary_redirect" => Some(307),
        b"bad_request" => Some(400),
        b"unauthorized" => Some(401),
        b"forbidden" => Some(403),
        b"not_found" => Some(404),
        b"method_not_allowed" => Some(405),
        b"not_acceptable" => Some(406),
        b"request_timeout" => Some(408),
        b"conflict" => Some(409),
        b"gone" => Some(410),
        b"unprocessable_entity" => Some(422),
        b"too_many_requests" => Some(429),
        b"internal_server_error" => Some(500),
        b"bad_gateway" => Some(502),
        b"service_unavailable" => Some(503),
        _ => None,
    }
}

/// Permitted symbols that are not specific status codes (used by numeric style).
const PERMITTED_SYMBOLS: &[&[u8]] = &[b"error", b"success", b"missing", b"redirect"];

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
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "symbolic");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if !STATUS_METHODS.contains(&call.name().as_slice()) {
            return Vec::new();
        }

        if let Some(status_value) = keyword_arg_value(&call, b"status") {
            match style {
                "numeric" => {
                    // Flag symbolic statuses, suggest numeric
                    if let Some(sym) = status_value.as_symbol_node() {
                        let sym_name = sym.unescaped();
                        if PERMITTED_SYMBOLS.contains(&sym_name.as_ref()) {
                            return Vec::new();
                        }
                        if let Some(code) = symbol_to_status_code(&sym_name) {
                            let sym_str = std::str::from_utf8(&sym_name).unwrap_or("?");
                            let loc = node.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            return vec![self.diagnostic(
                                source,
                                line,
                                column,
                                format!(
                                    "Use numeric HTTP status `{code}` instead of `:{sym_str}`."
                                ),
                            )];
                        }
                    }
                }
                _ => {
                    // "symbolic" (default): flag numeric statuses, suggest symbolic
                    if status_value.as_integer_node().is_some() {
                        let int_loc = status_value.location();
                        let code_text = std::str::from_utf8(int_loc.as_slice()).unwrap_or("");
                        if let Ok(code_num) = code_text.parse::<i64>() {
                            if let Some(sym) = status_code_to_symbol(code_num) {
                                let loc = node.location();
                                let (line, column) =
                                    source.offset_to_line_col(loc.start_offset());
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
            }
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HttpStatus, "cops/rails/http_status");

    #[test]
    fn numeric_style_flags_symbolic_status() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("numeric".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"render :foo, status: :ok\n";
        let diags = run_cop_full_with_config(&HttpStatus, source, config);
        assert!(!diags.is_empty(), "numeric style should flag symbolic :ok");
        assert!(diags[0].message.contains("200"));
    }

    #[test]
    fn numeric_style_allows_numeric_status() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("numeric".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"render :foo, status: 200\n";
        assert_cop_no_offenses_full_with_config(&HttpStatus, source, config);
    }

    #[test]
    fn numeric_style_permits_generic_symbols() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("numeric".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"render :foo, status: :error\n";
        assert_cop_no_offenses_full_with_config(&HttpStatus, source, config);
    }
}
