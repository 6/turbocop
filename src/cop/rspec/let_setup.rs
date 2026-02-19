use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashSet;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE};

/// RSpec/LetSetup: Flag `let!` that is not referenced in tests (only used for side effects).
pub struct LetSetup;

impl Cop for LetSetup {
    fn name(&self) -> &'static str {
        "RSpec/LetSetup"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();
        if !is_example_group(name) {
            return;
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
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        // Collect let! names and all identifiers used in the same scope
        let mut let_bang_decls: Vec<(Vec<u8>, usize, usize)> = Vec::new();
        let mut used_names: HashSet<Vec<u8>> = HashSet::new();

        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                let m = c.name().as_slice();
                if m == b"let!" && c.receiver().is_none() {
                    if let Some(let_name) = extract_let_name(&c) {
                        let loc = c.location();
                        let (line, col) = source.offset_to_line_col(loc.start_offset());
                        let_bang_decls.push((let_name, line, col));
                    }
                }
            }
            // Walk ALL siblings (including let! bodies) for identifier
            // collection. This matches RuboCop behavior where method_called?
            // searches the entire example group block, so a let! name used
            // inside a sibling let! body is not flagged.
            let mut collector = IdentifierCollector {
                names: &mut used_names,
            };
            collector.visit(&stmt);
        }

        for (let_name, line, col) in &let_bang_decls {
            if !used_names.contains(let_name) {
                diagnostics.push(self.diagnostic(
                    source,
                    *line,
                    *col,
                    "Do not use `let!` to setup objects not referenced in tests.".to_string(),
                ));
            }
        }

    }
}

fn extract_let_name(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let first = args.arguments().iter().next()?;
    if let Some(sym) = first.as_symbol_node() {
        return Some(sym.unescaped().to_vec());
    }
    if let Some(s) = first.as_string_node() {
        return Some(s.unescaped().to_vec());
    }
    None
}

/// Walks the entire AST subtree, collecting all receiverless call names and
/// local variable reads. This ensures `let!` references are found in any
/// nested structure (conditionals, blocks, string interpolations, etc.).
struct IdentifierCollector<'a> {
    names: &'a mut HashSet<Vec<u8>>,
}

impl<'pr> Visit<'pr> for IdentifierCollector<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.receiver().is_none() {
            self.names.insert(node.name().as_slice().to_vec());
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_local_variable_read_node(
        &mut self,
        node: &ruby_prism::LocalVariableReadNode<'pr>,
    ) {
        self.names.insert(node.name().as_slice().to_vec());
        ruby_prism::visit_local_variable_read_node(self, node);
    }
}

fn is_example_group(name: &[u8]) -> bool {
    matches!(
        name,
        b"describe"
            | b"context"
            | b"feature"
            | b"example_group"
            | b"xdescribe"
            | b"xcontext"
            | b"xfeature"
            | b"fdescribe"
            | b"fcontext"
            | b"ffeature"
            | b"shared_context"
            | b"shared_examples"
            | b"shared_examples_for"
            | b"it_behaves_like"
            | b"it_should_behave_like"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(LetSetup, "cops/rspec/let_setup");
}
