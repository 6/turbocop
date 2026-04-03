/// Rails/ActionControllerFlashBeforeRender
///
/// Investigation findings (2026-03-30):
/// - Root cause 19 (FN): nested local assignments like
///   `query = flash[:query] = params[:query].to_s` were skipped because only
///   statement-level `flash[:x] = ...` nodes were checked.
/// - Root cause 20 (FN): parent single-child `if` render context was not
///   propagated into embedded `respond_to do ... end` blocks, so flash inside
///   those blocks could not see the outer `else` render that RuboCop uses.
/// - Root cause 21 (FN): `lambda do ... end` assigned to a local variable was
///   treated like a statement-level block, incorrectly allowing a later outer
///   `redirect_to` to suppress the implicit-render offense.
/// - Root cause 22 (FP): `case ... else` branches with `flash` followed by an
///   outer `redirect_to` were treated like `when` branches. RuboCop only allows
///   that suppression for the `else` branch shape seen in the corpus.
/// - Root cause 23 (FP=2): `check_begin_body_with_rescue` passed rescue_context_nodes
///   (the rescue clause nodes containing render) as outer_siblings to nested if/unless.
///   In RuboCop, `each_ancestor(:if, :rescue)` finds the if/unless ancestor BEFORE
///   the rescue ancestor. The if/unless node's right_siblings are the remaining begin
///   body stmts, not the rescue clause content. Fixed by passing inner_remaining
///   (remaining begin body stmts) to nested if/unless instead of rescue_context_nodes.
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
///
/// ## Investigation (2026-03-19): FP=0, FN=6 — second fix attempt
///
/// Previous fix (775de516) reverted due to 5 new FPs. Root cause: `extra_outer_render`
/// parameter was set from `effective_render` (which includes the current if's own
/// else_has_render) instead of just `parent_render_flag`. The current if's else clause
/// render should only propagate to NESTED single-child if bodies (where Parser AST
/// flattens the if body), not to the current if's own body statements.
///
/// Fixed all three root causes with more targeted propagation:
/// - Root cause 13: def-with-rescue → BeginNode handling in check_def_body
/// - Root cause 14: multi-statement block implicit render → i > 0 check
/// - Root cause 15: nested single-child if with parent else render → pass
///   parent_render_flag (not effective_render) to check_branch_stmts_impl
///
/// Remaining 1 FN (browsermedia portlet.rb:228) is a RuboCop over-match:
/// `Cms::Portlet < ActiveRecord::Base` is not a controller but RuboCop's
/// `def_node_search :action_controller?` matches ANY reference in the class subtree.
///
/// ## Investigation (2026-03-19): FP=4, FN=1 — third fix
///
/// FP=4: All four FPs were flash as the last statement in a def-with-rescue body
/// (with or without ensure). In RuboCop's Parser AST, `each_ancestor(:if, :rescue)`
/// finds the rescue ancestor and only checks its right_siblings (ensure body or empty)
/// for render — implicit render detection is suppressed. nitrocop's `check_statements`
/// was incorrectly triggering implicit render for the last body statement. Fixed by
/// using `check_branch_stmts_with_outer` with `is_if_rescue_branch=true` and ensure
/// body nodes as outer context, matching RuboCop's rescue ancestor behavior.
///
/// FN=1: Fixed. `Cms::Portlet < ActiveRecord::Base` references
/// `ActionController::Base.view_paths` (line 121) in its body. RuboCop's
/// `def_node_search :action_controller?` searches the entire class subtree.
/// Added `class_body_references_action_controller` subtree search to match.
/// Initial attempt caused 2 new FNs on `rails__rails` because the manual
/// walker didn't handle modules (test files nest controllers inside modules
/// like `module ::Blog; class PostsController < ActionController::Base`).
/// Fixed by: (a) resetting `in_action_controller` per-class so each class
/// qualifies independently (matches RuboCop's `find_ancestor(:class)`),
/// and (b) using the full visitor for recursion after manually checking
/// the class's own methods, so modules and other nested structures are
/// properly traversed.
///
/// ## Reverted fix attempt (2026-03-23, commit fbedda13)
///
/// Attempted to fix FP and FN patterns in case/when and begin/rescue contexts.
/// Introduced FP=9 on standard corpus; reverted in 1bf1bea3.
///
/// **FP=9 (unused outer_siblings in case/when handler):** The newly added
/// `check_case_branch_stmts` method accepted `_outer_siblings` (prefixed with
/// underscore — unused). When flash was the last statement in a `when` body
/// (`inner_remaining.is_empty()`), the code unconditionally returned `true`,
/// treating it as implicit render without checking whether the case statement's
/// outer siblings contain `redirect_to` or `redirect_back`. In RuboCop, the
/// `use_redirect_to?` check walks up to the parent's right_siblings. Fix: use
/// `outer_siblings` and check for redirect_to when flash is the last statement
/// in a when body. Additionally, the begin/rescue suppression of outer_siblings
/// needs revisiting — `redirect_to` AFTER a begin block should still be visible.
///
/// ## Reverted fix attempt (2026-03-24, commit 998e83c7)
///
/// Attempted two fixes: (1) FP=4 by passing `&[]` as outer_siblings for begin
/// body when rescue exists, (2) FN case/when by adding `check_case_node_with_outer`
/// and `check_case_branch_stmts`. Introduced FP=28 (+24) on corpus; reverted in
/// a6172038.
///
/// **FP=24 (begin/rescue outer_siblings blanket suppression):** Passing `&[]` as
/// outer_siblings for the begin body when a rescue clause exists suppressed ALL
/// render detection from outer context. This broke cases where flash is in an
/// if/else branch INSIDE the begin body and render exists in outer siblings after
/// the begin/rescue block. The correct fix must only suppress implicit render for
/// flash that is a DIRECT child of the begin body (matching RuboCop's rescue
/// ancestor walk), not for flash nested inside if/else branches within the begin.
///
/// **FP from case/when:** The case/when handler may have also contributed FPs.
/// In RuboCop, `case` is NOT matched by `each_ancestor(:if, :rescue)` — flash in
/// a when body takes the "no ancestor" path in `followed_by_render?`, checking the
/// PARENT scope's right_siblings for render. A correct handler must check the case
/// node's outer siblings for both render AND redirect_to.
///
/// **Remaining FP=4 root cause (not yet fixed):** Flash inside begin/rescue body
/// followed by `respond_to` block containing BOTH render and redirect_to. RuboCop's
/// `use_redirect_to?` is recursive (uses `any_descendant?`), finding redirect_to
/// inside respond_to blocks. Our `is_redirect_sibling` only checks direct siblings.
/// Fix: make redirect detection recursive, or specifically check inside respond_to
/// block bodies for redirect_to.
///
/// **Remaining FN=32 breakdown:**
/// - case/when branches (~8-10 FN): needs careful outer_siblings wiring
/// - lambda/proc blocks as hash args (~7 FN): call.block() doesn't capture these
/// - respond_to with render in nested if/else (~2 FN): block body scoping issue
/// - unless inside begin/rescue (~1 FN): complex nesting
/// - nested assignment `query = flash[:query] = ...` (~1 FN): not a top-level stmt
///
/// ## Investigation (2026-03-26): FN cluster in case/when and nested lambda bodies
///
/// - Root cause 16 (FN~20): `CaseNode` branches were never checked. RuboCop does
///   not treat `case` as an `if/rescue` ancestor, so `when` bodies need their own
///   sibling logic: check later statements in the same branch first, then fall back
///   to the case node's outer siblings for explicit render or def-level implicit render.
/// - Root cause 17 (FN~8): only statement-level `call.block()` bodies were visited.
///   This missed `lambda { ... }` / `-> { ... }` bodies nested inside assignments or
///   hash arguments like `on_valid: lambda { ... }` and `on_invalid: ->() { ... }`.
///   Added descendant block/lambda traversal that preserves the enclosing sibling
///   context and reuses the existing block/branch analysis for the nested body.
/// - Root cause 18 (FP/FN): explicit `begin ... rescue ... end` blocks inside methods need
///   rescue-context handling. Flash in the begin body should see render in rescue/ensure
///   clauses, but not render after the begin/end block. Propagating method-level outer
///   siblings caused FPs like `country_bands_controller`, while suppressing too much caused
///   FNs like `advice_controller`.
use crate::cop::shared::method_dispatch_predicates;
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
        // Reset per class: RuboCop's inherit_action_controller_base? finds the
        // CLOSEST class ancestor and searches it. Each class qualifies independently.
        self.in_action_controller =
            is_action_controller_class(node) || class_body_references_action_controller(node);

        if self.in_action_controller {
            // Check this class's own instance methods and class-level blocks for flash.
            // Then use the full visitor for nested structures (modules, nested classes).
            if let Some(body) = node.body() {
                if let Some(stmts) = body.as_statements_node() {
                    for stmt in stmts.body().iter() {
                        if let Some(def_node) = stmt.as_def_node() {
                            if def_node.receiver().is_none() {
                                self.check_def_body(&def_node);
                            }
                        } else if let Some(call_node) = stmt.as_call_node() {
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
                        }
                    }
                }
                // Full visitor recurse: finds nested classes (including inside modules).
                // visit_class_node resets in_action_controller per-class, so nested
                // non-controller classes won't be falsely treated as controllers.
                self.visit(&body);
            }
        } else {
            // Not a controller — still recurse to find nested classes
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
        if let Some(stmts) = body.as_statements_node() {
            let body_nodes: Vec<ruby_prism::Node<'_>> = stmts.body().iter().collect();
            self.check_statements(&body_nodes);
        } else if let Some(begin_node) = body.as_begin_node() {
            // def ... rescue ... end (no explicit begin): Prism wraps body in BeginNode.
            // In RuboCop's Parser AST, flash in the body has a :rescue ancestor, so
            // each_ancestor(:if, :rescue) finds :rescue and only the rescue node's
            // right_siblings are checked for render. Those right_siblings are the
            // ensure body (if present) or empty. We replicate this by treating the
            // body as a rescue branch with ensure body as outer context.
            let ensure_stmts: Vec<ruby_prism::Node<'_>> = begin_node
                .ensure_clause()
                .and_then(|ec| ec.statements())
                .map(|stmts| stmts.body().iter().collect())
                .unwrap_or_default();

            if let Some(stmts) = begin_node.statements() {
                let body_nodes: Vec<_> = stmts.body().iter().collect();
                self.check_branch_stmts_with_outer(&body_nodes, &ensure_stmts, true);
            }
            if let Some(rescue) = begin_node.rescue_clause() {
                self.check_rescue_with_outer(&rescue, &ensure_stmts);
            }
        }
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

            // Nested local assignment: `query = flash[:query] = ...` should use the
            // local-variable write as the parent context, so later non-redirect
            // statements do not suppress the implicit-render offense.
            if let Some(write) = stmt.as_local_variable_write_node() {
                if let Some(flash_loc) = get_flash_assignment(&write.value()) {
                    let has_redirect = remaining.iter().any(|s| is_redirect_sibling(s));
                    if !has_redirect {
                        self.emit_diagnostic(flash_loc);
                    }
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

            if let Some(case_node) = stmt.as_case_node() {
                self.check_case_node_with_outer(&case_node, remaining, false);
            }

            let handled_context = stmt.as_if_node().is_some()
                || stmt.as_unless_node().is_some()
                || stmt.as_begin_node().is_some()
                || stmt.as_case_node().is_some();

            if !handled_context {
                self.check_embedded_contexts_with_outer(stmt, remaining, false, false);
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
                    // Single non-if/unless stmt: in Parser AST, this is a direct
                    // child of the if node. parent_render_flag captures render from
                    // a parent single-child if's else clause (but NOT this if's own
                    // else, since RuboCop checks the if_node's right_siblings, which
                    // are in the parent scope, not the else clause).
                    self.check_branch_stmts_impl(
                        &body_nodes,
                        outer_siblings,
                        true,
                        parent_render_flag,
                        true,
                    );
                }
            } else {
                // Multi-statement body: in Parser AST wrapped in begin.
                // parent_render_flag propagates render context from parent
                // single-child if chains (e.g., nested if whose parent's
                // else clause has render).
                self.check_branch_stmts_impl(
                    &body_nodes,
                    outer_siblings,
                    true,
                    parent_render_flag,
                    true,
                );
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
                    self.check_branch_stmts_impl(
                        &body_nodes,
                        outer_siblings,
                        true,
                        parent_render_flag,
                        true,
                    );
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
            let rescue_context_nodes = begin_rescue_context_nodes(begin_node);
            if !rescue_context_nodes.is_empty() {
                self.check_begin_body_with_rescue(&body_nodes, &rescue_context_nodes);
            } else {
                self.check_branch_stmts_with_outer(&body_nodes, outer_siblings, true);
            }
        }
        // For rescue clauses: RuboCop's each_ancestor(:rescue) finds the rescue node,
        // and rescue.right_siblings within the begin is empty. So pass empty outer.
        if let Some(rescue) = begin_node.rescue_clause() {
            self.check_rescue_with_outer(&rescue, &[]);
        }
    }

    fn check_begin_body_with_rescue(
        &mut self,
        begin_stmts: &[ruby_prism::Node<'_>],
        rescue_context_nodes: &[ruby_prism::Node<'_>],
    ) {
        // In Parser AST, when the begin body has a single statement, that statement
        // is a direct child of the rescue node (no begin wrapper). Its right_siblings
        // include the resbody nodes. When the body has multiple statements, they're
        // wrapped in a begin node, and each statement's right_siblings are the other
        // statements within the begin (not the resbody nodes).
        let single_stmt_body = begin_stmts.len() == 1;

        for (i, stmt) in begin_stmts.iter().enumerate() {
            let inner_remaining = &begin_stmts[i + 1..];

            // For if/unless: in single-stmt bodies, the node is a direct child of
            // rescue, so its right_siblings include resbody (use rescue_context_nodes).
            // In multi-stmt bodies, it's inside a begin wrapper, so right_siblings
            // are the remaining begin body stmts (use inner_remaining).
            let if_unless_outer = if single_stmt_body {
                rescue_context_nodes
            } else {
                inner_remaining
            };

            if let Some(nested_if) = stmt.as_if_node() {
                self.check_if_node_with_outer(&nested_if, if_unless_outer);
            }
            if let Some(nested_unless) = stmt.as_unless_node() {
                self.check_unless_node_with_outer(&nested_unless, if_unless_outer);
            }
            // begin/case/embedded: not :if or :rescue in Parser AST, so rescue
            // ancestor is found — keep using rescue_context_nodes.
            if let Some(nested_begin) = stmt.as_begin_node() {
                self.check_begin_node_with_outer(&nested_begin, rescue_context_nodes);
            }
            if let Some(nested_case) = stmt.as_case_node() {
                self.check_case_node_with_outer(&nested_case, rescue_context_nodes, true);
            }

            let handled_context = stmt.as_if_node().is_some()
                || stmt.as_unless_node().is_some()
                || stmt.as_begin_node().is_some()
                || stmt.as_case_node().is_some();

            if !handled_context {
                self.check_embedded_contexts_with_outer(stmt, rescue_context_nodes, true, false);
            }
        }
    }

    fn check_case_node_with_outer(
        &mut self,
        case_node: &ruby_prism::CaseNode<'_>,
        outer_siblings: &[ruby_prism::Node<'_>],
        in_if_rescue_context: bool,
    ) {
        for condition in case_node.conditions().iter() {
            let Some(when_node) = condition.as_when_node() else {
                continue;
            };
            let Some(stmts) = when_node.statements() else {
                continue;
            };
            let body_nodes: Vec<_> = stmts.body().iter().collect();
            if in_if_rescue_context {
                self.check_branch_stmts_with_outer(&body_nodes, outer_siblings, true);
            } else {
                self.check_case_branch_stmts(&body_nodes);
            }
        }

        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                let body_nodes: Vec<_> = stmts.body().iter().collect();
                if in_if_rescue_context {
                    self.check_branch_stmts_with_outer(&body_nodes, outer_siblings, true);
                } else {
                    self.check_case_else_branch_stmts(&body_nodes, outer_siblings);
                }
            }
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

    fn check_case_branch_stmts(&mut self, branch_stmts: &[ruby_prism::Node<'_>]) {
        for (i, stmt) in branch_stmts.iter().enumerate() {
            let inner_remaining = &branch_stmts[i + 1..];

            if let Some(flash_loc) = get_flash_assignment(stmt) {
                let inner_has_redirect = inner_remaining.iter().any(|s| is_redirect_sibling(s));
                let inner_has_render = inner_remaining.iter().any(|s| contains_render(s));

                let is_offense = if inner_has_render {
                    true
                } else if inner_remaining.is_empty() {
                    !inner_has_redirect
                } else {
                    false
                };

                if is_offense {
                    self.emit_diagnostic(flash_loc);
                }
            }

            if let Some(nested_if) = stmt.as_if_node() {
                self.check_if_node_with_outer(&nested_if, inner_remaining);
            }
            if let Some(nested_unless) = stmt.as_unless_node() {
                self.check_unless_node_with_outer(&nested_unless, inner_remaining);
            }
            if let Some(nested_begin) = stmt.as_begin_node() {
                self.check_begin_node_with_outer(&nested_begin, inner_remaining);
            }
            if let Some(nested_case) = stmt.as_case_node() {
                self.check_case_node_with_outer(&nested_case, inner_remaining, false);
            }

            let handled_context = stmt.as_if_node().is_some()
                || stmt.as_unless_node().is_some()
                || stmt.as_begin_node().is_some()
                || stmt.as_case_node().is_some();

            if !handled_context {
                self.check_embedded_contexts_with_outer(stmt, inner_remaining, false, false);
            }
        }
    }

    fn check_case_else_branch_stmts(
        &mut self,
        branch_stmts: &[ruby_prism::Node<'_>],
        outer_siblings: &[ruby_prism::Node<'_>],
    ) {
        let outer_has_redirect = outer_siblings.iter().any(|s| is_redirect_sibling(s));

        for (i, stmt) in branch_stmts.iter().enumerate() {
            let inner_remaining = &branch_stmts[i + 1..];

            if let Some(flash_loc) = get_flash_assignment(stmt) {
                let inner_has_redirect = inner_remaining.iter().any(|s| is_redirect_sibling(s));
                let inner_has_render = inner_remaining.iter().any(|s| contains_render(s));

                let is_offense = if inner_has_render {
                    true
                } else if inner_remaining.is_empty() {
                    !inner_has_redirect && !outer_has_redirect
                } else {
                    false
                };

                if is_offense {
                    self.emit_diagnostic(flash_loc);
                }
            }

            if let Some(nested_if) = stmt.as_if_node() {
                self.check_if_node_with_outer(&nested_if, inner_remaining);
            }
            if let Some(nested_unless) = stmt.as_unless_node() {
                self.check_unless_node_with_outer(&nested_unless, inner_remaining);
            }
            if let Some(nested_begin) = stmt.as_begin_node() {
                self.check_begin_node_with_outer(&nested_begin, inner_remaining);
            }
            if let Some(nested_case) = stmt.as_case_node() {
                self.check_case_node_with_outer(&nested_case, inner_remaining, false);
            }

            let handled_context = stmt.as_if_node().is_some()
                || stmt.as_unless_node().is_some()
                || stmt.as_begin_node().is_some()
                || stmt.as_case_node().is_some();

            if !handled_context {
                self.check_embedded_contexts_with_outer(stmt, inner_remaining, false, false);
            }
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
        self.check_branch_stmts_impl(
            branch_stmts,
            outer_siblings,
            is_if_rescue_branch,
            false,
            true,
        );
    }

    fn check_branch_stmts_impl(
        &mut self,
        branch_stmts: &[ruby_prism::Node<'_>],
        outer_siblings: &[ruby_prism::Node<'_>],
        is_if_rescue_branch: bool,
        extra_outer_render: bool,
        outer_redirect_visible: bool,
    ) {
        let outer_has_render =
            extra_outer_render || outer_siblings.iter().any(|s| contains_render(s));

        for (i, stmt) in branch_stmts.iter().enumerate() {
            let inner_remaining = &branch_stmts[i + 1..];

            if let Some(flash_loc) = get_flash_assignment(stmt) {
                let inner_has_render = inner_remaining.iter().any(|s| contains_render(s));
                let inner_has_redirect = inner_remaining.iter().any(|s| is_redirect_sibling(s));

                let is_offense = if is_if_rescue_branch {
                    // For if/rescue: only check outer siblings for render.
                    // No implicit render from branches.
                    !inner_has_redirect && outer_has_render
                } else {
                    // For block bodies: check inner siblings for render first (like def level).
                    if inner_has_render {
                        true
                    } else if inner_remaining.is_empty() {
                        // Flash is alone/last in block — implicit render.
                        // In Parser AST, single-statement block bodies place
                        // the statement directly under the block node, so
                        // parent.right_siblings includes outer scope. Multi-
                        // statement bodies are wrapped in begin, whose
                        // right_siblings are empty (outer redirect invisible).
                        if i > 0 {
                            // Multi-statement block: begin wrapper hides outer
                            // redirect. Always implicit render.
                            true
                        } else if outer_redirect_visible {
                            // Single-statement block: parent is block node,
                            // can see outer scope for redirect when the block
                            // is itself the statement (e.g. `each { flash... }`).
                            let outer_has_redirect =
                                outer_siblings.iter().any(|s| is_redirect_sibling(s));
                            !outer_has_redirect
                        } else {
                            // Embedded lambdas/blocks (e.g. local assignments)
                            // hide later outer redirects from RuboCop's context.parent.
                            true
                        }
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
            if let Some(nested_case) = stmt.as_case_node() {
                let case_outer = if is_if_rescue_branch {
                    outer_siblings
                } else {
                    inner_remaining
                };
                self.check_case_node_with_outer(&nested_case, case_outer, is_if_rescue_branch);
            }

            let handled_context = stmt.as_if_node().is_some()
                || stmt.as_unless_node().is_some()
                || stmt.as_begin_node().is_some()
                || stmt.as_case_node().is_some();

            if !handled_context {
                // In RuboCop, blocks and lambdas are transparent to each_ancestor(:if, :rescue).
                // When inside an if/rescue context, nested block-like bodies inherit the
                // if/rescue's outer siblings rather than their local sibling list.
                let embedded_outer = if is_if_rescue_branch {
                    outer_siblings
                } else {
                    inner_remaining
                };
                self.check_embedded_contexts_with_outer(
                    stmt,
                    embedded_outer,
                    is_if_rescue_branch,
                    extra_outer_render,
                );
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
        extra_outer_render: bool,
        outer_redirect_visible: bool,
    ) {
        if let Some(block_node) = block.as_block_node() {
            if let Some(body) = block_node.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let body_nodes: Vec<_> = stmts.body().iter().collect();
                    self.check_branch_stmts_impl(
                        &body_nodes,
                        outer_siblings,
                        in_if_rescue_context,
                        extra_outer_render,
                        outer_redirect_visible,
                    );
                }
            }
        }
    }

    fn check_lambda_body_with_outer(
        &mut self,
        lambda: &ruby_prism::LambdaNode<'_>,
        outer_siblings: &[ruby_prism::Node<'_>],
        in_if_rescue_context: bool,
        extra_outer_render: bool,
    ) {
        if let Some(body) = lambda.body() {
            if let Some(stmts) = body.as_statements_node() {
                let body_nodes: Vec<_> = stmts.body().iter().collect();
                self.check_branch_stmts_impl(
                    &body_nodes,
                    outer_siblings,
                    in_if_rescue_context,
                    extra_outer_render,
                    false,
                );
            }
        }
    }

    fn check_embedded_contexts_with_outer(
        &mut self,
        node: &ruby_prism::Node<'_>,
        outer_siblings: &[ruby_prism::Node<'_>],
        in_if_rescue_context: bool,
        extra_outer_render: bool,
    ) {
        let mut visitor = EmbeddedContextVisitor {
            flash_visitor: self,
            outer_siblings,
            in_if_rescue_context,
            extra_outer_render,
            statement_level_call: node.as_call_node().is_some(),
            call_depth: 0,
        };
        visitor.visit(node);
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

struct EmbeddedContextVisitor<'v, 'a, 'outer, 'pr> {
    flash_visitor: &'v mut FlashVisitor<'a>,
    outer_siblings: &'outer [ruby_prism::Node<'pr>],
    in_if_rescue_context: bool,
    extra_outer_render: bool,
    statement_level_call: bool,
    call_depth: usize,
}

impl<'a, 'outer, 'pr> Visit<'pr> for EmbeddedContextVisitor<'_, 'a, 'outer, 'pr> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(block) = node.block() {
            let outer_redirect_visible = self.statement_level_call && self.call_depth == 0;
            if block.as_block_node().is_some() {
                self.flash_visitor.check_block_body_with_outer(
                    &block,
                    self.outer_siblings,
                    self.in_if_rescue_context,
                    self.extra_outer_render,
                    outer_redirect_visible,
                );
            }
        }

        self.call_depth += 1;
        if let Some(receiver) = node.receiver() {
            self.visit(&receiver);
        }
        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
        }
        self.call_depth -= 1;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.flash_visitor.check_lambda_body_with_outer(
            node,
            self.outer_siblings,
            self.in_if_rescue_context,
            self.extra_outer_render,
        );
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        self.flash_visitor.check_case_node_with_outer(
            node,
            self.outer_siblings,
            self.in_if_rescue_context,
        );
    }

    fn visit_if_node(&mut self, _node: &ruby_prism::IfNode<'pr>) {}

    fn visit_unless_node(&mut self, _node: &ruby_prism::UnlessNode<'pr>) {}

    fn visit_begin_node(&mut self, _node: &ruby_prism::BeginNode<'pr>) {}
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

fn begin_rescue_context_nodes<'pr>(
    begin_node: &ruby_prism::BeginNode<'pr>,
) -> Vec<ruby_prism::Node<'pr>> {
    let mut nodes = Vec::new();
    if let Some(rescue) = begin_node.rescue_clause() {
        nodes.push(rescue.as_node());
    }
    if let Some(ensure_clause) = begin_node.ensure_clause() {
        nodes.push(ensure_clause.as_node());
    }
    nodes
}

/// Search a class body for any reference to ApplicationController or ActionController::Base.
/// Matches RuboCop's `def_node_search :action_controller?` which searches entire class subtrees,
/// not just the superclass. This handles cases like `Cms::Portlet < ActiveRecord::Base` that
/// reference `ActionController::Base.view_paths` in the body.
fn class_body_references_action_controller(class: &ruby_prism::ClassNode<'_>) -> bool {
    if let Some(body) = class.body() {
        let mut finder = ActionControllerRefFinder { found: false };
        finder.visit(&body);
        return finder.found;
    }
    false
}

struct ActionControllerRefFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for ActionControllerRefFinder {
    fn visit_constant_read_node(&mut self, node: &ruby_prism::ConstantReadNode<'pr>) {
        if !self.found && node.name().as_slice() == b"ApplicationController" {
            self.found = true;
        }
    }

    fn visit_constant_path_node(&mut self, node: &ruby_prism::ConstantPathNode<'pr>) {
        if self.found {
            return;
        }
        if let Some(name) = node.name() {
            // ::ApplicationController
            if name.as_slice() == b"ApplicationController" && node.parent().is_none() {
                self.found = true;
                return;
            }
            // ActionController::Base or ::ActionController::Base
            if name.as_slice() == b"Base" {
                if let Some(parent) = node.parent() {
                    if let Some(c) = parent.as_constant_read_node() {
                        if c.name().as_slice() == b"ActionController" {
                            self.found = true;
                            return;
                        }
                    }
                    if let Some(parent_cp) = parent.as_constant_path_node() {
                        if parent_cp.parent().is_none() {
                            if let Some(parent_name) = parent_cp.name() {
                                if parent_name.as_slice() == b"ActionController" {
                                    self.found = true;
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
        ruby_prism::visit_constant_path_node(self, node);
    }
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
        if method_dispatch_predicates::is_command(&call, b"redirect_to") {
            return true;
        }
    }
    // `return redirect_to ...`
    if let Some(ret) = node.as_return_node() {
        if let Some(args) = ret.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() == 1 {
                if let Some(call) = arg_list[0].as_call_node() {
                    if method_dispatch_predicates::is_command(&call, b"redirect_to") {
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
        if method_dispatch_predicates::is_command(node, self.method) {
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
