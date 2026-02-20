use crate::cop::factory_bot::{is_factory_call, FACTORY_BOT_SPEC_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, INTEGER_NODE, STRING_NODE, SYMBOL_NODE};

pub struct ExcessiveCreateList;

impl Cop for ExcessiveCreateList {
    fn name(&self) -> &'static str {
        "FactoryBot/ExcessiveCreateList"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        FACTORY_BOT_SPEC_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTEGER_NODE, STRING_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"create_list" {
            return;
        }

        let explicit_only = config.get_bool("ExplicitOnly", false);
        if !is_factory_call(call.receiver(), explicit_only) {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() < 2 {
            return;
        }

        // First arg must be symbol or string
        if arg_list[0].as_symbol_node().is_none() && arg_list[0].as_string_node().is_none() {
            return;
        }

        // Second arg must be an integer
        let count = match arg_list[1].as_integer_node() {
            Some(int) => {
                let src = &source.as_bytes()
                    [int.location().start_offset()..int.location().end_offset()];
                match std::str::from_utf8(src).ok().and_then(|s| s.parse::<i64>().ok()) {
                    Some(v) => v,
                    None => return,
                }
            }
            None => return,
        };

        let max_amount = config.get_usize("MaxAmount", 10) as i64;

        if count <= max_amount {
            return;
        }

        let count_loc = arg_list[1].location();
        let (line, column) = source.offset_to_line_col(count_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Avoid using `create_list` with more than {} items.",
                max_amount
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ExcessiveCreateList,
        "cops/factorybot/excessive_create_list"
    );
}
