use crate::cop::node_type::{CALL_NODE, SYMBOL_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RemoveConst;

impl Cop for RemoveConst {
    fn name(&self) -> &'static str {
        "RSpec/RemoveConst"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, SYMBOL_NODE]
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
        // Look for .send(:remove_const, ...) or .__send__(:remove_const, ...)
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = call.name().as_slice();
        if method != b"send" && method != b"__send__" {
            return;
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // First arg must be :remove_const
        let is_remove_const = if let Some(sym) = arg_list[0].as_symbol_node() {
            sym.unescaped() == b"remove_const"
        } else {
            false
        };

        if !is_remove_const {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Do not use remove_const in specs. Consider using e.g. `stub_const`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RemoveConst, "cops/rspec/remove_const");
}
