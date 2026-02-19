use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, FALSE_NODE, FLOAT_NODE, INTEGER_NODE, NIL_NODE, SYMBOL_NODE, TRUE_NODE};

pub struct BeEql;

impl Cop for BeEql {
    fn name(&self) -> &'static str {
        "RSpec/BeEql"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, FALSE_NODE, FLOAT_NODE, INTEGER_NODE, NIL_NODE, SYMBOL_NODE, TRUE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Detect eql(true), eql(false), eql(nil), eql(integer), eql(float), eql(:symbol)
        // Suggest using `be` instead. Only flags positive expectations (`.to`).
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" {
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

        let eql_call = match arg_list[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if eql_call.name().as_slice() != b"eql" || eql_call.receiver().is_some() {
            return Vec::new();
        }

        let eql_args = match eql_call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let eql_arg_list: Vec<_> = eql_args.arguments().iter().collect();
        if eql_arg_list.len() != 1 {
            return Vec::new();
        }

        let arg = &eql_arg_list[0];
        let is_primitive = arg.as_true_node().is_some()
            || arg.as_false_node().is_some()
            || arg.as_nil_node().is_some()
            || arg.as_integer_node().is_some()
            || arg.as_float_node().is_some()
            || arg.as_symbol_node().is_some();

        if !is_primitive {
            return Vec::new();
        }

        let loc = eql_call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Prefer `be` over `eql`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BeEql, "cops/rspec/be_eql");
}
