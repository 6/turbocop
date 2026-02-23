use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct NonNilCheck;

impl Cop for NonNilCheck {
    fn name(&self) -> &'static str {
        "Style/NonNilCheck"
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
        let include_semantic_changes = config.get_bool("IncludeSemanticChanges", false);
        let mut visitor = NonNilCheckVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            include_semantic_changes,
            in_predicate_method: false,
            predicate_last_stmt_offset: None,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct NonNilCheckVisitor<'a, 'src> {
    cop: &'a NonNilCheck,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    include_semantic_changes: bool,
    in_predicate_method: bool,
    /// Start offset of the last statement in the current predicate method body.
    predicate_last_stmt_offset: Option<usize>,
}

impl<'pr> Visit<'pr> for NonNilCheckVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let name = node.name().as_slice();
        let is_predicate = name.ends_with(b"?");

        let prev_in_predicate = self.in_predicate_method;
        let prev_last_offset = self.predicate_last_stmt_offset;

        self.in_predicate_method = is_predicate;
        self.predicate_last_stmt_offset = if is_predicate {
            node.body().and_then(|body| {
                if let Some(stmts) = body.as_statements_node() {
                    let body_stmts: Vec<_> = stmts.body().iter().collect();
                    body_stmts.last().map(|n| n.location().start_offset())
                } else {
                    Some(body.location().start_offset())
                }
            })
        } else {
            None
        };

        ruby_prism::visit_def_node(self, node);

        self.in_predicate_method = prev_in_predicate;
        self.predicate_last_stmt_offset = prev_last_offset;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method = node.name().as_slice();

        // Pattern 1: x != nil
        if method == b"!=" {
            if let Some(args) = node.arguments() {
                let args_vec: Vec<_> = args.arguments().iter().collect();
                if args_vec.len() == 1
                    && args_vec[0].as_nil_node().is_some()
                    && node.receiver().is_some()
                {
                    // RuboCop skips the last expression of predicate methods (def foo?)
                    let is_predicate_return = self.in_predicate_method
                        && self.predicate_last_stmt_offset == Some(node.location().start_offset());
                    if !is_predicate_return {
                        let loc = node.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        if self.include_semantic_changes {
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                "Explicit non-nil checks are usually redundant.".to_string(),
                            ));
                        } else {
                            let receiver_src =
                                std::str::from_utf8(node.receiver().unwrap().location().as_slice())
                                    .unwrap_or("x");
                            let current_src = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                format!("Prefer `!{}.nil?` over `{}`.", receiver_src, current_src),
                            ));
                        }
                    }
                }
            }
        }

        // Pattern 2: !x.nil? (only with IncludeSemanticChanges)
        if self.include_semantic_changes && method == b"!" {
            if let Some(receiver) = node.receiver() {
                if let Some(inner_call) = receiver.as_call_node() {
                    if inner_call.name().as_slice() == b"nil?"
                        && inner_call.arguments().is_none()
                        && inner_call.receiver().is_some()
                    {
                        let is_predicate_return = self.in_predicate_method
                            && self.predicate_last_stmt_offset
                                == Some(node.location().start_offset());
                        if !is_predicate_return {
                            let loc = node.location();
                            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                "Explicit non-nil checks are usually redundant.".to_string(),
                            ));
                        }
                    }
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NonNilCheck, "cops/style/non_nil_check");
}
