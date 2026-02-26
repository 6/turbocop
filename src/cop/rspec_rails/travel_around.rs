use ruby_prism::Visit;

use crate::cop::node_type::{
    BLOCK_ARGUMENT_NODE, BLOCK_NODE, CALL_NODE, STATEMENTS_NODE, SYMBOL_NODE,
};
use crate::cop::rspec_rails::RSPEC_RAILS_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct TravelAround;

const TRAVEL_METHODS: &[&[u8]] = &[b"freeze_time", b"travel", b"travel_to"];

impl Cop for TravelAround {
    fn name(&self) -> &'static str {
        "RSpecRails/TravelAround"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_RAILS_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_ARGUMENT_NODE,
            BLOCK_NODE,
            CALL_NODE,
            STATEMENTS_NODE,
            SYMBOL_NODE,
        ]
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
        // We look for `around` blocks and then check their body for travel patterns.
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"around" || call.receiver().is_some() {
            return;
        }

        // Check for around(:all) or around(:suite) - those are exempt
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if let Some(sym) = arg.as_symbol_node() {
                    let sym_name = sym.unescaped();
                    if sym_name == b"all" || sym_name == b"suite" {
                        return;
                    }
                }
            }
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        // Recursively search for travel calls anywhere in the around block body
        let mut finder = TravelFinder {
            offsets: Vec::new(),
        };
        finder.visit(&body);

        for offset in finder.offsets {
            let (line, column) = source.offset_to_line_col(offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer to travel in `before` rather than `around`.".to_string(),
            ));
        }
    }
}

struct TravelFinder {
    offsets: Vec<usize>,
}

impl<'pr> Visit<'pr> for TravelFinder {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();
        if TRAVEL_METHODS.contains(&name) && node.receiver().is_none() {
            if let Some(travel_block) = node.block() {
                // Pattern 1: travel_method do ... example.run ... end
                if let Some(block_node) = travel_block.as_block_node() {
                    if let Some(travel_body) = block_node.body() {
                        if let Some(stmts) = travel_body.as_statements_node() {
                            let stmt_list: Vec<_> = stmts.body().iter().collect();
                            if stmt_list.len() == 1 {
                                if let Some(run_call) = stmt_list[0].as_call_node() {
                                    if run_call.name().as_slice() == b"run" {
                                        self.offsets.push(node.location().start_offset());
                                    }
                                }
                            }
                        }
                    }
                }

                // Pattern 2: travel_method(&example)
                if travel_block.as_block_argument_node().is_some() {
                    self.offsets.push(node.location().start_offset());
                }
            }
        }

        // Continue visiting children to find nested travel calls
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TravelAround, "cops/rspecrails/travel_around");
}
