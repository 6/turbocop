use crate::cop::rspec_rails::RSPEC_RAILS_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, BLOCK_NODE, CALL_NODE, STATEMENTS_NODE, SYMBOL_NODE};

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
        &[BLOCK_ARGUMENT_NODE, BLOCK_NODE, CALL_NODE, STATEMENTS_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // We look for `around` blocks and then check their body for travel patterns.
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"around" || call.receiver().is_some() {
            return Vec::new();
        }

        // Check for around(:all) or around(:suite) - those are exempt
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if let Some(sym) = arg.as_symbol_node() {
                    let sym_name = sym.unescaped();
                    if sym_name == b"all" || sym_name == b"suite" {
                        return Vec::new();
                    }
                }
            }
        }

        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();

        for stmt in stmts.body().iter() {
            if let Some(travel_call) = stmt.as_call_node() {
                let travel_name = travel_call.name().as_slice();
                if !TRAVEL_METHODS.iter().any(|m| *m == travel_name) {
                    continue;
                }
                if travel_call.receiver().is_some() {
                    continue;
                }

                let travel_block = match travel_call.block() {
                    Some(b) => b,
                    None => continue,
                };

                // Pattern 1: travel_method do ... example.run ... end
                if let Some(travel_block_node) = travel_block.as_block_node() {
                    if let Some(travel_body) = travel_block_node.body() {
                        if let Some(travel_stmts) = travel_body.as_statements_node() {
                            let stmt_list: Vec<_> = travel_stmts.body().iter().collect();
                            if stmt_list.len() == 1 {
                                if let Some(run_call) = stmt_list[0].as_call_node() {
                                    if run_call.name().as_slice() == b"run" {
                                        let loc = travel_call.location();
                                        let (line, column) =
                                            source.offset_to_line_col(loc.start_offset());
                                        diagnostics.push(self.diagnostic(
                                            source,
                                            line,
                                            column,
                                            "Prefer to travel in `before` rather than `around`.".to_string(),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }

                // Pattern 2: travel_method(&example)
                if travel_block.as_block_argument_node().is_some() {
                    let loc = travel_call.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Prefer to travel in `before` rather than `around`.".to_string(),
                    ));
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TravelAround, "cops/rspecrails/travel_around");
}
