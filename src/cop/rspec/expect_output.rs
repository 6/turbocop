use crate::cop::node_type::{
    BEGIN_NODE, BLOCK_NODE, CALL_NODE, DEF_NODE, ELSE_NODE, GLOBAL_VARIABLE_WRITE_NODE, IF_NODE,
    PROGRAM_NODE, STATEMENTS_NODE, SYMBOL_NODE,
};
use crate::cop::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_hook};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ExpectOutput;

impl Cop for ExpectOutput {
    fn name(&self) -> &'static str {
        "RSpec/ExpectOutput"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BEGIN_NODE,
            BLOCK_NODE,
            CALL_NODE,
            DEF_NODE,
            ELSE_NODE,
            GLOBAL_VARIABLE_WRITE_NODE,
            IF_NODE,
            PROGRAM_NODE,
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
        // Only process at program level to walk the full AST with context
        let program = match node.as_program_node() {
            Some(p) => p,
            None => return,
        };

        let mut visitor = ExpectOutputVisitor {
            source,
            diagnostics: Vec::new(),
            in_example_scope: false,
        };
        visitor.visit(&program.statements().body().iter().collect::<Vec<_>>()[..]);
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ExpectOutputVisitor<'a> {
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    in_example_scope: bool,
}

impl<'a> ExpectOutputVisitor<'a> {
    fn visit(&mut self, nodes: &[ruby_prism::Node<'_>]) {
        for node in nodes {
            self.visit_node(node);
        }
    }

    fn visit_node(&mut self, node: &ruby_prism::Node<'_>) {
        // Check for $stdout/$stderr assignments only when inside example scope
        if let Some(gvw) = node.as_global_variable_write_node() {
            if self.in_example_scope {
                let name = gvw.name().as_slice();
                let stream = if name == b"$stdout" {
                    Some("stdout")
                } else if name == b"$stderr" {
                    Some("stderr")
                } else {
                    None
                };
                if let Some(stream) = stream {
                    let loc = gvw.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(Diagnostic {
                        path: self.source.path_str().to_string(),
                        location: crate::diagnostic::Location { line, column },
                        severity: Severity::Convention,
                        cop_name: "RSpec/ExpectOutput".to_string(),
                        message: format!(
                            "Use `expect {{ ... }}.to output(...).to_{stream}` instead of mutating ${stream}."
                        ),
                        corrected: false,
                    });
                }
            }
            return;
        }

        // Track example scope: it/specify blocks set in_example_scope
        if let Some(call) = node.as_call_node() {
            let name = call.name().as_slice();
            if call.receiver().is_none() && (is_rspec_example(name) || is_per_example_hook(&call)) {
                if let Some(block) = call.block() {
                    if let Some(block_node) = block.as_block_node() {
                        let old = self.in_example_scope;
                        self.in_example_scope = true;
                        if let Some(body) = block_node.body() {
                            self.visit_node(&body);
                        }
                        self.in_example_scope = old;
                        return;
                    }
                }
            }

            // Don't enter def nodes — method definitions are not example scopes
            // (this handles spec/support/helpers.rb utility methods)
        }

        // Skip def nodes — assignments in method definitions are not in example scope
        if node.as_def_node().is_some() {
            return;
        }

        // Recurse into child nodes
        if let Some(stmts) = node.as_statements_node() {
            for stmt in stmts.body().iter() {
                self.visit_node(&stmt);
            }
            return;
        }

        if let Some(call) = node.as_call_node() {
            if let Some(recv) = call.receiver() {
                self.visit_node(&recv);
            }
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    self.visit_node(&arg);
                }
            }
            if let Some(block) = call.block() {
                if let Some(block_node) = block.as_block_node() {
                    if let Some(body) = block_node.body() {
                        self.visit_node(&body);
                    }
                }
            }
            return;
        }

        if let Some(begin) = node.as_begin_node() {
            if let Some(stmts) = begin.statements() {
                for stmt in stmts.body().iter() {
                    self.visit_node(&stmt);
                }
            }
            return;
        }

        if let Some(if_node) = node.as_if_node() {
            if let Some(stmts) = if_node.statements() {
                for stmt in stmts.body().iter() {
                    self.visit_node(&stmt);
                }
            }
            if let Some(subsequent) = if_node.subsequent() {
                self.visit_node(&subsequent);
            }
            return;
        }

        if let Some(else_node) = node.as_else_node() {
            if let Some(stmts) = else_node.statements() {
                for stmt in stmts.body().iter() {
                    self.visit_node(&stmt);
                }
            }
        }
    }
}

/// Check if a call is a per-example hook (before/after/around :each or default)
fn is_per_example_hook(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    if !is_rspec_hook(name) {
        return false;
    }
    // Check if it's :all or :context scope (those are NOT per-example)
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            if let Some(sym) = arg.as_symbol_node() {
                let val = sym.unescaped();
                if val == b"all" || val == b"context" || val == b"suite" {
                    return false;
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExpectOutput, "cops/rspec/expect_output");
}
