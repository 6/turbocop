use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct MapIntoArray;

impl Cop for MapIntoArray {
    fn name(&self) -> &'static str {
        "Style/MapIntoArray"
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
        let mut visitor = MapIntoArrayVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MapIntoArrayVisitor<'a> {
    cop: &'a MapIntoArray,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl MapIntoArrayVisitor<'_> {
    /// Check if a statements node contains:
    ///   dest = []
    ///   ...each { |x| dest << expr }
    /// Pattern: look at pairs of siblings in a statements block.
    fn check_statements(&mut self, stmts: &[ruby_prism::Node<'_>]) {
        for (i, stmt) in stmts.iter().enumerate() {
            // Check if this is a `collection.each { |x| var << expr }` pattern
            let call = match stmt.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            if call.name().as_slice() != b"each" {
                continue;
            }
            if call.receiver().is_none() {
                continue;
            }
            // each must have no arguments
            if call.arguments().is_some() {
                continue;
            }

            let block = match call.block() {
                Some(b) => b,
                None => continue,
            };
            let block_node = match block.as_block_node() {
                Some(b) => b,
                None => continue,
            };
            let body = match block_node.body() {
                Some(b) => b,
                None => continue,
            };
            let body_stmts = match body.as_statements_node() {
                Some(s) => s,
                None => continue,
            };
            let body_nodes: Vec<_> = body_stmts.body().iter().collect();
            if body_nodes.len() != 1 {
                continue;
            }

            // Check for var << expr or var.push(expr) or var.append(expr)
            let push_call = match body_nodes[0].as_call_node() {
                Some(c) => c,
                None => continue,
            };
            let push_method = push_call.name().as_slice();
            if push_method != b"<<" && push_method != b"push" && push_method != b"append" {
                continue;
            }

            // Receiver must be a local variable
            let push_receiver = match push_call.receiver() {
                Some(r) => r,
                None => continue,
            };
            let lvar = match push_receiver.as_local_variable_read_node() {
                Some(l) => l,
                None => continue,
            };

            let var_name = lvar.name();

            // Check that the push argument is suitable (not a splat, etc.)
            if let Some(args) = push_call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 {
                    continue;
                }
                // Skip if argument is a splat
                if arg_list[0].as_splat_node().is_some() {
                    continue;
                }
            } else {
                continue;
            }

            // Now check: is there a preceding `var = []` in the same scope?
            let mut found_empty_array_init = false;
            let mut init_idx = 0;
            for j in (0..i).rev() {
                if let Some(asgn) = stmts[j].as_local_variable_write_node() {
                    if asgn.name().as_slice() == var_name.as_slice() {
                        // Check if the value is an empty array `[]`
                        let value = asgn.value();
                        if let Some(arr) = value.as_array_node() {
                            if arr.elements().iter().count() == 0 {
                                found_empty_array_init = true;
                                init_idx = j;
                            }
                        }
                        break; // found the most recent assignment, stop
                    }
                }
            }

            if !found_empty_array_init {
                continue;
            }

            // Check that var is not referenced between the init and the each call.
            // If there are other uses of the variable (like `var << something`),
            // we can't guarantee it's still an empty array.
            let var_name_slice = var_name.as_slice();
            let mut has_intermediate_ref = false;
            for j in (init_idx + 1)..i {
                if references_variable(&stmts[j], var_name_slice) {
                    has_intermediate_ref = true;
                    break;
                }
            }
            if has_intermediate_ref {
                continue;
            }

            // Receiver of `each` must not be `self`
            if let Some(each_receiver) = call.receiver() {
                if each_receiver.as_self_node().is_some() {
                    continue;
                }
            }

            let loc = call.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Use `map` instead of `each` to map elements into an array.".to_string(),
            ));
        }
    }
}

impl<'pr> Visit<'pr> for MapIntoArrayVisitor<'_> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let stmts: Vec<_> = node.body().iter().collect();
        self.check_statements(&stmts);
        ruby_prism::visit_statements_node(self, node);
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        if let Some(body) = node.statements() {
            let stmts: Vec<_> = body.body().iter().collect();
            self.check_statements(&stmts);
        }
        ruby_prism::visit_begin_node(self, node);
    }
}

/// Check if a node (recursively) references a local variable with the given name.
fn references_variable(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(lv) = node.as_local_variable_read_node() {
        if lv.name().as_slice() == var_name {
            return true;
        }
    }
    if let Some(lv) = node.as_local_variable_write_node() {
        if lv.name().as_slice() == var_name {
            return true;
        }
    }
    // Check children recursively
    struct VarRefFinder<'a> {
        var_name: &'a [u8],
        found: bool,
    }
    impl<'pr> ruby_prism::Visit<'pr> for VarRefFinder<'_> {
        fn visit_local_variable_read_node(
            &mut self,
            node: &ruby_prism::LocalVariableReadNode<'pr>,
        ) {
            if node.name().as_slice() == self.var_name {
                self.found = true;
            }
        }
        fn visit_local_variable_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableWriteNode<'pr>,
        ) {
            if node.name().as_slice() == self.var_name {
                self.found = true;
            }
            // Must recurse into the value of the write node, otherwise
            // we miss references inside the RHS (e.g., `entries = src.map { order << x }`)
            ruby_prism::visit_local_variable_write_node(self, node);
        }
        fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
            // Check receiver and arguments
            ruby_prism::visit_call_node(self, node);
        }
    }
    let mut finder = VarRefFinder {
        var_name,
        found: false,
    };
    ruby_prism::Visit::visit(&mut finder, node);
    finder.found
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MapIntoArray, "cops/style/map_into_array");
}
