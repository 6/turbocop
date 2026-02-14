use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
/// RSpec/DescribedClass: Use `described_class` instead of referencing the class directly.
pub struct DescribedClass;

impl Cop for DescribedClass {
    fn name(&self) -> &'static str {
        "RSpec/DescribedClass"
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
        let program = match node.as_program_node() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let mut visitor = DescribedClassVisitor {
            source,
            diagnostics: Vec::new(),
            described_class_name: None,
        };

        for stmt in program.statements().body().iter() {
            visitor.check_statement(&stmt);
        }

        visitor.diagnostics
    }
}

struct DescribedClassVisitor<'a> {
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    described_class_name: Option<Vec<u8>>,
}

impl<'a> DescribedClassVisitor<'a> {
    fn check_statement(&mut self, node: &ruby_prism::Node<'_>) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();
        if name != b"describe" {
            return;
        }

        // Must be receiverless or RSpec.describe
        if let Some(recv) = call.receiver() {
            let is_rspec = if let Some(cr) = recv.as_constant_read_node() {
                cr.name().as_slice() == b"RSpec"
            } else if let Some(cp) = recv.as_constant_path_node() {
                cp.name().map_or(false, |n| n.as_slice() == b"RSpec") && cp.parent().is_none()
            } else {
                false
            };
            if !is_rspec {
                return;
            }
        }

        // Extract the described class from the first argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let first_arg = &arg_list[0];
        let class_name = extract_constant_source(self.source, first_arg);

        if class_name.is_none() {
            return; // Not a constant reference
        }

        let old = self.described_class_name.take();
        self.described_class_name = class_name;

        // Walk the block body looking for references to the described class
        if let Some(block) = call.block() {
            if let Some(block_node) = block.as_block_node() {
                if let Some(body) = block_node.body() {
                    self.walk_block_body(&body);
                }
            }
        }

        self.described_class_name = old;
    }

    fn walk_block_body(&mut self, node: &ruby_prism::Node<'_>) {
        if let Some(stmts) = node.as_statements_node() {
            for stmt in stmts.body().iter() {
                self.check_for_class_reference(&stmt);
            }
        }
    }

    fn check_for_class_reference(&mut self, node: &ruby_prism::Node<'_>) {
        let class_name = match &self.described_class_name {
            Some(n) => n.clone(),
            None => return,
        };

        // Check if this is a nested describe with a class arg - recurse
        if let Some(call) = node.as_call_node() {
            let name = call.name().as_slice();
            if name == b"describe" || name == b"context" {
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if !arg_list.is_empty() {
                        if let Some(nested_class) = extract_constant_source(self.source, &arg_list[0]) {
                            // Nested describe with class - use that class name
                            let old = self.described_class_name.take();
                            self.described_class_name = Some(nested_class);
                            if let Some(block) = call.block() {
                                if let Some(block_node) = block.as_block_node() {
                                    if let Some(body) = block_node.body() {
                                        self.walk_block_body(&body);
                                    }
                                }
                            }
                            self.described_class_name = old;
                            return;
                        }
                    }
                }
                // describe/context without class - walk body
                if let Some(block) = call.block() {
                    if let Some(block_node) = block.as_block_node() {
                        if let Some(body) = block_node.body() {
                            self.walk_block_body(&body);
                        }
                    }
                }
                return;
            }

            // Skip scope-changing methods (Class.new, Module.new, etc.)
            if is_scope_change(&call) {
                return;
            }

            // For other calls with blocks, recurse into the block
            if let Some(block) = call.block() {
                if let Some(block_node) = block.as_block_node() {
                    if let Some(body) = block_node.body() {
                        self.walk_block_body(&body);
                    }
                }
            }

            // Check arguments and receiver
            if let Some(recv) = call.receiver() {
                self.check_constant_ref(&recv, &class_name);
            }
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    self.check_constant_ref(&arg, &class_name);
                }
            }
            return;
        }

        // Skip class/module definitions
        if node.as_class_node().is_some() || node.as_module_node().is_some() {
            return;
        }

        // For any other node, check if it's a constant reference
        self.check_constant_ref(node, &class_name);
    }

    fn check_constant_ref(&mut self, node: &ruby_prism::Node<'_>, class_name: &[u8]) {
        if let Some(cr) = node.as_constant_read_node() {
            if cr.name().as_slice() == class_name {
                let loc = cr.location();
                let (line, col) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(Diagnostic {
                    path: self.source.path_str().to_string(),
                    location: crate::diagnostic::Location { line, column: col },
                    severity: Severity::Convention,
                    cop_name: "RSpec/DescribedClass".to_string(),
                    message: format!(
                        "Use `described_class` instead of `{}`.",
                        std::str::from_utf8(class_name).unwrap_or("?")
                    ),
                });
            }
        } else if let Some(cp) = node.as_constant_path_node() {
            let full = extract_constant_source(self.source, node);
            if let Some(full_name) = full {
                if full_name == class_name {
                    let loc = cp.location();
                    let (line, col) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(Diagnostic {
                        path: self.source.path_str().to_string(),
                        location: crate::diagnostic::Location { line, column: col },
                        severity: Severity::Convention,
                        cop_name: "RSpec/DescribedClass".to_string(),
                        message: format!(
                            "Use `described_class` instead of `{}`.",
                            std::str::from_utf8(class_name).unwrap_or("?")
                        ),
                    });
                }
            }
        }
    }
}

fn extract_constant_source<'a>(source: &'a SourceFile, node: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
    if node.as_constant_read_node().is_some() || node.as_constant_path_node().is_some() {
        let loc = node.location();
        let bytes = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
        // Skip if starts with :: (absolute path)
        Some(bytes.to_vec())
    } else {
        None
    }
}

fn is_scope_change(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    if let Some(recv) = call.receiver() {
        if let Some(cr) = recv.as_constant_read_node() {
            let class_name = cr.name().as_slice();
            if (class_name == b"Class"
                || class_name == b"Module"
                || class_name == b"Struct"
                || class_name == b"Data")
                && (name == b"new" || name == b"define")
            {
                return true;
            }
        }
    }
    // Also skip *_eval, *_exec methods
    if name.ends_with(b"_eval") || name.ends_with(b"_exec") {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(DescribedClass, "cops/rspec/described_class");
}
