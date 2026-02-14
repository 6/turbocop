use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use std::collections::HashSet;

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

        let name = call.name().as_slice();
        if !is_example_group(name) {
            return Vec::new();
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

        // Collect let! names and check if they are referenced in the same scope
        let mut let_bang_decls: Vec<(Vec<u8>, usize, usize)> = Vec::new(); // (name, line, col)

        // Collect all identifiers used in examples, hooks, and other calls
        let mut used_names: HashSet<Vec<u8>> = HashSet::new();

        // Also check if any ancestor scope defines the same let! (override case)
        // We'll collect names from nested groups that override, skip those

        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                let m = c.name().as_slice();
                if m == b"let!" && c.receiver().is_none() {
                    if let Some(let_name) = extract_let_name(&c) {
                        let loc = c.location();
                        let (line, col) = source.offset_to_line_col(loc.start_offset());
                        let_bang_decls.push((let_name, line, col));
                    }
                } else {
                    // Collect all identifiers used in this statement
                    collect_identifiers(source, &stmt, &mut used_names);
                }
            } else {
                collect_identifiers(source, &stmt, &mut used_names);
            }
        }

        let mut diagnostics = Vec::new();
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

        diagnostics
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

fn collect_identifiers(source: &SourceFile, node: &ruby_prism::Node<'_>, names: &mut HashSet<Vec<u8>>) {
    // Look for bare method calls (receiverless calls that could reference let variables)
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none() {
            let name = call.name().as_slice();
            names.insert(name.to_vec());
        }
        // Recurse into receiver
        if let Some(recv) = call.receiver() {
            collect_identifiers(source, &recv, names);
        }
        // Recurse into arguments
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                collect_identifiers(source, &arg, names);
            }
        }
        // Recurse into block
        if let Some(block) = call.block() {
            if let Some(block_node) = block.as_block_node() {
                if let Some(body) = block_node.body() {
                    collect_identifiers(source, &body, names);
                }
            }
        }
    } else if let Some(stmts) = node.as_statements_node() {
        for stmt in stmts.body().iter() {
            collect_identifiers(source, &stmt, names);
        }
    } else if let Some(lvar) = node.as_local_variable_read_node() {
        names.insert(lvar.name().as_slice().to_vec());
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
