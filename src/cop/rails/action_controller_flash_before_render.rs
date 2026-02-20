use crate::cop::util;
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

    fn default_include(&self) -> &'static [&'static str] {
        &["app/controllers/**/*.rb"]
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
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_action_controller = was_in_controller;
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if self.in_action_controller && node.receiver().is_none() {
            self.check_def(node);
        }
    }
}

impl FlashVisitor<'_> {

    fn check_def(&mut self, def_node: &ruby_prism::DefNode<'_>) {
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

    fn check_statements(&mut self, stmts: &[ruby_prism::Node<'_>]) {
        for (i, stmt) in stmts.iter().enumerate() {
            // Check if this statement is a flash assignment
            if let Some(flash_loc) = get_flash_assignment(stmt) {
                // Check if any subsequent sibling is a render (not redirect_to)
                let remaining = &stmts[i + 1..];
                let has_render = remaining.iter().any(|s| contains_render(s));
                let has_redirect = remaining.iter().any(|s| contains_redirect(s));

                if has_render && !has_redirect {
                    let (line, column) = self.source.offset_to_line_col(flash_loc);
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Use `flash.now` before `render`.".to_string(),
                    ));
                }
            }

            // Also check inside rescue/if blocks
            if let Some(rescue_node) = stmt.as_begin_node() {
                if let Some(body) = rescue_node.statements() {
                    let inner: Vec<_> = body.body().iter().collect();
                    self.check_statements(&inner);
                }
                if let Some(rescue) = rescue_node.rescue_clause() {
                    self.check_rescue(&rescue);
                }
            }
            if let Some(if_node) = stmt.as_if_node() {
                self.check_if_node(&if_node);
            }
        }
    }

    fn check_rescue(&mut self, rescue: &ruby_prism::RescueNode<'_>) {
        if let Some(stmts) = rescue.statements() {
            let body_nodes: Vec<_> = stmts.body().iter().collect();
            self.check_statements(&body_nodes);
        }
        if let Some(subsequent) = rescue.subsequent() {
            self.check_rescue(&subsequent);
        }
    }

    fn check_if_node(&mut self, if_node: &ruby_prism::IfNode<'_>) {
        if let Some(stmts) = if_node.statements() {
            let body_nodes: Vec<_> = stmts.body().iter().collect();
            self.check_statements(&body_nodes);
        }
        if let Some(subsequent) = if_node.subsequent() {
            if let Some(elsif) = subsequent.as_if_node() {
                self.check_if_node(&elsif);
            }
            if let Some(else_clause) = subsequent.as_else_node() {
                if let Some(stmts) = else_clause.statements() {
                    let body_nodes: Vec<_> = stmts.body().iter().collect();
                    self.check_statements(&body_nodes);
                }
            }
        }
    }
}

/// Check if a class inherits from ApplicationController or ActionController::Base
fn is_action_controller_class(class: &ruby_prism::ClassNode<'_>) -> bool {
    let superclass = match class.superclass() {
        Some(s) => s,
        None => return false,
    };

    // ApplicationController
    if let Some(c) = superclass.as_constant_read_node() {
        if c.name().as_slice() == b"ApplicationController" {
            return true;
        }
    }

    // ActionController::Base
    if let Some(cp) = superclass.as_constant_path_node() {
        if let Some(name) = cp.name() {
            if name.as_slice() == b"Base" {
                if let Some(parent) = cp.parent() {
                    if util::constant_name(&parent) == Some(b"ActionController") {
                        return true;
                    }
                }
            }
        }
    }

    // Also match anything ending in Controller (like Devise::PasswordsController, etc.)
    // This is a heuristic since Include patterns already restrict to controllers/
    if let Some(c) = superclass.as_constant_read_node() {
        if c.name().as_slice().ends_with(b"Controller") {
            return true;
        }
    }
    if let Some(cp) = superclass.as_constant_path_node() {
        if let Some(name) = cp.name() {
            if name.as_slice().ends_with(b"Controller") {
                return true;
            }
        }
    }

    false
}

/// Check if a node is `flash[:key] = value` and return the flash location offset
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

/// Check if a node contains a `render` call
fn contains_render(node: &ruby_prism::Node<'_>) -> bool {
    let mut finder = CallFinder { method: b"render", found: false };
    finder.visit(node);
    finder.found
}

/// Check if a node contains a `redirect_to` call
fn contains_redirect(node: &ruby_prism::Node<'_>) -> bool {
    let mut finder = CallFinder { method: b"redirect_to", found: false };
    finder.visit(node);
    finder.found
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
            // Continue searching children
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
