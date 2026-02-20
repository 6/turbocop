use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct ConstantRegexp;

const MSG: &str =
    "Extract this regexp into a constant, memoize it, or append an `/o` option to its options.";

impl Cop for ConstantRegexp {
    fn name(&self) -> &'static str {
        "Performance/ConstantRegexp"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let mut visitor = ConstantRegexpVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_constant_assignment: false,
            in_or_assignment: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ConstantRegexpVisitor<'a, 'src> {
    cop: &'a ConstantRegexp,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    in_constant_assignment: bool,
    in_or_assignment: bool,
}

impl<'pr> Visit<'pr> for ConstantRegexpVisitor<'_, '_> {
    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        let prev = self.in_constant_assignment;
        self.in_constant_assignment = true;
        ruby_prism::visit_constant_write_node(self, node);
        self.in_constant_assignment = prev;
    }

    fn visit_constant_path_write_node(
        &mut self,
        node: &ruby_prism::ConstantPathWriteNode<'pr>,
    ) {
        let prev = self.in_constant_assignment;
        self.in_constant_assignment = true;
        ruby_prism::visit_constant_path_write_node(self, node);
        self.in_constant_assignment = prev;
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        let prev = self.in_or_assignment;
        self.in_or_assignment = true;
        ruby_prism::visit_local_variable_or_write_node(self, node);
        self.in_or_assignment = prev;
    }

    fn visit_instance_variable_or_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOrWriteNode<'pr>,
    ) {
        let prev = self.in_or_assignment;
        self.in_or_assignment = true;
        ruby_prism::visit_instance_variable_or_write_node(self, node);
        self.in_or_assignment = prev;
    }

    fn visit_class_variable_or_write_node(
        &mut self,
        node: &ruby_prism::ClassVariableOrWriteNode<'pr>,
    ) {
        let prev = self.in_or_assignment;
        self.in_or_assignment = true;
        ruby_prism::visit_class_variable_or_write_node(self, node);
        self.in_or_assignment = prev;
    }

    fn visit_interpolated_regular_expression_node(
        &mut self,
        node: &ruby_prism::InterpolatedRegularExpressionNode<'pr>,
    ) {
        self.check_interpolated_regexp(node);
        ruby_prism::visit_interpolated_regular_expression_node(self, node);
    }
}

impl ConstantRegexpVisitor<'_, '_> {
    fn check_interpolated_regexp(
        &mut self,
        node: &ruby_prism::InterpolatedRegularExpressionNode<'_>,
    ) {
        // Skip if inside constant assignment or ||= assignment
        if self.in_constant_assignment || self.in_or_assignment {
            return;
        }

        // Check for /o flag — if present, skip
        let closing = node.closing_loc().as_slice();
        if closing.contains(&b'o') {
            return;
        }

        // Check that the regexp has interpolation and all interpolated parts are constants
        // or Regexp.escape(CONST)
        let parts = node.parts();
        let mut has_interpolation = false;

        for part in parts.iter() {
            if let Some(embedded) = part.as_embedded_statements_node() {
                has_interpolation = true;
                // The embedded expression must be a constant or Regexp.escape(CONST)
                let stmts = match embedded.statements() {
                    Some(s) => s,
                    None => return, // empty interpolation, skip
                };
                let body = stmts.body();
                if body.len() != 1 {
                    return;
                }
                let inner = match body.iter().next() {
                    Some(n) => n,
                    None => return,
                };
                if !is_const_or_regexp_escape(&inner) {
                    return;
                }
            }
            // String parts are fine (literal text in the regex)
        }

        if !has_interpolation {
            return;
        }

        // Check that the regexp has only ONE interpolation (single_interpolation? check)
        // Actually, single_interpolation means only one part total that is ONLY interpolation
        // (the entire regexp is just `#{CONST}` with no other content).
        // RuboCop skips these because they're likely to be `o` flag candidates.
        // Wait, re-reading the source: node.single_interpolation? returns true when there is
        // exactly one child that is a begin node - i.e. /#{CONST}/ with nothing else.
        // The cop skips these (returns early). Let me re-check...
        //
        // Actually from the source:
        //   return if within_allowed_assignment?(node) || !include_interpolated_const?(node) || node.single_interpolation?
        // So it DOES skip single_interpolation. Let me check what single_interpolation means.
        // In RuboCop, single_interpolation? means the regexp is purely interpolation with no
        // literal parts at all: /#{CONST}/
        // Wait, that's still flagged in the spec tests! The first test is /\A#{CONST}/ which has
        // literal \A. Let me re-read...
        //
        // Looking at the spec again: the first test IS flagged. So single_interpolation must mean
        // when the whole regex is just one interpolation with nothing else: /#{CONST}/ exactly.
        // But looking at the specs, there's no test for /#{CONST}/ alone. Let me just not implement
        // this check, since it's a rare edge case and there's no test coverage for it.
        // Actually, single_interpolation? in RuboCop's node API means the regexp has only one part
        // and that part is interpolation (no literal portions). This would match /#{X}/ but not
        // /foo#{X}/ or /#{X}bar/. I'll implement this check.
        let mut interp_count = 0;
        let mut string_count = 0;
        for part in parts.iter() {
            if part.as_embedded_statements_node().is_some() {
                interp_count += 1;
            } else if part.as_string_node().is_some() {
                // Check if the string part is non-empty
                let s = part.as_string_node().unwrap();
                if !s.unescaped().is_empty() {
                    string_count += 1;
                }
            }
        }
        if interp_count == 1 && string_count == 0 {
            // Single interpolation with no literal text — skip
            return;
        }

        let loc = node.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics
            .push(self.cop.diagnostic(self.source, line, column, MSG.to_string()));
    }
}

/// Check if a node is a constant (ConstantReadNode or ConstantPathNode)
/// or a Regexp.escape(CONST) call.
fn is_const_or_regexp_escape(node: &ruby_prism::Node<'_>) -> bool {
    // Check for constant read/path
    if node.as_constant_read_node().is_some() || node.as_constant_path_node().is_some() {
        return true;
    }

    // Check for Regexp.escape(CONST)
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"escape" {
            if let Some(recv) = call.receiver() {
                if let Some(cr) = recv.as_constant_read_node() {
                    if cr.name().as_slice() == b"Regexp" {
                        // Check that the argument is a constant
                        if let Some(args) = call.arguments() {
                            let arg_list: Vec<_> = args.arguments().iter().collect();
                            if arg_list.len() == 1 {
                                let arg = &arg_list[0];
                                if arg.as_constant_read_node().is_some()
                                    || arg.as_constant_path_node().is_some()
                                {
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

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConstantRegexp, "cops/performance/constant_regexp");
}
