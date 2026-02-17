use crate::cop::factory_bot::{is_factory_call, FACTORY_BOT_SPEC_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

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

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"create_list" {
            return Vec::new();
        }

        let explicit_only = config.get_bool("ExplicitOnly", false);
        if !is_factory_call(call.receiver(), explicit_only) {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() < 2 {
            return Vec::new();
        }

        // First arg must be symbol or string
        if arg_list[0].as_symbol_node().is_none() && arg_list[0].as_string_node().is_none() {
            return Vec::new();
        }

        // Second arg must be an integer
        let count = match arg_list[1].as_integer_node() {
            Some(int) => {
                let val: i64 = int.value().try_into().unwrap_or(0);
                val
            }
            None => return Vec::new(),
        };

        let max_amount = config.get_usize("MaxAmount", 10) as i64;

        if count <= max_amount {
            return Vec::new();
        }

        let count_loc = arg_list[1].location();
        let (line, column) = source.offset_to_line_col(count_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!(
                "Avoid using `create_list` with more than {} items.",
                max_amount
            ),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ExcessiveCreateList,
        "cops/factory_bot/excessive_create_list"
    );
}
