use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct RedundantBlockCall;

impl Cop for RedundantBlockCall {
    fn name(&self) -> &'static str {
        "Performance/RedundantBlockCall"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = DefVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct DefVisitor<'a, 'src> {
    cop: &'a RedundantBlockCall,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for DefVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        check_def(self.cop, self.source, node, &mut self.diagnostics);
        // Continue recursing into nested defs (they have their own scope,
        // handled by BlockCallFinder not descending into defs)
        ruby_prism::visit_def_node(self, node);
    }
}

/// Check a def node for a &blockarg parameter and block.call usage.
fn check_def(
    cop: &RedundantBlockCall,
    source: &SourceFile,
    def_node: &ruby_prism::DefNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Look for a &blockarg parameter
    let params = match def_node.parameters() {
        Some(p) => p,
        None => return,
    };

    let blockarg = match params.block() {
        Some(b) => b,
        None => return,
    };

    let blockarg_name = match blockarg.name() {
        Some(n) => n,
        None => return,
    };

    let arg_name = blockarg_name.as_slice();

    // Now look for <arg_name>.call in the body
    let body = match def_node.body() {
        Some(b) => b,
        None => return,
    };

    // Check if the block arg is reassigned in the body — if so, skip
    let mut reassign_finder = ReassignFinder {
        name: arg_name,
        found: false,
    };
    reassign_finder.visit(&body);
    if reassign_finder.found {
        return;
    }

    let mut call_finder = BlockCallFinder {
        cop,
        source,
        arg_name,
        diagnostics,
    };
    call_finder.visit(&body);
}

struct ReassignFinder<'a> {
    name: &'a [u8],
    found: bool,
}

impl<'pr> Visit<'pr> for ReassignFinder<'_> {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
        ruby_prism::visit_local_variable_write_node(self, node);
    }
}

struct BlockCallFinder<'a, 'src, 'd> {
    cop: &'a RedundantBlockCall,
    source: &'src SourceFile,
    arg_name: &'a [u8],
    diagnostics: &'d mut Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for BlockCallFinder<'_, '_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"call" {
            if let Some(recv) = node.receiver() {
                if let Some(local_var) = recv.as_local_variable_read_node() {
                    if local_var.name().as_slice() == self.arg_name {
                        // Don't flag if the call itself has a block literal
                        // (e.g., block.call { ... })
                        if node.block().is_none() || node.block().unwrap().as_block_node().is_none()
                        {
                            let loc = node.location();
                            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                            let msg = format!(
                                "Use `yield` instead of `{}.call`.",
                                std::str::from_utf8(self.arg_name).unwrap_or("block")
                            );
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                msg,
                            ));
                        }
                    }
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {
        // Don't descend into nested def nodes (they have their own scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantBlockCall, "cops/performance/redundant_block_call");
}
