use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Next;

/// Iterator methods whose blocks should use `next` instead of wrapping conditionals
const ITERATION_METHODS: &[&[u8]] = &[
    b"each",
    b"each_with_index",
    b"each_with_object",
    b"each_pair",
    b"each_key",
    b"each_value",
    b"each_slice",
    b"each_cons",
    b"collect",
    b"map",
    b"select",
    b"filter",
    b"reject",
    b"detect",
    b"find",
    b"find_all",
    b"flat_map",
    b"collect_concat",
    b"any?",
    b"all?",
    b"none?",
    b"sort_by",
    b"min_by",
    b"max_by",
    b"times",
    b"upto",
    b"downto",
    b"reverse_each",
];

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
        if body_stmts.len() != 1 {
            return;
        }

        let stmt = &body_stmts[0];

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

        if ITERATION_METHODS.contains(&method_bytes) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Next, "cops/style/next");
}
