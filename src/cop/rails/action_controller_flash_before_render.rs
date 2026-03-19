/// Rails/ActionControllerFlashBeforeRender
///
/// Investigation findings (2026-03-18):
/// - Root cause 1: `default_include` was set to `["app/controllers/**/*.rb"]` but the vendor
///   config has NO Include restriction. The cop should run on all Ruby files; class-inheritance
///   detection handles scoping. This caused 0% match rate on the corpus.
/// - Root cause 2: Implicit render was not handled. RuboCop fires when `flash[:x] = val` appears
///   in a def/block with no subsequent siblings AND no redirect_to following the parent — the
///   implicit render case. The old code required `has_render && !has_redirect`, missing this.
/// - Root cause 3: `::ApplicationController` (ConstantPathNode with nil parent) and
///   `::ActionController::Base` were not handled. These are `ConstantPathNode` nodes, not
///   `ConstantReadNode`, so the old check missed the `::` prefix form.
/// - Root cause 4: Flash inside an if/rescue branch with render at the outer def level was not
///   detected. The RuboCop impl walks up to the if/rescue ancestor and checks its siblings.
/// - Root cause 5: `before_action do` blocks at class level need to be visited, not just def
///   nodes. The visitor now also checks block bodies inside class-level call nodes.
/// - Root cause 6 (FP=399): Heuristic matching ANY superclass ending in `Controller` caused FPs
///   on qualified names like `Admin::ApplicationController`. RuboCop only matches bare
///   `ApplicationController`, `::ApplicationController`, `ActionController::Base`, and
///   `::ActionController::Base`. Removed the heuristic.
/// - Root cause 7 (FN=48): `contains_redirect` was recursive, searching inside blocks for
///   `redirect_to`. RuboCop's `use_redirect_to?` only checks direct siblings (non-recursive)
///   and only matches `redirect_to` (not `redirect_back`). Changed to non-recursive
///   `is_redirect_sibling` that matches RuboCop's behavior.
/// - Root cause 8 (FP=20): outer_siblings from the method body were propagated through all
///   nesting levels of if/rescue/block. RuboCop only checks the FIRST if/rescue ancestor's
///   right_siblings, which may be empty for deeply nested structures. Fixed by passing the
///   nested node's remaining siblings within its current branch context instead.
/// - Root cause 9 (FN): `UnlessNode` was not handled — Prism has separate `UnlessNode` and
///   `IfNode` types. Added unless handling in all places that handle if.
/// - Root cause 10 (FP=13): In respond_to format blocks (format.html do...end), the
///   is_if_rescue_branch flag was propagated through blocks, causing render in sibling format
///   blocks (e.g. format.api) to be treated as the if ancestor's right_siblings. Fixed by
///   making blocks transparent: when in if/rescue context, blocks inherit the if/rescue's
///   outer_siblings rather than the block's own sibling format blocks.
/// - Root cause 11 (FN=11): Else clauses in check_if_node_impl were passed empty outer_siblings
///   (&[]) instead of the if node's outer_siblings. Flash in else branches couldn't see render
///   in the if node's right siblings (e.g. respond_to with render after if/else). Fixed by
///   passing outer_siblings through to else clause processing.
/// - Root cause 12 (FN): Implicit render in block bodies (flash alone in each/tap blocks) was
///   not detected. When inner_remaining is empty in a block, used outer_has_render instead of
///   the implicit render check (!outer_has_redirect). Fixed to match RuboCop's
///   context.right_siblings.empty? && !use_redirect_to?(context.parent) logic.
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct ActionControllerFlashBeforeRender;

impl Cop for ActionControllerFlashBeforeRender {
    fn name(&self) -> &'static str {
        "Rails/ActionControllerFlashBeforeRender"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    // No Include restriction — vendor config/default.yml has none.
    // Class-inheritance detection scopes to ActionController descendants.

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = FlashVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_action_controller: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct FlashVisitor<'a> {
    cop: &'a ActionControllerFlashBeforeRender,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    in_action_controller: bool,
}

impl<'pr> Visit<'pr> for FlashVisitor<'_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let was_in_controller = self.in_action_controller;
        if is_action_controller_class(node) {
            self.in_action_controller = true;
        }
        if self.in_action_controller {
            // Manually walk the class body to find def nodes and class-level blocks
            // (e.g. `before_action do ... end`). This avoids double-visiting that
            // would occur if we used the default visitor alongside manual recursion.
            if let Some(body) = node.body() {
                if let Some(stmts) = body.as_statements_node() {
                    for stmt in stmts.body().iter() {
                        if let Some(def_node) = stmt.as_def_node() {
                            // Instance method
                            if def_node.receiver().is_none() {
                                self.check_def_body(&def_node);
                            }
                        } else if let Some(call_node) = stmt.as_call_node() {
                            // Class-level call with block: `before_action do ... end`
                            if let Some(block) = call_node.block() {
                                if let Some(block_node) = block.as_block_node() {
                                    if let Some(body_inner) = block_node.body() {
                                        if let Some(block_stmts) = body_inner.as_statements_node() {
                                            let body_nodes: Vec<_> =
                                                block_stmts.body().iter().collect();
                                            self.check_statements(&body_nodes);
                                        }
                                    }
                                }
                            }
                        } else if let Some(nested_class) = stmt.as_class_node() {
                            // Handle nested classes
                            self.visit_class_node(&nested_class);
                        }
                    }
                }
            }
        } else {
            // Not in a controller — still recurse to find nested classes
            if let Some(body) = node.body() {
                self.visit(&body);
            }
        }
        self.in_action_controller = was_in_controller;
    }
}

impl FlashVisitor<'_> {
    fn check_def_body(&mut self, def_node: &ruby_prism::DefNode<'_>) {
        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_nodes: Vec<ruby_prism::Node<'_>> = stmts.body().iter().collect();
        self.check_statements(&body_nodes);
    }

    /// Check a list of sibling statements for flash-before-render patterns.
    ///
    /// This is the top-level checker for def bodies and class-level block bodies.
    /// For each statement:
    /// - If it is a flash assignment: check if any subsequent sibling contains render,
    ///   OR if there are no subsequent siblings and no redirect among siblings → implicit render.
    /// - If it is an if/unless/rescue block: recurse into its branches, using the
    ///   remaining siblings as the outer context for render detection (matching RuboCop's
    ///   behavior of checking the first if/rescue ancestor's right_siblings).
    fn check_statements(&mut self, stmts: &[ruby_prism::Node<'_>]) {
        for (i, stmt) in stmts.iter().enumerate() {
            let remaining = &stmts[i + 1..];

            // Check if this statement is a flash assignment (top-level)
            if let Some(flash_loc) = get_flash_assignment(stmt) {
                let has_render = remaining.iter().any(|s| contains_render(s));
                let has_redirect = remaining.iter().any(|s| is_redirect_sibling(s));

                // Offense if:
                // (a) explicit render follows without redirect, or
                // (b) no siblings at all (implicit render) and no redirect
                let is_offense = if remaining.is_empty() {
                    // Implicit render: no explicit render or redirect after flash
                    !has_redirect
                } else {
                    has_render && !has_redirect
                };

                if is_offense {
                    self.emit_diagnostic(flash_loc);
                }
            }

            // Flash inside an if/else branch: check if render appears in the outer context
            if let Some(if_node) = stmt.as_if_node() {
                self.check_if_node_with_outer(&if_node, remaining);
            }

            // Flash inside an unless block
            if let Some(unless_node) = stmt.as_unless_node() {
                self.check_unless_node_with_outer(&unless_node, remaining);
            }

            // Flash inside a begin/rescue block: similar outer-context check
            if let Some(begin_node) = stmt.as_begin_node() {
                self.check_begin_node_with_outer(&begin_node, remaining);
            }

            // Recurse into respond_to/format blocks (nested block bodies).
            // Pass outer siblings so implicit-render detection can see outer redirect/render.
            if let Some(call_node) = stmt.as_call_node() {
                if let Some(block) = call_node.block() {
                    self.check_block_body_with_outer(&block, remaining, false);
                }
            }
        }
    }

    /// Check an if-node's branches. Flash assignments inside branches are offenses
    /// if the outer siblings (the if node's right siblings) contain render.
    ///
    /// RuboCop uses Parser AST where single-statement if bodies place the child
    /// directly as a child of the if node (no begin wrapper). This means the child's
    /// right_siblings include the else clause. For multi-statement bodies, a begin
    /// wrapper isolates children from the else clause. We replicate this by collecting
    /// "else siblings" — nodes from the else/elsif branches — and including them
    /// in the outer context when the if body has exactly one statement.
    fn check_if_node_with_outer(
        &mut self,
        if_node: &ruby_prism::IfNode<'_>,
        outer_siblings: &[ruby_prism::Node<'_>],
    ) {
        self.check_if_node_impl(if_node, outer_siblings, false);
    }

    /// Core implementation for checking an if node's branches.
    ///
    /// `parent_render_flag`: extra render context from a parent single-statement
    /// branch. In Parser AST, when an if is the sole child of another if's body
    /// (no begin wrapper), its right_siblings include the parent's else clause.
    /// This flag carries that information.
    fn check_if_node_impl(
        &mut self,
        if_node: &ruby_prism::IfNode<'_>,
        outer_siblings: &[ruby_prism::Node<'_>],
        parent_render_flag: bool,
    ) {
        // In Parser AST, when the if body is a single node, it's placed directly
        // as a child of the if (no begin wrapper), so its right_siblings include
        // the else body. We compute `else_has_render` for this case.
        let single_stmt_body = if_node
            .statements()
            .is_some_and(|s| s.body().iter().count() == 1);
        let else_has_render = if single_stmt_body {
            if_node.subsequent().is_some_and(|s| contains_render(&s))
        } else {
            false
        };

        // Combine with parent render flag
        let effective_render = parent_render_flag || else_has_render;

        // Check flash in the if-branch body.
        // When the body is a single if/unless node, directly recurse into it with
        // the effective_render flag (which includes this if's else clause render).
        // This avoids going through check_branch_stmts_with_outer which would lose
        // the parent_render_flag context.
        if let Some(stmts) = if_node.statements() {
            let body_nodes: Vec<_> = stmts.body().iter().collect();
            if body_nodes.len() == 1 {
                if let Some(nested_if) = body_nodes[0].as_if_node() {
                    // In Parser AST, single-stmt if body has no begin wrapper, so
                    // the nested if's right_siblings = parent's else clause (captured
                    // in effective_render). The parent's outer_siblings do NOT leak
                    // through — pass &[] to match RuboCop's ancestor walk behavior.
                    self.check_if_node_impl(&nested_if, &[], effective_render);
                } else if let Some(nested_unless) = body_nodes[0].as_unless_node() {
                    self.check_unless_node_with_outer(&nested_unless, &[]);
                } else {
                    self.check_branch_stmts_with_outer(&body_nodes, outer_siblings, true);
                }
            } else {
                self.check_branch_stmts_with_outer(&body_nodes, outer_siblings, true);
            }
        }
        // Check subsequent elsif/else clauses.
        // For elsif/else, outer_siblings is empty (in Parser AST, elsif is the
        // last child of the parent if). But parent_render_flag still applies.
        if let Some(subsequent) = if_node.subsequent() {
            if let Some(elsif) = subsequent.as_if_node() {
                self.check_if_node_impl(&elsif, &[], parent_render_flag);
            }
            if let Some(else_clause) = subsequent.as_else_node() {
                if let Some(stmts) = else_clause.statements() {
                    let body_nodes: Vec<_> = stmts.body().iter().collect();
                    // When parent_render_flag is set, flash in this else body
                    // should be flagged because in Parser AST the parent if's
                    // right_siblings (which include render) apply here too.
                    if parent_render_flag {
                        for (i, stmt) in body_nodes.iter().enumerate() {
                            if let Some(flash_loc) = get_flash_assignment(stmt) {
                                let remaining = &body_nodes[i + 1..];
                                let has_redirect = remaining.iter().any(|s| is_redirect_sibling(s));
                                if !has_redirect {
                                    self.emit_diagnostic(flash_loc);
                                }
                            }
                        }
                    }
                    // Pass outer_siblings so else branch can see the if node's
                    // outer context (e.g., render/respond_to after the if/else).
                    self.check_branch_stmts_with_outer(&body_nodes, outer_siblings, true);
                }
            }
        }
    }

    /// Check an unless-node's body. Mirrors check_if_node_with_outer.
    fn check_unless_node_with_outer(
        &mut self,
        unless_node: &ruby_prism::UnlessNode<'_>,
        outer_siblings: &[ruby_prism::Node<'_>],
    ) {
        if let Some(stmts) = unless_node.statements() {
            let body_nodes: Vec<_> = stmts.body().iter().collect();
            self.check_branch_stmts_with_outer(&body_nodes, outer_siblings, true);
        }
        // unless ... else ... end (rare but possible)
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                let body_nodes: Vec<_> = stmts.body().iter().collect();
                self.check_branch_stmts_with_outer(&body_nodes, &[], true);
            }
        }
    }

    fn check_begin_node_with_outer(
        &mut self,
        begin_node: &ruby_prism::BeginNode<'_>,
        outer_siblings: &[ruby_prism::Node<'_>],
    ) {
        if let Some(stmts) = begin_node.statements() {
            let body_nodes: Vec<_> = stmts.body().iter().collect();
            self.check_branch_stmts_with_outer(&body_nodes, outer_siblings, true);
        }
        // For rescue clauses: RuboCop's each_ancestor(:rescue) finds the rescue node,
        // and rescue.right_siblings within the begin is empty. So pass empty outer.
        if let Some(rescue) = begin_node.rescue_clause() {
            self.check_rescue_with_outer(&rescue, &[]);
        }
    }

    fn check_rescue_with_outer(
        &mut self,
        rescue: &ruby_prism::RescueNode<'_>,
        outer_siblings: &[ruby_prism::Node<'_>],
    ) {
        if let Some(stmts) = rescue.statements() {
            let body_nodes: Vec<_> = stmts.body().iter().collect();
            self.check_branch_stmts_with_outer(&body_nodes, outer_siblings, true);
        }
        if let Some(subsequent) = rescue.subsequent() {
            self.check_rescue_with_outer(&subsequent, outer_siblings);
        }
    }

    /// Check statements inside a branch or block body with outer context awareness.
    ///
    /// `is_if_rescue_branch`: true for if/rescue branches, false for block bodies.
    ///
    /// For **if/rescue branches** (`is_if_rescue_branch=true`):
    /// RuboCop walks up to the if/rescue ancestor and checks its right siblings
    /// for render. It does NOT check for render within the same branch.
    ///
    /// For **block bodies** (`is_if_rescue_branch=false`):
    /// Blocks are treated like def bodies — flash's inner siblings ARE checked
    /// for render. If render is found, offense. Otherwise falls back to outer.
    fn check_branch_stmts_with_outer(
        &mut self,
        branch_stmts: &[ruby_prism::Node<'_>],
        outer_siblings: &[ruby_prism::Node<'_>],
        is_if_rescue_branch: bool,
    ) {
        let outer_has_render = outer_siblings.iter().any(|s| contains_render(s));

        for (i, stmt) in branch_stmts.iter().enumerate() {
            let inner_remaining = &branch_stmts[i + 1..];

            if let Some(flash_loc) = get_flash_assignment(stmt) {
                // RuboCop's use_redirect_to? checks flash's direct siblings for redirect_to
                let inner_has_redirect = inner_remaining.iter().any(|s| is_redirect_sibling(s));

                // If redirect_to appears after flash in the same branch → no offense
                if inner_has_redirect {
                    continue;
                }

                let is_offense = if is_if_rescue_branch {
                    // For if/rescue: only check outer siblings for render.
                    // No implicit render from branches.
                    outer_has_render
                } else {
                    // For block bodies: check inner siblings for render first (like def level).
                    let inner_has_render = inner_remaining.iter().any(|s| contains_render(s));
                    if inner_has_render {
                        true
                    } else if inner_remaining.is_empty() {
                        // Flash is alone/last in block — implicit render.
                        // RuboCop checks context.parent.right_siblings for redirect.
                        let outer_has_redirect =
                            outer_siblings.iter().any(|s| is_redirect_sibling(s));
                        !outer_has_redirect
                    } else {
                        false
                    }
                };

                if is_offense {
                    self.emit_diagnostic(flash_loc);
                }
            }

            // Recurse into nested if/unless/rescue/block inside the branch.
            // Pass `inner_remaining` (the remaining siblings within this branch)
            // as the outer context, not the original `outer_siblings`.
            if let Some(nested_if) = stmt.as_if_node() {
                self.check_if_node_with_outer(&nested_if, inner_remaining);
            }
            if let Some(nested_unless) = stmt.as_unless_node() {
                self.check_unless_node_with_outer(&nested_unless, inner_remaining);
            }
            if let Some(nested_begin) = stmt.as_begin_node() {
                self.check_begin_node_with_outer(&nested_begin, inner_remaining);
            }
            if let Some(call_node) = stmt.as_call_node() {
                if let Some(block) = call_node.block() {
                    // In RuboCop, blocks are transparent to each_ancestor(:if, :rescue).
                    // When inside an if/rescue context, the block inherits the if/rescue's
                    // outer siblings, not the block's own siblings within the parent scope.
                    let block_outer = if is_if_rescue_branch {
                        outer_siblings
                    } else {
                        inner_remaining
                    };
                    self.check_block_body_with_outer(&block, block_outer, is_if_rescue_branch);
                }
            }
        }
    }

    /// Check a block body with awareness of the outer sibling context.
    /// `in_if_rescue_context`: when true, the block is inside an if/rescue branch.
    /// RuboCop's ancestor walk is transparent to blocks — if flash is inside a
    /// block that's inside an if, the if ancestor is found, not the block.
    /// So blocks inside if/rescue use `is_if_rescue_branch=true` to only check
    /// outer siblings for render, not inner block siblings.
    fn check_block_body_with_outer(
        &mut self,
        block: &ruby_prism::Node<'_>,
        outer_siblings: &[ruby_prism::Node<'_>],
        in_if_rescue_context: bool,
    ) {
        if let Some(block_node) = block.as_block_node() {
            if let Some(body) = block_node.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let body_nodes: Vec<_> = stmts.body().iter().collect();
                    self.check_branch_stmts_with_outer(
                        &body_nodes,
                        outer_siblings,
                        in_if_rescue_context,
                    );
                }
            }
        }
    }

    fn emit_diagnostic(&mut self, flash_loc: usize) {
        let (line, column) = self.source.offset_to_line_col(flash_loc);
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Use `flash.now` before `render`.".to_string(),
        ));
    }
}

/// Check if a class inherits from ApplicationController, ActionController::Base,
/// or their top-level (::) variants.
fn is_action_controller_class(class: &ruby_prism::ClassNode<'_>) -> bool {
    let superclass = match class.superclass() {
        Some(s) => s,
        None => return false,
    };

    // `ApplicationController` (bare constant)
    if let Some(c) = superclass.as_constant_read_node() {
        if c.name().as_slice() == b"ApplicationController" {
            return true;
        }
    }

    // `ActionController::Base` (qualified path)
    if let Some(cp) = superclass.as_constant_path_node() {
        if let Some(name) = cp.name() {
            if name.as_slice() == b"Base" {
                if let Some(parent) = cp.parent() {
                    if let Some(c) = parent.as_constant_read_node() {
                        if c.name().as_slice() == b"ActionController" {
                            return true;
                        }
                    }
                }
            }
        }
    }

    // `::ApplicationController` (top-level constant path, no parent)
    if let Some(cp) = superclass.as_constant_path_node() {
        if cp.parent().is_none() {
            if let Some(name) = cp.name() {
                if name.as_slice() == b"ApplicationController" {
                    return true;
                }
            }
        }
    }

    // `::ActionController::Base` (top-level qualified path)
    if let Some(cp) = superclass.as_constant_path_node() {
        if let Some(name) = cp.name() {
            if name.as_slice() == b"Base" {
                if let Some(parent) = cp.parent() {
                    if let Some(parent_cp) = parent.as_constant_path_node() {
                        if parent_cp.parent().is_none() {
                            if let Some(parent_name) = parent_cp.name() {
                                if parent_name.as_slice() == b"ActionController" {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    false
}

/// Check if a node is `flash[:key] = value` and return the flash receiver location offset.
fn get_flash_assignment(node: &ruby_prism::Node<'_>) -> Option<usize> {
    let call = node.as_call_node()?;
    if call.name().as_slice() != b"[]=" {
        return None;
    }
    let receiver = call.receiver()?;
    let recv_call = receiver.as_call_node()?;
    if recv_call.name().as_slice() != b"flash" || recv_call.receiver().is_some() {
        return None;
    }
    let loc = recv_call.message_loc().unwrap_or(recv_call.location());
    Some(loc.start_offset())
}

/// Check if a node contains a `render` call (no receiver).
fn contains_render(node: &ruby_prism::Node<'_>) -> bool {
    let mut finder = CallFinder {
        method: b"render",
        found: false,
    };
    finder.visit(node);
    finder.found
}

/// Check if a node IS a `redirect_to` call (no receiver), non-recursive.
/// Also unwraps `return redirect_to ...` (ReturnNode with a single child).
/// Matches RuboCop's `use_redirect_to?` which only checks direct siblings,
/// not recursing into blocks/if/etc, and only matches `redirect_to` (not `redirect_back`).
fn is_redirect_sibling(node: &ruby_prism::Node<'_>) -> bool {
    // Direct `redirect_to ...`
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none() && call.name().as_slice() == b"redirect_to" {
            return true;
        }
    }
    // `return redirect_to ...`
    if let Some(ret) = node.as_return_node() {
        if let Some(args) = ret.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() == 1 {
                if let Some(call) = arg_list[0].as_call_node() {
                    if call.receiver().is_none() && call.name().as_slice() == b"redirect_to" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

struct CallFinder<'a> {
    method: &'a [u8],
    found: bool,
}

impl<'pr> Visit<'pr> for CallFinder<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == self.method && node.receiver().is_none() {
            self.found = true;
        }
        if !self.found {
            ruby_prism::visit_call_node(self, node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ActionControllerFlashBeforeRender,
        "cops/rails/action_controller_flash_before_render"
    );
}
