use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct BeNil;

impl Cop for BeNil {
    fn name(&self) -> &'static str {
        "RSpec/BeNil"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "be_nil");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
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

        let matcher_call = match arg_list[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if matcher_call.receiver().is_some() {
            return Vec::new();
        }

        let matcher_name = matcher_call.name().as_slice();

        if enforced_style == "be_nil" {
            // Flag `be(nil)` — prefer `be_nil`
            if matcher_name != b"be" {
                return Vec::new();
            }
            let matcher_args = match matcher_call.arguments() {
                Some(a) => a,
                None => return Vec::new(),
            };
            let matcher_arg_list: Vec<_> = matcher_args.arguments().iter().collect();
            if matcher_arg_list.len() != 1 || matcher_arg_list[0].as_nil_node().is_none() {
                return Vec::new();
            }
            let loc = matcher_call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            vec![self.diagnostic(
                source,
                line,
                column,
                "Prefer `be_nil` over `be(nil)`.".to_string(),
            )]
        } else {
            // Flag `be_nil` — prefer `be(nil)`
            if matcher_name != b"be_nil" {
                return Vec::new();
            }
            if matcher_call.arguments().is_some() {
                return Vec::new();
            }
            let loc = matcher_call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            vec![self.diagnostic(
                source,
                line,
                column,
                "Prefer `be(nil)` over `be_nil`.".to_string(),
            )]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BeNil, "cops/rspec/be_nil");
}
