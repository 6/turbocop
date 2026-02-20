use crate::cop::factory_bot::{is_factory_bot_receiver, FACTORY_BOT_METHODS, FACTORY_BOT_SPEC_INCLUDE};
use crate::cop::util::is_rspec_example_group;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

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

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = SyntaxMethodsVisitor {
            cop: self,
            source,
            in_example_group: false,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct SyntaxMethodsVisitor<'a> {
    cop: &'a SyntaxMethods,
    source: &'a SourceFile,
    in_example_group: bool,
    diagnostics: Vec<Diagnostic>,
}

/// Check if a call node is an RSpec example group (describe/context/feature/etc.)
/// with the appropriate receiver (nil, RSpec, or ::RSpec).
fn is_spec_group_call(call: &ruby_prism::CallNode<'_>) -> bool {
    let method_name = call.name().as_slice();
    if !is_rspec_example_group(method_name) {
        return false;
    }

    // Receiver must be nil (bare call) or RSpec/::RSpec constant
    match call.receiver() {
        None => true,
        Some(recv) => {
            if let Some(cr) = recv.as_constant_read_node() {
                cr.name().as_slice() == b"RSpec"
            } else if let Some(cp) = recv.as_constant_path_node() {
                cp.parent().is_none()
                    && cp.name().map_or(false, |n| n.as_slice() == b"RSpec")
            } else {
                false
            }
        }
    }
}

impl<'pr> Visit<'pr> for SyntaxMethodsVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check if this call itself is a FactoryBot.method inside an example group
        if self.in_example_group {
            let method_name = node.name().as_slice();
            let method_str = std::str::from_utf8(method_name).unwrap_or("");
            if FACTORY_BOT_METHODS.contains(&method_str) {
                if let Some(recv) = node.receiver() {
                    if is_factory_bot_receiver(&recv) {
                        let recv_loc = recv.location();
                        let (line, column) =
                            self.source.offset_to_line_col(recv_loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            format!(
                                "Use `{}` from `FactoryBot::Syntax::Methods`.",
                                method_str
                            ),
                        ));
                    }
                }
            }
        }

        // Check if this call node with a block is an RSpec example group
        let enters_example_group = node.block().is_some() && is_spec_group_call(node);

        let was_eg = self.in_example_group;
        if enters_example_group {
            self.in_example_group = true;
        }
        ruby_prism::visit_call_node(self, node);
        self.in_example_group = was_eg;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SyntaxMethods, "cops/factorybot/syntax_methods");
}
