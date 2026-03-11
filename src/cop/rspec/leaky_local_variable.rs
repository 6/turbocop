use crate::cop::node_type::{BLOCK_NODE, CALL_NODE};
use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Flags local variable assignments at the example-group level that are then
/// referenced inside examples, hooks, let, or subject blocks. Use `let` instead.
///
/// ## Root cause of previous FP/FN gap (23 FP, 933 FN)
///
/// The old implementation only collected direct `LocalVariableWriteNode` children
/// of the block body (top-level statements). Assignments nested inside
/// `if`/`unless`/`case`/`begin` or iterator blocks were missed (933 FN).
///
/// FPs came from not properly handling block parameter shadowing and variables
/// used only in example descriptions/metadata.
///
/// ## Current approach
///
/// Instead of VariableForce (which RuboCop uses), we take a pragmatic approach:
/// 1. When visiting an example group block, recursively collect ALL local variable
///    assignments within the block body, stopping at scope boundaries (examples,
///    hooks, let, subject, nested example groups).
/// 2. For each assignment, check if the variable is referenced inside any example
///    scope (examples, hooks, let, subject, includes args).
/// 3. Exclude "allowed" references: variables used only in example descriptions,
///    metadata keyword args, `it_behaves_like` first arg, or interpolated
///    string/symbol args to includes methods.
/// 4. Respect block parameter shadowing throughout.
///
/// ## Investigation (FP=41, FN=409, 2026-03-10)
///
/// **FP fix: reassignment-before-use (41 FPs)**
/// RuboCop's VariableForce performs flow-sensitive analysis, tracking that a
/// variable reassigned inside an example block before any read creates a new
/// binding that doesn't reference the outer scope. Our implementation now checks
/// `var_written_before_read_in_stmts` to suppress offenses when the first mention
/// of the variable in the block is an unconditional write.
///
/// **FN fix: missing `include_context` (contributes to 409 FNs)**
/// `is_includes_method` was missing `include_context`. RuboCop's `Includes.all`
/// includes both `Examples` (`it_behaves_like`, `it_should_behave_like`,
/// `include_examples`) and `Context` (`include_context`). Variables passed as
/// non-first args to `include_context` should be flagged.
///
/// **Remaining FN gap:** The bulk of the 409 FNs likely comes from cases where
/// RuboCop's VariableForce tracks variable references across Ruby scope
/// boundaries that our AST-walking approach doesn't replicate (e.g., variables
/// assigned before the top-level `describe` block, complex flow-sensitive
/// reassignment patterns). A full VariableForce implementation would close this
/// gap but is a significant engineering effort.
///
/// ## Investigation (FP=32, FN=409, 2026-03-11)
///
/// **FN fix: file-level variables (major FN source)**
/// Added `check_source` to detect variables assigned at file level (outside
/// describe blocks) that are referenced inside example scopes within describe
/// blocks. Corpus FN examples showed patterns like `spec_helper/xcscheme.rb:5`
/// where variables are assigned at line 2-6 before any describe block.
/// Implementation uses `check_file_level_vars` which collects file-level
/// assignments and checks them against all describe blocks in the file.
///
/// **FP fix: begin-block reassignment (reduces remaining FPs)**
/// Improved `is_unconditional_var_write` to recurse into `begin` blocks and
/// parenthesized expressions. A write inside `begin; x = ...; end` at the
/// start of an example block means the outer variable is never read, matching
/// RuboCop's VariableForce behavior.
///
/// **Remaining gaps:** 32 FPs from prior cycle likely involve complex
/// reassignment patterns (e.g., reassignment after non-reading statements,
/// or inside rescue blocks). 409 FNs from prior cycle partially addressed
/// by file-level variable detection; remaining FNs likely from VariableForce's
/// comprehensive scope tracking that we don't fully replicate.
pub struct LeakyLocalVariable;

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
        &[BLOCK_NODE, CALL_NODE]
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        check_file_level_vars(source, &parse_result.node(), diagnostics, self);
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        let is_example_group = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                && is_rspec_example_group(method_name)
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

/// A local variable assignment found in the example-group scope.
struct VarAssign {
    name: Vec<u8>,
    offset: usize,
}

/// Check for file-level variable assignments that leak into describe blocks.
/// This handles the case where variables are assigned before/outside the top-level
/// describe block and then referenced inside example scopes within it.
fn check_file_level_vars(
    source: &SourceFile,
    program: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &LeakyLocalVariable,
) {
    let program_node = match program.as_program_node() {
        Some(p) => p,
        None => return,
    };
    let stmts = match program_node.statements().body().is_empty() {
        true => return,
        false => program_node.statements(),
    };

    // Collect file-level variable assignments (not inside describe blocks)
    let mut file_level_assigns: Vec<VarAssign> = Vec::new();
    for stmt in stmts.body().iter() {
        collect_file_level_assignments(&stmt, &mut file_level_assigns);
    }

    if file_level_assigns.is_empty() {
        return;
    }

    // For each file-level assignment, check if the variable is referenced
    // inside any example scope within any describe block in the file
    for assign in &file_level_assigns {
        let mut used = false;
        for stmt in stmts.body().iter() {
            if check_var_used_in_describe_blocks(&stmt, &assign.name) {
                used = true;
                break;
            }
        }
        if used {
            let (line, column) = source.offset_to_line_col(assign.offset);
            diagnostics.push(
                cop.diagnostic(
                    source,
                    line,
                    column,
                    "Do not use local variables defined outside of examples inside of them."
                        .to_string(),
                ),
            );
        }
    }
}

/// Collect variable assignments at file level, stopping at describe blocks,
/// class/module definitions, and method definitions.
fn collect_file_level_assignments(node: &ruby_prism::Node<'_>, assigns: &mut Vec<VarAssign>) {
    // Direct assignment
    if let Some(lw) = node.as_local_variable_write_node() {
        assigns.push(VarAssign {
            name: lw.name().as_slice().to_vec(),
            offset: lw.location().start_offset(),
        });
        return;
    }

    // or-write: `x ||= expr`
    if let Some(ow) = node.as_local_variable_or_write_node() {
        assigns.push(VarAssign {
            name: ow.name().as_slice().to_vec(),
            offset: ow.location().start_offset(),
        });
        return;
    }

    // and-write: `x &&= expr`
    if let Some(aw) = node.as_local_variable_and_write_node() {
        assigns.push(VarAssign {
            name: aw.name().as_slice().to_vec(),
            offset: aw.location().start_offset(),
        });
        return;
    }

    // Multi-write: `a, b = expr`
    if let Some(mw) = node.as_multi_write_node() {
        for target in mw.lefts().iter() {
            if let Some(lt) = target.as_local_variable_target_node() {
                assigns.push(VarAssign {
                    name: lt.name().as_slice().to_vec(),
                    offset: lt.location().start_offset(),
                });
            }
        }
        return;
    }

    // Stop at describe blocks, classes, modules, defs - these are scope boundaries
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        let no_recv = call.receiver().is_none()
            || (call
                .receiver()
                .is_some_and(|r| util::constant_name(&r).is_some_and(|n| n == b"RSpec")));
        if no_recv && is_rspec_example_group(name) {
            return;
        }
        // For other calls (e.g., iterators), recurse into block body
        if let Some(blk) = call.block() {
            if let Some(bn) = blk.as_block_node() {
                if let Some(body) = bn.body() {
                    if let Some(stmts) = body.as_statements_node() {
                        for s in stmts.body().iter() {
                            collect_file_level_assignments(&s, assigns);
                        }
                    }
                }
            }
        }
        return;
    }

    // Recurse through control flow
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            for s in stmts.body().iter() {
                collect_file_level_assignments(&s, assigns);
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            collect_file_level_assignments(&subsequent, assigns);
        }
        return;
    }

    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for s in stmts.body().iter() {
                collect_file_level_assignments(&s, assigns);
            }
        }
    }

    // Stop at class/module/def
}

/// Check if a variable is referenced inside any example scope within describe
/// blocks found in the given node tree.
fn check_var_used_in_describe_blocks(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        let is_eg = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                && is_rspec_example_group(name)
        } else {
            is_rspec_example_group(name)
        };

        if is_eg {
            // Found a describe block - check if the variable is used in its example scopes
            if let Some(blk) = call.block() {
                if let Some(bn) = blk.as_block_node() {
                    if let Some(body) = bn.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            for s in stmts.body().iter() {
                                if check_var_used_in_example_scopes(&s, var_name) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
            return false;
        }

        // For other calls with blocks, recurse
        if let Some(blk) = call.block() {
            if let Some(bn) = blk.as_block_node() {
                if let Some(body) = bn.body() {
                    if let Some(stmts) = body.as_statements_node() {
                        for s in stmts.body().iter() {
                            if check_var_used_in_describe_blocks(&s, var_name) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        return false;
    }

    // Recurse through control flow
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            for s in stmts.body().iter() {
                if check_var_used_in_describe_blocks(&s, var_name) {
                    return true;
                }
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            if check_var_used_in_describe_blocks(&subsequent, var_name) {
                return true;
            }
        }
        return false;
    }

    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for s in stmts.body().iter() {
                if check_var_used_in_describe_blocks(&s, var_name) {
                    return true;
                }
            }
        }
        return false;
    }

    false
}

/// Check an example group block for leaky local variables.
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

    // Collect all local variable assignments in this scope (recursively through
    // non-scope-boundary nodes like if/unless/case/begin, but stopping at
    // example scopes and nested example groups).
    let mut assignments: Vec<VarAssign> = Vec::new();
    for stmt in stmts.body().iter() {
        collect_assignments_in_scope(&stmt, &mut assignments);
    }

    // For each assignment, check if the variable is referenced inside any
    // example scope within this block.
    for assign in &assignments {
        let mut used_in_example_scope = false;
        for stmt in stmts.body().iter() {
            if check_var_used_in_example_scopes(&stmt, &assign.name) {
                used_in_example_scope = true;
                break;
            }
        }

        if used_in_example_scope {
            let (line, column) = source.offset_to_line_col(assign.offset);
            diagnostics.push(
                cop.diagnostic(
                    source,
                    line,
                    column,
                    "Do not use local variables defined outside of examples inside of them."
                        .to_string(),
                ),
            );
        }
    }
}

/// Recursively collect local variable assignments within a node, stopping at
/// scope boundaries (examples, hooks, let, subject, nested example groups,
/// method definitions, class/module definitions).
fn collect_assignments_in_scope(node: &ruby_prism::Node<'_>, assigns: &mut Vec<VarAssign>) {
    // Direct assignment
    if let Some(lw) = node.as_local_variable_write_node() {
        assigns.push(VarAssign {
            name: lw.name().as_slice().to_vec(),
            offset: lw.location().start_offset(),
        });
        return;
    }

    // or-write: `x ||= expr`
    if let Some(ow) = node.as_local_variable_or_write_node() {
        assigns.push(VarAssign {
            name: ow.name().as_slice().to_vec(),
            offset: ow.location().start_offset(),
        });
        return;
    }

    // and-write: `x &&= expr`
    if let Some(aw) = node.as_local_variable_and_write_node() {
        assigns.push(VarAssign {
            name: aw.name().as_slice().to_vec(),
            offset: aw.location().start_offset(),
        });
        return;
    }

    // Multi-write: `a, b = expr` -- collect targets
    if let Some(mw) = node.as_multi_write_node() {
        for target in mw.lefts().iter() {
            if let Some(lt) = target.as_local_variable_target_node() {
                assigns.push(VarAssign {
                    name: lt.name().as_slice().to_vec(),
                    offset: lt.location().start_offset(),
                });
            }
        }
        if let Some(rest) = mw.rest() {
            if let Some(sr) = rest.as_splat_node() {
                if let Some(expr) = sr.expression() {
                    if let Some(lt) = expr.as_local_variable_target_node() {
                        assigns.push(VarAssign {
                            name: lt.name().as_slice().to_vec(),
                            offset: lt.location().start_offset(),
                        });
                    }
                }
            }
        }
        for target in mw.rights().iter() {
            if let Some(lt) = target.as_local_variable_target_node() {
                assigns.push(VarAssign {
                    name: lt.name().as_slice().to_vec(),
                    offset: lt.location().start_offset(),
                });
            }
        }
        return;
    }

    // Call nodes: stop at scope boundaries, recurse into non-scope calls
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        let no_recv = call.receiver().is_none();

        // Stop at example scopes, nested example groups, includes methods
        if no_recv
            && (is_example_scope(name) || is_rspec_example_group(name) || is_includes_method(name))
        {
            return;
        }

        // For other calls (e.g., `each do ... end`), recurse into the block body
        if let Some(blk) = call.block() {
            if let Some(bn) = blk.as_block_node() {
                if let Some(body) = bn.body() {
                    if let Some(stmts) = body.as_statements_node() {
                        for s in stmts.body().iter() {
                            collect_assignments_in_scope(&s, assigns);
                        }
                    }
                }
            }
        }
        return;
    }

    // If/Unless: recurse into branches
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            for s in stmts.body().iter() {
                collect_assignments_in_scope(&s, assigns);
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            collect_assignments_in_scope(&subsequent, assigns);
        }
        return;
    }
    if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            for s in stmts.body().iter() {
                collect_assignments_in_scope(&s, assigns);
            }
        }
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    collect_assignments_in_scope(&s, assigns);
                }
            }
        }
        return;
    }

    // Else node
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            for s in stmts.body().iter() {
                collect_assignments_in_scope(&s, assigns);
            }
        }
        return;
    }

    // Case/When/In
    if let Some(case_node) = node.as_case_node() {
        for cond in case_node.conditions().iter() {
            if let Some(when_node) = cond.as_when_node() {
                if let Some(stmts) = when_node.statements() {
                    for s in stmts.body().iter() {
                        collect_assignments_in_scope(&s, assigns);
                    }
                }
            }
            if let Some(in_node) = cond.as_in_node() {
                if let Some(stmts) = in_node.statements() {
                    for s in stmts.body().iter() {
                        collect_assignments_in_scope(&s, assigns);
                    }
                }
            }
        }
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    collect_assignments_in_scope(&s, assigns);
                }
            }
        }
        return;
    }

    // CaseMatch (pattern matching)
    if let Some(cm) = node.as_case_match_node() {
        for cond in cm.conditions().iter() {
            if let Some(in_node) = cond.as_in_node() {
                if let Some(stmts) = in_node.statements() {
                    for s in stmts.body().iter() {
                        collect_assignments_in_scope(&s, assigns);
                    }
                }
            }
        }
        if let Some(else_clause) = cm.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    collect_assignments_in_scope(&s, assigns);
                }
            }
        }
        return;
    }

    // Begin/Rescue/Ensure
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for s in stmts.body().iter() {
                collect_assignments_in_scope(&s, assigns);
            }
        }
        if let Some(rescue_clause) = begin_node.rescue_clause() {
            collect_assignments_in_rescue_node(&rescue_clause, assigns);
        }
        if let Some(else_clause) = begin_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    collect_assignments_in_scope(&s, assigns);
                }
            }
        }
        if let Some(ensure_clause) = begin_node.ensure_clause() {
            if let Some(stmts) = ensure_clause.statements() {
                for s in stmts.body().iter() {
                    collect_assignments_in_scope(&s, assigns);
                }
            }
        }
        return;
    }

    // Parentheses
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            collect_assignments_in_scope(&body, assigns);
        }
        return;
    }

    // Statements node
    if let Some(stmts) = node.as_statements_node() {
        for s in stmts.body().iter() {
            collect_assignments_in_scope(&s, assigns);
        }
        return;
    }

    // While/Until loops
    if let Some(while_node) = node.as_while_node() {
        if let Some(stmts) = while_node.statements() {
            for s in stmts.body().iter() {
                collect_assignments_in_scope(&s, assigns);
            }
        }
        return;
    }
    if let Some(until_node) = node.as_until_node() {
        if let Some(stmts) = until_node.statements() {
            for s in stmts.body().iter() {
                collect_assignments_in_scope(&s, assigns);
            }
        }
        return;
    }

    // For loop
    if let Some(for_node) = node.as_for_node() {
        if let Some(stmts) = for_node.statements() {
            for s in stmts.body().iter() {
                collect_assignments_in_scope(&s, assigns);
            }
        }
    }

    // Stop at class/module/def -- these are Ruby scope boundaries
}

/// Recurse through rescue clause chain.
fn collect_assignments_in_rescue_node(
    rescue_node: &ruby_prism::RescueNode<'_>,
    assigns: &mut Vec<VarAssign>,
) {
    if let Some(stmts) = rescue_node.statements() {
        for s in stmts.body().iter() {
            collect_assignments_in_scope(&s, assigns);
        }
    }
    if let Some(subsequent) = rescue_node.subsequent() {
        collect_assignments_in_rescue_node(&subsequent, assigns);
    }
}

/// Check if a variable is referenced inside any example scope within the given
/// node tree. Walks through the example group body looking for example scopes
/// and checks if the variable is referenced inside them.
fn check_var_used_in_example_scopes(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        let no_recv = call.receiver().is_none();

        // Example scopes: it, before, let, subject, etc.
        if no_recv && is_example_scope(name) {
            // Check if the block body references the variable
            if let Some(blk) = call.block() {
                if let Some(bn) = blk.as_block_node() {
                    if block_body_references_var(bn, var_name) {
                        return true;
                    }
                }
            }
            // If the var is only in args (description, metadata), it's allowed
            return false;
        }

        // Includes methods: it_behaves_like, include_examples, etc.
        if no_recv && is_includes_method(name) {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                for (i, arg) in arg_list.iter().enumerate() {
                    if i == 0 {
                        // First arg (shared example name) is allowed
                        continue;
                    }
                    // Subsequent args in interpolated string/symbol are allowed
                    if is_interpolated_string_or_symbol(arg) {
                        continue;
                    }
                    if node_references_var(arg, var_name) {
                        return true;
                    }
                }
            }
            // Check block body of includes method
            if let Some(blk) = call.block() {
                if let Some(bn) = blk.as_block_node() {
                    if block_body_references_var(bn, var_name) {
                        return true;
                    }
                }
            }
            return false;
        }

        // Nested example groups: recurse into their body
        if no_recv && is_rspec_example_group(name) {
            if let Some(blk) = call.block() {
                if let Some(bn) = blk.as_block_node() {
                    if let Some(body) = bn.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            for s in stmts.body().iter() {
                                if check_var_used_in_example_scopes(&s, var_name) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
            return false;
        }

        // For other calls with blocks (e.g., `each do ... end`), recurse
        if let Some(blk) = call.block() {
            if let Some(bn) = blk.as_block_node() {
                if let Some(body) = bn.body() {
                    if let Some(stmts) = body.as_statements_node() {
                        for s in stmts.body().iter() {
                            if check_var_used_in_example_scopes(&s, var_name) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        return false;
    }

    // Recurse through control flow structures
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            for s in stmts.body().iter() {
                if check_var_used_in_example_scopes(&s, var_name) {
                    return true;
                }
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            if check_var_used_in_example_scopes(&subsequent, var_name) {
                return true;
            }
        }
        return false;
    }
    if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            for s in stmts.body().iter() {
                if check_var_used_in_example_scopes(&s, var_name) {
                    return true;
                }
            }
        }
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    if check_var_used_in_example_scopes(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            for s in stmts.body().iter() {
                if check_var_used_in_example_scopes(&s, var_name) {
                    return true;
                }
            }
        }
        return false;
    }

    // Case/When
    if let Some(case_node) = node.as_case_node() {
        for cond in case_node.conditions().iter() {
            if let Some(when_node) = cond.as_when_node() {
                if let Some(stmts) = when_node.statements() {
                    for s in stmts.body().iter() {
                        if check_var_used_in_example_scopes(&s, var_name) {
                            return true;
                        }
                    }
                }
            }
        }
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    if check_var_used_in_example_scopes(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    // Begin/Rescue
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for s in stmts.body().iter() {
                if check_var_used_in_example_scopes(&s, var_name) {
                    return true;
                }
            }
        }
        if let Some(rescue_clause) = begin_node.rescue_clause() {
            if check_var_in_rescue_scopes_inner(&rescue_clause, var_name) {
                return true;
            }
        }
        if let Some(else_clause) = begin_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    if check_var_used_in_example_scopes(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        if let Some(ensure_clause) = begin_node.ensure_clause() {
            if let Some(stmts) = ensure_clause.statements() {
                for s in stmts.body().iter() {
                    if check_var_used_in_example_scopes(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    // Parentheses
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            if check_var_used_in_example_scopes(&body, var_name) {
                return true;
            }
        }
        return false;
    }

    false
}

/// Check rescue chain for example scope references.
fn check_var_in_rescue_scopes_inner(
    rescue_node: &ruby_prism::RescueNode<'_>,
    var_name: &[u8],
) -> bool {
    if let Some(stmts) = rescue_node.statements() {
        for s in stmts.body().iter() {
            if check_var_used_in_example_scopes(&s, var_name) {
                return true;
            }
        }
    }
    if let Some(subsequent) = rescue_node.subsequent() {
        if check_var_in_rescue_scopes_inner(&subsequent, var_name) {
            return true;
        }
    }
    false
}

/// Check if a node is an interpolated string or symbol.
fn is_interpolated_string_or_symbol(node: &ruby_prism::Node<'_>) -> bool {
    node.as_interpolated_string_node().is_some() || node.as_interpolated_symbol_node().is_some()
}

/// Check if the body of a block references a variable. Does a deep recursive
/// search through all node types. Respects block parameter shadowing and
/// reassignment-before-use (if the variable is unconditionally written before
/// any read in the block, the outer variable is not actually referenced).
fn block_body_references_var(block: ruby_prism::BlockNode<'_>, var_name: &[u8]) -> bool {
    // If the block has a parameter with the same name, it shadows the outer var
    if block_has_param(&block, var_name) {
        return false;
    }

    let body = match block.body() {
        Some(b) => b,
        None => return false,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };

    // Check if the variable is reassigned before any read in the block.
    // If the first mention of the variable is an unconditional write (not a read),
    // then the outer variable is never actually referenced.
    if var_written_before_read_in_stmts(&stmts, var_name) {
        return false;
    }

    for stmt in stmts.body().iter() {
        if node_references_var(&stmt, var_name) {
            return true;
        }
    }
    false
}

/// Check if a variable is unconditionally written before being read in a
/// sequence of statements. Returns true if the variable is guaranteed to be
/// assigned before any read occurs, meaning the outer scope's value is never
/// used. This matches RuboCop's VariableForce flow-sensitive analysis for the
/// common case of reassignment at the beginning of a block.
fn var_written_before_read_in_stmts(
    stmts: &ruby_prism::StatementsNode<'_>,
    var_name: &[u8],
) -> bool {
    var_written_before_read_in_body(stmts, var_name)
}

/// Check if a node is an unconditional write to the given variable.
/// Matches direct `var = expr` assignments and multi-writes, but not
/// `var ||= expr` or conditional assignments (those might not execute).
/// Also recurses into `begin` blocks and parentheses, since those always
/// execute their contents.
fn is_unconditional_var_write(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(lw) = node.as_local_variable_write_node() {
        return lw.name().as_slice() == var_name;
    }
    if let Some(mw) = node.as_multi_write_node() {
        for target in mw.lefts().iter() {
            if let Some(lt) = target.as_local_variable_target_node() {
                if lt.name().as_slice() == var_name {
                    return true;
                }
            }
        }
        return false;
    }
    // `begin ... end` always executes — check if the first statement in the
    // begin body is an unconditional write (recursively).
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            return var_written_before_read_in_body(&stmts, var_name);
        }
    }
    // Parenthesized expressions always execute
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            if let Some(stmts) = body.as_statements_node() {
                return var_written_before_read_in_body(&stmts, var_name);
            }
        }
    }
    false
}

/// Check if a variable is written before read in a sequence of statements.
/// Extracted from `var_written_before_read_in_stmts` for reuse.
fn var_written_before_read_in_body(
    stmts: &ruby_prism::StatementsNode<'_>,
    var_name: &[u8],
) -> bool {
    for stmt in stmts.body().iter() {
        if is_unconditional_var_write(&stmt, var_name) {
            return true;
        }
        if node_reads_var(&stmt, var_name) {
            return false;
        }
    }
    false
}

/// Check if a node reads (but doesn't write) the given variable.
/// Returns true if the variable name appears as a read anywhere in the node.
fn node_reads_var(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(lv) = node.as_local_variable_read_node() {
        return lv.name().as_slice() == var_name;
    }
    // For writes, check if the RHS reads the variable
    if let Some(lw) = node.as_local_variable_write_node() {
        if lw.name().as_slice() == var_name {
            // The write itself doesn't read, but the RHS might
            return node_reads_var(&lw.value(), var_name);
        }
    }
    // For all other node types, delegate to the full reference checker
    // (this is a conservative check - any reference counts as a read)
    node_references_var(node, var_name)
}

/// Check if a block has a parameter with the given name (for shadowing).
fn block_has_param(block: &ruby_prism::BlockNode<'_>, var_name: &[u8]) -> bool {
    let params = match block.parameters() {
        Some(p) => p,
        None => return false,
    };
    let params_node = match params.as_block_parameters_node() {
        Some(p) => p,
        None => return false,
    };
    let inner = match params_node.parameters() {
        Some(p) => p,
        None => return false,
    };
    for p in inner.requireds().iter() {
        if let Some(rp) = p.as_required_parameter_node() {
            if rp.name().as_slice() == var_name {
                return true;
            }
        }
    }
    for p in inner.optionals().iter() {
        if let Some(op) = p.as_optional_parameter_node() {
            if op.name().as_slice() == var_name {
                return true;
            }
        }
    }
    if let Some(rest) = inner.rest() {
        if let Some(rp) = rest.as_rest_parameter_node() {
            if let Some(name) = rp.name() {
                if name.as_slice() == var_name {
                    return true;
                }
            }
        }
    }
    for p in inner.keywords().iter() {
        if let Some(kp) = p.as_required_keyword_parameter_node() {
            if kp.name().as_slice() == var_name {
                return true;
            }
        }
        if let Some(kp) = p.as_optional_keyword_parameter_node() {
            if kp.name().as_slice() == var_name {
                return true;
            }
        }
    }
    false
}

/// Deep recursive check: does any node in the subtree reference the variable?
fn node_references_var(node: &ruby_prism::Node<'_>, var_name: &[u8]) -> bool {
    if let Some(lv) = node.as_local_variable_read_node() {
        if lv.name().as_slice() == var_name {
            return true;
        }
        return false;
    }

    // Local variable write: only check the RHS for references
    if let Some(lw) = node.as_local_variable_write_node() {
        return node_references_var(&lw.value(), var_name);
    }

    // For call nodes with blocks, check if block params shadow the variable
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
                if !block_has_param(&bn, var_name) {
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
        return false;
    }

    // Instance variable write: check RHS
    if let Some(iw) = node.as_instance_variable_write_node() {
        return node_references_var(&iw.value(), var_name);
    }

    // Local variable or-write / and-write
    if let Some(ow) = node.as_local_variable_or_write_node() {
        return node_references_var(&ow.value(), var_name);
    }
    if let Some(aw) = node.as_local_variable_and_write_node() {
        return node_references_var(&aw.value(), var_name);
    }

    // Multi-write
    if let Some(mw) = node.as_multi_write_node() {
        return node_references_var(&mw.value(), var_name);
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
        return false;
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
        return false;
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
        return false;
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
        return false;
    }

    // Parentheses node
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            return node_references_var(&body, var_name);
        }
        return false;
    }

    // And/Or nodes
    if let Some(and_node) = node.as_and_node() {
        return node_references_var(&and_node.left(), var_name)
            || node_references_var(&and_node.right(), var_name);
    }
    if let Some(or_node) = node.as_or_node() {
        return node_references_var(&or_node.left(), var_name)
            || node_references_var(&or_node.right(), var_name);
    }

    // Interpolated strings/symbols
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
        return false;
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
        return false;
    }

    // Array
    if let Some(arr) = node.as_array_node() {
        for elem in arr.elements().iter() {
            if node_references_var(&elem, var_name) {
                return true;
            }
        }
        return false;
    }

    // Hash / KeywordHash
    if let Some(hash) = node.as_hash_node() {
        for elem in hash.elements().iter() {
            if let Some(assoc) = elem.as_assoc_node() {
                if node_references_var(&assoc.key(), var_name)
                    || node_references_var(&assoc.value(), var_name)
                {
                    return true;
                }
            }
            if let Some(splat) = elem.as_assoc_splat_node() {
                if let Some(expr) = splat.value() {
                    if node_references_var(&expr, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }
    if let Some(kw) = node.as_keyword_hash_node() {
        for elem in kw.elements().iter() {
            if let Some(assoc) = elem.as_assoc_node() {
                if node_references_var(&assoc.key(), var_name)
                    || node_references_var(&assoc.value(), var_name)
                {
                    return true;
                }
            }
            if let Some(splat) = elem.as_assoc_splat_node() {
                if let Some(expr) = splat.value() {
                    if node_references_var(&expr, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    // Splat / AssocSplat
    if let Some(splat) = node.as_splat_node() {
        if let Some(expr) = splat.expression() {
            return node_references_var(&expr, var_name);
        }
        return false;
    }
    if let Some(assoc_splat) = node.as_assoc_splat_node() {
        if let Some(expr) = assoc_splat.value() {
            return node_references_var(&expr, var_name);
        }
        return false;
    }

    // Embedded statements
    if let Some(embedded) = node.as_embedded_statements_node() {
        if let Some(stmts) = embedded.statements() {
            for s in stmts.body().iter() {
                if node_references_var(&s, var_name) {
                    return true;
                }
            }
        }
        return false;
    }

    // Case/When
    if let Some(case_node) = node.as_case_node() {
        if let Some(pred) = case_node.predicate() {
            if node_references_var(&pred, var_name) {
                return true;
            }
        }
        for cond in case_node.conditions().iter() {
            if let Some(when_node) = cond.as_when_node() {
                for c in when_node.conditions().iter() {
                    if node_references_var(&c, var_name) {
                        return true;
                    }
                }
                if let Some(stmts) = when_node.statements() {
                    for s in stmts.body().iter() {
                        if node_references_var(&s, var_name) {
                            return true;
                        }
                    }
                }
            }
        }
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    if node_references_var(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    // Begin/Rescue/Ensure
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for s in stmts.body().iter() {
                if node_references_var(&s, var_name) {
                    return true;
                }
            }
        }
        if let Some(rescue_clause) = begin_node.rescue_clause() {
            if node_references_var_in_rescue_inner(&rescue_clause, var_name) {
                return true;
            }
        }
        if let Some(else_clause) = begin_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for s in stmts.body().iter() {
                    if node_references_var(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        if let Some(ensure_clause) = begin_node.ensure_clause() {
            if let Some(stmts) = ensure_clause.statements() {
                for s in stmts.body().iter() {
                    if node_references_var(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    // Rescue node
    if let Some(rescue_node) = node.as_rescue_node() {
        return node_references_var_in_rescue_inner(&rescue_node, var_name);
    }

    // While/Until
    if let Some(while_node) = node.as_while_node() {
        if node_references_var(&while_node.predicate(), var_name) {
            return true;
        }
        if let Some(stmts) = while_node.statements() {
            for s in stmts.body().iter() {
                if node_references_var(&s, var_name) {
                    return true;
                }
            }
        }
        return false;
    }
    if let Some(until_node) = node.as_until_node() {
        if node_references_var(&until_node.predicate(), var_name) {
            return true;
        }
        if let Some(stmts) = until_node.statements() {
            for s in stmts.body().iter() {
                if node_references_var(&s, var_name) {
                    return true;
                }
            }
        }
        return false;
    }

    // Range
    if let Some(range) = node.as_range_node() {
        if let Some(left) = range.left() {
            if node_references_var(&left, var_name) {
                return true;
            }
        }
        if let Some(right) = range.right() {
            if node_references_var(&right, var_name) {
                return true;
            }
        }
        return false;
    }

    // Lambda
    if let Some(lambda) = node.as_lambda_node() {
        if let Some(body) = lambda.body() {
            if let Some(stmts) = body.as_statements_node() {
                for s in stmts.body().iter() {
                    if node_references_var(&s, var_name) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    // Defined?
    if let Some(def) = node.as_defined_node() {
        return node_references_var(&def.value(), var_name);
    }

    // StatementsNode
    if let Some(stmts) = node.as_statements_node() {
        for s in stmts.body().iter() {
            if node_references_var(&s, var_name) {
                return true;
            }
        }
        return false;
    }

    // Yield
    if let Some(yield_node) = node.as_yield_node() {
        if let Some(args) = yield_node.arguments() {
            for arg in args.arguments().iter() {
                if node_references_var(&arg, var_name) {
                    return true;
                }
            }
        }
        return false;
    }

    false
}

/// Check rescue chain for variable references.
fn node_references_var_in_rescue_inner(
    rescue_node: &ruby_prism::RescueNode<'_>,
    var_name: &[u8],
) -> bool {
    if let Some(stmts) = rescue_node.statements() {
        for s in stmts.body().iter() {
            if node_references_var(&s, var_name) {
                return true;
            }
        }
    }
    if let Some(subsequent) = rescue_node.subsequent() {
        if node_references_var_in_rescue_inner(&subsequent, var_name) {
            return true;
        }
    }
    false
}

/// Check if a method name represents an example scope
fn is_example_scope(name: &[u8]) -> bool {
    let s = std::str::from_utf8(name).unwrap_or("");
    crate::cop::util::RSPEC_EXAMPLES.contains(&s)
        || crate::cop::util::RSPEC_HOOKS.contains(&s)
        || crate::cop::util::RSPEC_LETS.contains(&s)
        || crate::cop::util::RSPEC_SUBJECTS.contains(&s)
}

/// Check if a method name is an RSpec includes method
fn is_includes_method(name: &[u8]) -> bool {
    matches!(
        name,
        b"it_behaves_like" | b"it_should_behave_like" | b"include_examples" | b"include_context"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LeakyLocalVariable, "cops/rspec/leaky_local_variable");
}
