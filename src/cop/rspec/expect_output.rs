use ruby_prism::Visit;

use crate::cop::shared::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_hook};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/ExpectOutput flags `$stdout` and `$stderr` assignments inside RSpec
/// example blocks and per-example hooks. Uses Prism `Visit` trait for generic
/// AST traversal so all intermediate node types (rescue, case, loops, etc.)
/// are automatically handled without explicit enumeration.
///
/// ## Investigation notes
/// - Handles both `GlobalVariableWriteNode` (simple `$stdout = ...`) and
///   `MultiWriteNode` with `GlobalVariableTargetNode` targets (parallel
///   assignment like `@old, $stdout = $stdout, StringIO.new`).
/// - The multi-write pattern accounts for all 19 corpus FNs (jruby, natalie).
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
        &[crate::cop::shared::node_type::PROGRAM_NODE]
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
        let program = match node.as_program_node() {
            Some(p) => p,
            None => return,
        };

        let mut visitor = ExpectOutputVisitor {
            source,
            diagnostics: Vec::new(),
            in_example_scope: false,
        };
        visitor.visit(&program.as_node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ExpectOutputVisitor<'a> {
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    in_example_scope: bool,
}

impl<'pr> Visit<'pr> for ExpectOutputVisitor<'_> {
    fn visit_global_variable_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableWriteNode<'pr>,
    ) {
        if self.in_example_scope {
            let name = node.name().as_slice();
            let stream = if name == b"$stdout" {
                Some("stdout")
            } else if name == b"$stderr" {
                Some("stderr")
            } else {
                None
            };
            if let Some(stream) = stream {
                let loc = node.location();
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
        // Don't recurse into value (no need, and default would)
    }

    fn visit_multi_write_node(&mut self, node: &ruby_prism::MultiWriteNode<'pr>) {
        if self.in_example_scope {
            for target in node.lefts().iter() {
                if let Some(gt) = target.as_global_variable_target_node() {
                    let name = gt.name().as_slice();
                    let stream = if name == b"$stdout" {
                        Some("stdout")
                    } else if name == b"$stderr" {
                        Some("stderr")
                    } else {
                        None
                    };
                    if let Some(stream) = stream {
                        let loc = node.location();
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
            }
            // Also check rest target (splat position)
            if let Some(rest) = node.rest() {
                if let Some(gt) = rest.as_global_variable_target_node() {
                    let name = gt.name().as_slice();
                    let stream = if name == b"$stdout" {
                        Some("stdout")
                    } else if name == b"$stderr" {
                        Some("stderr")
                    } else {
                        None
                    };
                    if let Some(stream) = stream {
                        let loc = node.location();
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
            }
        }
        // Don't recurse — no nested multi-writes to check
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();

        // Check if this is an example or per-example hook with a block
        if node.receiver().is_none() && (is_rspec_example(name) || is_per_example_hook(node)) {
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
                    let old = self.in_example_scope;
                    self.in_example_scope = true;
                    if let Some(body) = block_node.body() {
                        self.visit(&body);
                    }
                    self.in_example_scope = old;
                    return;
                }
            }
        }

        // For all other calls, continue default traversal
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Method definitions are NOT example scope themselves, but they may
        // contain hooks/examples (e.g., `def capture_output!; around do ...`).
        // Reset in_example_scope to false and continue traversal so that any
        // hooks or examples nested inside the def are still detected.
        let old = self.in_example_scope;
        self.in_example_scope = false;
        ruby_prism::visit_def_node(self, node);
        self.in_example_scope = old;
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
