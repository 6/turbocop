use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STRING_NODE};

pub struct Env;

impl Cop for Env {
    fn name(&self) -> &'static str {
        "Rails/Env"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE]
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

        if call.name().as_slice() != b"[]" {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Handle both ConstantReadNode (ENV) and ConstantPathNode (::ENV)
        if util::constant_name(&recv) != Some(b"ENV") {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let key_str = if let Some(s) = arg_list[0].as_string_node() {
            let u = s.unescaped();
            if u != b"RAILS_ENV" && u != b"RACK_ENV" {
                return;
            }
            String::from_utf8_lossy(u).to_string()
        } else {
            return;
        };
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `Rails.env` instead of `ENV['{key_str}']`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Env, "cops/rails/env");
}
