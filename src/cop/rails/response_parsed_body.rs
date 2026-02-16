use crate::cop::util;
use crate::cop::{Cop, CopConfig, EnabledState};
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

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // This cop is pending/unsafe in RuboCop (Enabled: pending, Safe: false).
        // Only fire when explicitly enabled in the project config.
        if config.enabled != EnabledState::True {
            return Vec::new();
        }
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"parse" {
            return Vec::new();
        }

        // Receiver must be constant `JSON`
        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        // Handle both ConstantReadNode (JSON) and ConstantPathNode (::JSON)
        if util::constant_name(&recv) != Some(b"JSON") {
            return Vec::new();
        }

        // First argument should be `response.body`
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let arg_call = match arg_list[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if arg_call.name().as_slice() != b"body" {
            return Vec::new();
        }

        // The receiver of .body should be `response`
        let body_recv = match arg_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let body_recv_call = match body_recv.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if body_recv_call.name().as_slice() != b"response" {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `response.parsed_body` instead of `JSON.parse(response.body)`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_config() -> CopConfig {
        CopConfig {
            enabled: EnabledState::True,
            ..CopConfig::default()
        }
    }

    #[test]
    fn offense_fixture() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &ResponseParsedBody,
            include_bytes!("../../../testdata/cops/rails/response_parsed_body/offense.rb"),
            enabled_config(),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &ResponseParsedBody,
            include_bytes!("../../../testdata/cops/rails/response_parsed_body/no_offense.rb"),
            enabled_config(),
        );
    }
}
