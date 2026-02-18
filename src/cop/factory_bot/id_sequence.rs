use crate::cop::factory_bot::{is_factory_bot_receiver, FACTORY_BOT_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct IdSequence;

impl Cop for IdSequence {
    fn name(&self) -> &'static str {
        "FactoryBot/IdSequence"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        FACTORY_BOT_DEFAULT_INCLUDE
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

        if call.name().as_slice() != b"sequence" {
            return Vec::new();
        }

        // Receiver must be nil or FactoryBot
        match call.receiver() {
            None => {}
            Some(recv) => {
                if !is_factory_bot_receiver(&recv) {
                    return Vec::new();
                }
            }
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // First argument must be :id symbol
        let first = &arg_list[0];
        let is_id = first
            .as_symbol_node()
            .map_or(false, |s| s.unescaped() == b"id");

        if !is_id {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Do not create a sequence for an id attribute".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IdSequence, "cops/factorybot/id_sequence");
}
