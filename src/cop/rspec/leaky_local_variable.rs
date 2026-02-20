use crate::cop::util::{self, is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{AND_NODE, ARRAY_NODE, ASSOC_NODE, ASSOC_SPLAT_NODE, BLOCK_NODE, CALL_NODE, ELSE_NODE, EMBEDDED_STATEMENTS_NODE, HASH_NODE, IF_NODE, INSTANCE_VARIABLE_WRITE_NODE, INTERPOLATED_STRING_NODE, INTERPOLATED_SYMBOL_NODE, KEYWORD_HASH_NODE, LOCAL_VARIABLE_OR_WRITE_NODE, LOCAL_VARIABLE_READ_NODE, LOCAL_VARIABLE_WRITE_NODE, MULTI_WRITE_NODE, OR_NODE, PARENTHESES_NODE, RETURN_NODE, SPLAT_NODE, STATEMENTS_NODE, UNLESS_NODE};

pub struct LeakyLocalVariable;

/// Flags local variable assignments at the example-group level that are then
/// referenced inside examples, hooks, let, or subject blocks. Use `let` instead.
impl Cop for LeakyLocalVariable {
    fn name(&self) -> &'static str {
        "RSpec/LeakyLocalVariable"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[AND_NODE, ARRAY_NODE, ASSOC_NODE, ASSOC_SPLAT_NODE, BLOCK_NODE, CALL_NODE, ELSE_NODE, EMBEDDED_STATEMENTS_NODE, HASH_NODE, IF_NODE, INSTANCE_VARIABLE_WRITE_NODE, INTERPOLATED_STRING_NODE, INTERPOLATED_SYMBOL_NODE, KEYWORD_HASH_NODE, LOCAL_VARIABLE_OR_WRITE_NODE, LOCAL_VARIABLE_READ_NODE, LOCAL_VARIABLE_WRITE_NODE, MULTI_WRITE_NODE, OR_NODE, PARENTHESES_NODE, RETURN_NODE, SPLAT_NODE, STATEMENTS_NODE, UNLESS_NODE]
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
        // Look for describe/context blocks (including RSpec.describe)
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        let is_example_group = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).map_or(false, |n| n == b"RSpec") && is_rspec_example_group(method_name)
        } else {
            is_rspec_example_group(method_name)
        };

        if !is_example_group {
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

        check_scope_for_leaky_vars(source, block_node, diagnostics, self);
    }
}

fn check_scope_for_leaky_vars(
    source: &SourceFile,
    block: ruby_prism::BlockNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &LeakyLocalVariable,
) {
    let body = match block.body() {
        Some(b) => b,
        None => return,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return,
    };

    let stmt_list: Vec<_> = stmts.body().iter().collect();

    // Collect local variable assignments at this scope level
    struct VarAssign {
        name: Vec<u8>,
        offset: usize,
    }

    let mut assignments: Vec<VarAssign> = Vec::new();

    for stmt in &stmt_list {
        if let Some(lw) = stmt.as_local_variable_write_node() {
            assignments.push(VarAssign {
                name: lw.name().as_slice().to_vec(),
                offset: lw.location().start_offset(),
            });
        }
    }

    // For each assignment, check if the variable is used inside any example/hook/let/subject block
    for assign in &assignments {
        let mut used_in_block = false;
        for stmt in &stmt_list {
            if let Some(call) = stmt.as_call_node() {
                let name = call.name().as_slice();
                // Check if this is an example, hook, let, subject, or it_behaves_like
                let is_inner_scope = matches!(
                    name,
                    b"it" | b"specify" | b"example" | b"scenario"
                        | b"xit" | b"xspecify" | b"xexample" | b"xscenario"
                        | b"fit" | b"fspecify" | b"fexample" | b"fscenario"
                        | b"before" | b"after" | b"around"
                        | b"let" | b"let!"
                        | b"subject" | b"subject!"
                ) && call.receiver().is_none();

                let is_it_behaves_like = matches!(
                    name,
                    b"it_behaves_like" | b"it_should_behave_like" | b"include_examples"
                ) && call.receiver().is_none();

                if is_inner_scope {
                    if let Some(blk) = call.block() {
                        if let Some(bn) = blk.as_block_node() {
                            if block_body_references_var(bn, &assign.name) {
                                used_in_block = true;
                                break;
                            }
                        }
                    }
                    // Also check string interpolation in the first argument (e.g., `it "foo #{var}"`)
                    if let Some(args) = call.arguments() {
                        for arg in args.arguments().iter() {
                            if is_inner_scope && arg_references_var_in_interpolation(&arg, &assign.name) {
                                // If it's used ONLY in the description, not in the block body, skip
                                // But if used in BOTH description AND body, flag it
                                if let Some(blk) = call.block() {
                                    if let Some(bn) = blk.as_block_node() {
                                        if block_body_references_var(bn, &assign.name) {
                                            used_in_block = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if is_it_behaves_like {
                    // Check arguments for direct reference to the variable
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        // Skip the first arg (the shared example name) unless it's the var itself
                        for (i, arg) in arg_list.iter().enumerate() {
                            if i == 0 {
                                // First arg is the shared example name — skip unless it's the var
                                continue;
                            }
                            if node_references_var(arg, &assign.name) {
                                used_in_block = true;
                                break;
                            }
                        }
                    }
                    if used_in_block {
                        break;
                    }
                }

                // Check nested example groups (describe/context) — a variable
                // assigned at an outer scope leaks into inner example groups'
                // examples and hooks.
                if is_rspec_example_group(name) && call.receiver().is_none() {
                    if let Some(blk) = call.block() {
                        if let Some(bn) = blk.as_block_node() {
                            if var_used_in_nested_scopes(bn, &assign.name) {
                                used_in_block = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        if used_in_block {
            let (line, column) = source.offset_to_line_col(assign.offset);
            diagnostics.push(cop.diagnostic(
                source,
                line,
                column,
                "Do not use local variables defined outside of examples inside of them.".to_string(),
            ));
        }
    }

    // NOTE: We do NOT recurse into nested example groups here, because
    // the walker's `check_node` will visit them separately. Recursing
    // would cause duplicate diagnostics for the same assignment.
}

/// Check if a variable is used inside examples/hooks/let/subject blocks within
/// a nested example group block. Recurses into further nested example groups.
fn var_used_in_nested_scopes(block: ruby_prism::BlockNode<'_>, var_name: &[u8]) -> bool {
    let body = match block.body() {
        Some(b) => b,
        None => return false,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };

    for stmt in stmts.body().iter() {
        if let Some(call) = stmt.as_call_node() {
            let name = call.name().as_slice();
            let is_inner_scope = matches!(
                name,
                b"it" | b"specify" | b"example" | b"scenario"
                    | b"xit" | b"xspecify" | b"xexample" | b"xscenario"
                    | b"fit" | b"fspecify" | b"fexample" | b"fscenario"
                    | b"before" | b"after" | b"around"
                    | b"let" | b"let!"
                    | b"subject" | b"subject!"
            ) && call.receiver().is_none();

            if is_inner_scope {
                if let Some(blk) = call.block() {
                    if let Some(bn) = blk.as_block_node() {
                        if block_body_references_var(bn, var_name) {
                            return true;
                        }
                    }
                }
            }

            // Recurse into nested example groups
            if is_rspec_example_group(name) && call.receiver().is_none() {
                if let Some(blk) = call.block() {
                    if let Some(bn) = blk.as_block_node() {
                        if var_used_in_nested_scopes(bn, var_name) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

fn block_body_references_var(block: ruby_prism::BlockNode<'_>, var_name: &[u8]) -> bool {
    let body = match block.body() {
        Some(b) => b,
        None => return false,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };

    for stmt in stmts.body().iter() {
        if node_references_var(&stmt, var_name) {
            return true;
        }
    }
    false
}

fn node_references_var(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(lv) = node.as_local_variable_read_node() {
        if lv.name().as_slice() == var_name {
            return true;
        }
    }

    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            if node_references_var(&recv, var_name) {
                return true;
            }
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if node_references_var(&arg, var_name) {
                    return true;
                }
            }
        }
        if let Some(block) = call.block() {
            if let Some(bn) = block.as_block_node() {
                if let Some(body) = bn.body() {
                    if let Some(stmts) = body.as_statements_node() {
                        for s in stmts.body().iter() {
                            if node_references_var(&s, var_name) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }

    // Local variable assignment: `x = expr` -- check if the RHS references the var
    if let Some(lw) = node.as_local_variable_write_node() {
        if node_references_var(&lw.value(), var_name) {
            return true;
        }
    }

    // Instance variable assignment: `@x = expr`
    if let Some(iw) = node.as_instance_variable_write_node() {
        if node_references_var(&iw.value(), var_name) {
            return true;
        }
    }

    // Local variable or-write: `x ||= expr`
    if let Some(ow) = node.as_local_variable_or_write_node() {
        if node_references_var(&ow.value(), var_name) {
            return true;
        }
    }

    // Multi-write: `a, b = expr`
    if let Some(mw) = node.as_multi_write_node() {
        if node_references_var(&mw.value(), var_name) {
            return true;
        }
    }

    // If/Unless nodes
    if let Some(if_node) = node.as_if_node() {
        if node_references_var(&if_node.predicate(), var_name) {
            return true;
        }
        if let Some(stmts) = if_node.statements() {
            for s in stmts.body().iter() {
                if node_references_var(&s, var_name) {
                    return true;
                }
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            if node_references_var(&subsequent, var_name) {
                return true;
            }
        }
    }

    if let Some(unless_node) = node.as_unless_node() {
        if node_references_var(&unless_node.predicate(), var_name) {
            return true;
        }
        if let Some(stmts) = unless_node.statements() {
            for s in stmts.body().iter() {
                if node_references_var(&s, var_name) {
                    return true;
                }
            }
        }
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    if node_references_var(&s, var_name) {
                        return true;
                    }
                }
            }
        }
    }

    // ElseNode
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            for s in stmts.body().iter() {
                if node_references_var(&s, var_name) {
                    return true;
                }
            }
        }
    }

    // Return node
    if let Some(ret) = node.as_return_node() {
        if let Some(args) = ret.arguments() {
            for arg in args.arguments().iter() {
                if node_references_var(&arg, var_name) {
                    return true;
                }
            }
        }
    }

    // Parentheses node
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            if node_references_var(&body, var_name) {
                return true;
            }
        }
    }

    // And/Or nodes
    if let Some(and_node) = node.as_and_node() {
        if node_references_var(&and_node.left(), var_name) || node_references_var(&and_node.right(), var_name) {
            return true;
        }
    }
    if let Some(or_node) = node.as_or_node() {
        if node_references_var(&or_node.left(), var_name) || node_references_var(&or_node.right(), var_name) {
            return true;
        }
    }

    if let Some(interp) = node.as_interpolated_string_node() {
        for part in interp.parts().iter() {
            if let Some(embedded) = part.as_embedded_statements_node() {
                if let Some(stmts) = embedded.statements() {
                    for s in stmts.body().iter() {
                        if node_references_var(&s, var_name) {
                            return true;
                        }
                    }
                }
            }
        }
    }

    if let Some(interp) = node.as_interpolated_symbol_node() {
        for part in interp.parts().iter() {
            if let Some(embedded) = part.as_embedded_statements_node() {
                if let Some(stmts) = embedded.statements() {
                    for s in stmts.body().iter() {
                        if node_references_var(&s, var_name) {
                            return true;
                        }
                    }
                }
            }
        }
    }

    if let Some(arr) = node.as_array_node() {
        for elem in arr.elements().iter() {
            if node_references_var(&elem, var_name) {
                return true;
            }
        }
    }

    if let Some(hash) = node.as_hash_node() {
        for elem in hash.elements().iter() {
            if let Some(assoc) = elem.as_assoc_node() {
                if node_references_var(&assoc.key(), var_name) || node_references_var(&assoc.value(), var_name) {
                    return true;
                }
            }
        }
    }

    // Check keyword hash arguments
    if let Some(kw) = node.as_keyword_hash_node() {
        for elem in kw.elements().iter() {
            if let Some(assoc) = elem.as_assoc_node() {
                if node_references_var(&assoc.key(), var_name) || node_references_var(&assoc.value(), var_name) {
                    return true;
                }
            }
        }
    }

    // Splat node (e.g., *args)
    if let Some(splat) = node.as_splat_node() {
        if let Some(expr) = splat.expression() {
            if node_references_var(&expr, var_name) {
                return true;
            }
        }
    }

    // AssocSplatNode (e.g., **opts)
    if let Some(assoc_splat) = node.as_assoc_splat_node() {
        if let Some(expr) = assoc_splat.value() {
            if node_references_var(&expr, var_name) {
                return true;
            }
        }
    }

    false
}

fn arg_references_var_in_interpolation(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(interp) = node.as_interpolated_string_node() {
        for part in interp.parts().iter() {
            if let Some(embedded) = part.as_embedded_statements_node() {
                if let Some(stmts) = embedded.statements() {
                    for s in stmts.body().iter() {
                        if node_references_var(&s, var_name) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LeakyLocalVariable, "cops/rspec/leaky_local_variable");
}
