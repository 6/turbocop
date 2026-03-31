use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/Next: Use `next` to skip iteration instead of wrapping conditionals.
///
/// Key fixes applied:
/// - Check the LAST statement in block body, not just single-statement bodies
///   (RuboCop's `ends_with_condition?` logic). This was the main source of FN.
/// - Added `while`/`until` loop support (RuboCop's `on_while`/`on_until`).
/// - Added `loop` and other missing enumerator methods (`inject`, `reduce`,
///   `find_index`, `map!`, `select!`, `reject!`).
/// - Added `each_*` prefix matching for dynamic enumerator methods.
/// - Removed `any?`/`none?` (not in RuboCop's ENUMERATOR_METHODS, caused FP).
///
/// Remaining FN sources: `AllowConsecutiveConditionals` handling, `if_else_children?`
/// check (skip when nested if-else in body), `exit_body_type?` (skip when body is
/// break/return), and some config/context differences.
pub struct Next;

/// Iterator methods whose blocks should use `next` instead of wrapping conditionals.
/// Matches RuboCop's `ENUMERATOR_METHODS` plus any method starting with `each_`.
const ITERATION_METHODS: &[&[u8]] = &[
    b"collect",
    b"collect_concat",
    b"detect",
    b"downto",
    b"each",
    b"filter",
    b"find",
    b"find_all",
    b"find_index",
    b"flat_map",
    b"inject",
    b"loop",
    b"map",
    b"map!",
    b"max_by",
    b"min_by",
    b"reduce",
    b"reject",
    b"reject!",
    b"reverse_each",
    b"select",
    b"select!",
    b"sort_by",
    b"times",
    b"upto",
];

/// Check if a method name is an enumerator method (static list or `each_*` prefix)
fn is_enumerator_method(name: &[u8]) -> bool {
    ITERATION_METHODS.contains(&name) || name.starts_with(b"each_")
}

impl Cop for Next {
    fn name(&self) -> &'static str {
        "Style/Next"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "skip_modifier_ifs");
        let min_body_length = config.get_usize("MinBodyLength", 3);
        let _allow_consecutive = config.get_bool("AllowConsecutiveConditionals", false);
        let mut visitor = NextVisitor {
            cop: self,
            source,
            style,
            min_body_length,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct NextVisitor<'a> {
    cop: &'a Next,
    source: &'a SourceFile,
    style: &'a str,
    min_body_length: usize,
    diagnostics: Vec<Diagnostic>,
}

impl NextVisitor<'_> {
    fn check_block_body(&mut self, body: &ruby_prism::Node<'_>) {
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_stmts: Vec<_> = stmts.body().iter().collect();
        if body_stmts.is_empty() {
            return;
        }

        // RuboCop checks if the LAST statement is an if/unless (ends_with_condition?)
        let stmt = &body_stmts[body_stmts.len() - 1];

        // Check for if/unless that wraps the entire block body
        if let Some(if_node) = stmt.as_if_node() {
            // Skip if it has an else branch
            if if_node.subsequent().is_some() {
                return;
            }

            // Skip modifier ifs if style is skip_modifier_ifs
            if self.style == "skip_modifier_ifs" {
                if let Some(kw_loc) = if_node.if_keyword_loc() {
                    // Modifier if: the keyword comes after the body
                    let kw = kw_loc.as_slice();
                    if kw == b"if" || kw == b"unless" {
                        if let Some(body_stmts) = if_node.statements() {
                            let body_loc = body_stmts.location();
                            if body_loc.start_offset() < kw_loc.start_offset() {
                                return;
                            }
                        }
                    }
                }
            }

            // Check body length
            if let Some(if_body) = if_node.statements() {
                let if_body_stmts: Vec<_> = if_body.body().iter().collect();
                if if_body_stmts.len() < self.min_body_length {
                    return;
                }
            } else {
                return;
            }

            if let Some(kw_loc) = if_node.if_keyword_loc() {
                let (line, column) = self.source.offset_to_line_col(kw_loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Use `next` to skip iteration.".to_string(),
                ));
            }
        } else if let Some(unless_node) = stmt.as_unless_node() {
            // Skip if it has an else branch
            if unless_node.else_clause().is_some() {
                return;
            }

            // Skip modifier unless if style is skip_modifier_ifs
            if self.style == "skip_modifier_ifs" {
                let kw_loc = unless_node.keyword_loc();
                if let Some(body_stmts) = unless_node.statements() {
                    let body_loc = body_stmts.location();
                    if body_loc.start_offset() < kw_loc.start_offset() {
                        return;
                    }
                }
            }

            // Check body length
            if let Some(unless_body) = unless_node.statements() {
                let unless_body_stmts: Vec<_> = unless_body.body().iter().collect();
                if unless_body_stmts.len() < self.min_body_length {
                    return;
                }
            } else {
                return;
            }

            let kw_loc = unless_node.keyword_loc();
            let (line, column) = self.source.offset_to_line_col(kw_loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Use `next` to skip iteration.".to_string(),
            ));
        }
    }
}

impl<'pr> Visit<'pr> for NextVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_bytes = node.name().as_slice();

        if is_enumerator_method(method_bytes) {
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
                    if let Some(body) = block_node.body() {
                        self.check_block_body(&body);
                    }
                }
            }
        }

        // Visit children
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
        }
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'pr>) {
        if let Some(stmts) = node.statements() {
            self.check_block_body(&stmts.as_node());
        }
        // Visit children
        self.visit(&node.collection());
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        if let Some(stmts) = node.statements() {
            self.check_block_body(&stmts.as_node());
        }
        // Visit children
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        if let Some(stmts) = node.statements() {
            self.check_block_body(&stmts.as_node());
        }
        // Visit children
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Next, "cops/style/next");
}
