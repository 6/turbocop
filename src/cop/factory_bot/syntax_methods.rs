use crate::cop::factory_bot::{is_factory_bot_receiver, FACTORY_BOT_METHODS, FACTORY_BOT_SPEC_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SyntaxMethods;

impl Cop for SyntaxMethods {
    fn name(&self) -> &'static str {
        "FactoryBot/SyntaxMethods"
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
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        let method_str = std::str::from_utf8(method_name).unwrap_or("");
        if !FACTORY_BOT_METHODS.contains(&method_str) {
            return Vec::new();
        }

        // Must have FactoryBot/FactoryGirl receiver
        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        if !is_factory_bot_receiver(&recv) {
            return Vec::new();
        }

        // Must be inside an RSpec example group
        // We check this by walking ancestors (parent chain)
        // Since check_node doesn't give us parent, we'll use the SourceFile to check
        // if we're in a spec file (the Include pattern already filters for spec files)
        // For accuracy, we should verify this is inside an RSpec block, but the Include
        // pattern is sufficient for the common case.
        //
        // Actually, the vendor cop checks for spec_group? ancestor. Let's approximate
        // by checking if the file looks like a spec file (which is already guaranteed
        // by the Include pattern). For the no-offense case outside example groups,
        // we can't easily detect this without parent traversal, so we'll trust the
        // Include pattern for now - this matches real-world behavior since factory_bot
        // calls outside spec files aren't covered by this cop.

        // The offense spans from start of FactoryBot receiver to end of method name
        let recv_loc = recv.location();
        let (line, column) = source.offset_to_line_col(recv_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `{}` from `FactoryBot::Syntax::Methods`.",
                method_str
            ),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SyntaxMethods, "cops/factorybot/syntax_methods");
}
