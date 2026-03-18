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
    /// For each statement:
    /// - If it is a flash assignment: check if any subsequent sibling contains render,
    ///   OR if there are no subsequent siblings and no redirect among siblings → implicit render.
    /// - If it is an if/rescue block: recurse into its branches, treating the parent
    ///   statements as the outer context for render detection.
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
                    let (line, column) = self.source.offset_to_line_col(flash_loc);
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Use `flash.now` before `render`.".to_string(),
                    ));
                }
            }

            // Flash inside an if/else branch: check if render appears in the outer context
            if let Some(if_node) = stmt.as_if_node() {
                self.check_if_node_with_outer(&if_node, remaining);
            }

            // Flash inside a begin/rescue block: similar outer-context check
            if let Some(begin_node) = stmt.as_begin_node() {
                self.check_begin_node_with_outer(&begin_node, remaining);
            }

            // Recurse into respond_to/format blocks (nested block bodies).
            // Pass outer siblings so implicit-render detection can see outer redirect/render.
            if let Some(call_node) = stmt.as_call_node() {
                if let Some(block) = call_node.block() {
                    self.check_block_body_with_outer(&block, remaining);
                }
            }
        }
    }

    /// Check an if-node's branches. Flash assignments inside branches are offenses
    /// if the outer siblings (or the branch itself if no outer context) contain render.
    fn check_if_node_with_outer(
        &mut self,
        if_node: &ruby_prism::IfNode<'_>,
        outer_siblings: &[ruby_prism::Node<'_>],
    ) {
        // Check flash in the if-branch body with outer siblings as render context
        if let Some(stmts) = if_node.statements() {
            let body_nodes: Vec<_> = stmts.body().iter().collect();
            self.check_branch_stmts_with_outer(&body_nodes, outer_siblings);
        }
        // Check subsequent elsif/else clauses
        if let Some(subsequent) = if_node.subsequent() {
            if let Some(elsif) = subsequent.as_if_node() {
                self.check_if_node_with_outer(&elsif, outer_siblings);
            }
            if let Some(else_clause) = subsequent.as_else_node() {
                if let Some(stmts) = else_clause.statements() {
                    let body_nodes: Vec<_> = stmts.body().iter().collect();
                    self.check_branch_stmts_with_outer(&body_nodes, outer_siblings);
                }
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
            self.check_branch_stmts_with_outer(&body_nodes, outer_siblings);
        }
        if let Some(rescue) = begin_node.rescue_clause() {
            self.check_rescue_with_outer(&rescue, outer_siblings);
        }
    }

    fn check_rescue_with_outer(
        &mut self,
        rescue: &ruby_prism::RescueNode<'_>,
        outer_siblings: &[ruby_prism::Node<'_>],
    ) {
        if let Some(stmts) = rescue.statements() {
            let body_nodes: Vec<_> = stmts.body().iter().collect();
            self.check_branch_stmts_with_outer(&body_nodes, outer_siblings);
        }
        if let Some(subsequent) = rescue.subsequent() {
            self.check_rescue_with_outer(&subsequent, outer_siblings);
        }
    }

    /// Check statements inside a branch (if/rescue body). Flash assignments are offenses
    /// if:
    /// - There is a redirect in the branch itself → no offense
    /// - There is a render in the outer siblings (after the if/rescue) → offense
    /// - Flash has further siblings inside the branch that contain render → offense
    fn check_branch_stmts_with_outer(
        &mut self,
        branch_stmts: &[ruby_prism::Node<'_>],
        outer_siblings: &[ruby_prism::Node<'_>],
    ) {
        let outer_has_render = outer_siblings.iter().any(|s| contains_render(s));
        let outer_has_redirect = outer_siblings.iter().any(|s| is_redirect_sibling(s));

        for (i, stmt) in branch_stmts.iter().enumerate() {
            let inner_remaining = &branch_stmts[i + 1..];

            if let Some(flash_loc) = get_flash_assignment(stmt) {
                let inner_has_render = inner_remaining.iter().any(|s| contains_render(s));
                let inner_has_redirect = inner_remaining.iter().any(|s| is_redirect_sibling(s));

                // If redirect appears in the same branch after flash → no offense
                if inner_has_redirect {
                    continue;
                }

                let is_offense = if inner_has_render {
                    // render in same branch after flash
                    true
                } else if !inner_remaining.is_empty() {
                    // There are siblings in the branch but none is render/redirect → no offense
                    // (something else happens after flash in the branch)
                    false
                } else {
                    // Flash is last in branch — check outer context
                    // If outer siblings contain redirect → no offense
                    if outer_has_redirect {
                        false
                    } else {
                        // Offense if outer siblings contain render, or no outer siblings (implicit render)
                        outer_has_render || outer_siblings.is_empty()
                    }
                };

                if is_offense {
                    let (line, column) = self.source.offset_to_line_col(flash_loc);
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Use `flash.now` before `render`.".to_string(),
                    ));
                }
            }

            // Recurse into nested if/rescue inside the branch
            if let Some(nested_if) = stmt.as_if_node() {
                self.check_if_node_with_outer(&nested_if, outer_siblings);
            }
            if let Some(nested_begin) = stmt.as_begin_node() {
                self.check_begin_node_with_outer(&nested_begin, outer_siblings);
            }
            // Recurse into nested call blocks (e.g. respond_to → format.js do...end)
            if let Some(call_node) = stmt.as_call_node() {
                if let Some(block) = call_node.block() {
                    self.check_block_body_with_outer(&block, outer_siblings);
                }
            }
        }
    }

    /// Check a block body with awareness of the outer sibling context.
    ///
    /// For flash inside a block body:
    /// - If flash has siblings inside the block that contain render → offense (no outer context needed)
    /// - If flash is alone in the block (no inner siblings): treat outer siblings as context.
    ///   If outer siblings contain redirect → no offense; if outer contains render or no outer → offense.
    ///
    /// This correctly handles:
    /// - `respond_to do |f|; flash; render; end` → offense (render inside block)
    /// - `messages.each { flash }; redirect_to` → no offense (outer redirect)
    /// - `messages.each { flash }; render` → offense (outer render)
    fn check_block_body_with_outer(
        &mut self,
        block: &ruby_prism::Node<'_>,
        outer_siblings: &[ruby_prism::Node<'_>],
    ) {
        if let Some(block_node) = block.as_block_node() {
            if let Some(body) = block_node.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let body_nodes: Vec<_> = stmts.body().iter().collect();
                    // Use branch-with-outer logic: flash alone in block uses outer context
                    self.check_branch_stmts_with_outer(&body_nodes, outer_siblings);
                }
            }
        }
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
