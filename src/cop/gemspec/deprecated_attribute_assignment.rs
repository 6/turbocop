use ruby_prism::Visit;

use crate::cop::shared::constant_predicates;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-03)
///
/// Corpus oracle (run 22651309591) reported FP=0, FN=0. 100% conformance.
///
/// ## Extended corpus investigation (2026-03-21)
///
/// Extended corpus reported FP=4, FN=0. All 4 FPs came from
/// `Gem::Specification.new` with positional args (name, version).
/// RuboCop's `gem_specification` NodePattern requires `.new` with no
/// positional args — added `node.arguments().is_none()` check.
pub struct DeprecatedAttributeAssignment;

const ATTR_TEST_FILES: &[u8] = b"test_files";
const ATTR_DATE: &[u8] = b"date";
const ATTR_SPEC_VERSION: &[u8] = b"specification_version";
const ATTR_RUBYGEMS_VERSION: &[u8] = b"rubygems_version";

impl Cop for DeprecatedAttributeAssignment {
    fn name(&self) -> &'static str {
        "Gemspec/DeprecatedAttributeAssignment"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = GemspecVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            found_assignments: Vec::new(),
        };
        visitor.visit(&parse_result.node());

        if let Some(ref mut corr) = corrections {
            let bytes = source.as_bytes();
            for (diag, found) in visitor
                .diagnostics
                .iter()
                .zip(visitor.found_assignments.iter())
            {
                // Find the full line range to delete (including leading whitespace and newline).
                let mut line_start = found.stmt_start;
                while line_start > 0 && bytes[line_start - 1] != b'\n' {
                    line_start -= 1;
                }
                let mut line_end = found.stmt_end;
                while line_end < bytes.len() && bytes[line_end] != b'\n' {
                    line_end += 1;
                }
                if line_end < bytes.len() && bytes[line_end] == b'\n' {
                    line_end += 1;
                }
                corr.push(crate::correction::Correction {
                    start: line_start,
                    end: line_end,
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                let mut d = diag.clone();
                d.corrected = true;
                diagnostics.push(d);
            }
        } else {
            diagnostics.extend(visitor.diagnostics);
        }
    }
}

struct GemspecVisitor<'a> {
    cop: &'a DeprecatedAttributeAssignment,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    found_assignments: Vec<FoundAssignment>,
}

impl GemspecVisitor<'_> {
    fn is_gem_specification_receiver(receiver: &ruby_prism::Node<'_>) -> bool {
        let path = match receiver.as_constant_path_node() {
            Some(path) => path,
            None => return false,
        };
        if path.name().map(|n| n.as_slice()) != Some(b"Specification") {
            return false;
        }
        let parent = match path.parent() {
            Some(parent) => parent,
            None => return false,
        };
        constant_predicates::constant_short_name(&parent) == Some(b"Gem")
    }
}

impl<'pr> Visit<'pr> for GemspecVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"new"
            && node
                .receiver()
                .is_some_and(|recv| Self::is_gem_specification_receiver(&recv))
            && node.arguments().is_none()
            && node.block().is_some()
        {
            if let Some(block) = node.block().and_then(|b| b.as_block_node()) {
                if let Some(block_param) = block_parameter_name(&block) {
                    let mut finder = DeprecatedAssignmentFinder {
                        source: self.source,
                        block_param,
                        found: None,
                    };
                    finder.visit(&block.as_node());
                    if let Some(found) = finder.found {
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            found.line,
                            found.column,
                            format!("Do not set `{}` in gemspec.", found.attribute),
                        ));
                        self.found_assignments.push(found);
                    }
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }
}

struct FoundAssignment {
    line: usize,
    column: usize,
    attribute: &'static str,
    /// Byte offset of the entire statement (for autocorrect line deletion).
    stmt_start: usize,
    stmt_end: usize,
}

struct DeprecatedAssignmentFinder<'a> {
    source: &'a SourceFile,
    block_param: Vec<u8>,
    found: Option<FoundAssignment>,
}

impl DeprecatedAssignmentFinder<'_> {
    fn receiver_matches_block_param(&self, receiver: Option<ruby_prism::Node<'_>>) -> bool {
        let recv = match receiver.and_then(|r| r.as_local_variable_read_node()) {
            Some(recv) => recv,
            None => return false,
        };
        recv.name().as_slice() == self.block_param.as_slice()
    }

    fn record_from_loc(
        &mut self,
        msg_loc: ruby_prism::Location<'_>,
        stmt_loc: ruby_prism::Location<'_>,
        attribute: &'static str,
    ) {
        if self.found.is_some() {
            return;
        }
        let (line, column) = self.source.offset_to_line_col(msg_loc.start_offset());
        self.found = Some(FoundAssignment {
            line,
            column,
            attribute,
            stmt_start: stmt_loc.start_offset(),
            stmt_end: stmt_loc.end_offset(),
        });
    }
}

impl<'pr> Visit<'pr> for DeprecatedAssignmentFinder<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.found.is_none() && self.receiver_matches_block_param(node.receiver()) {
            if let Some(method_name) = node.name().as_slice().strip_suffix(b"=") {
                if let Some(attribute) = deprecated_attribute_name(method_name) {
                    self.record_from_loc(
                        node.message_loc().unwrap_or(node.location()),
                        node.location(),
                        attribute,
                    );
                }
            }
        }
        if self.found.is_none() {
            ruby_prism::visit_call_node(self, node);
        }
    }

    fn visit_call_operator_write_node(&mut self, node: &ruby_prism::CallOperatorWriteNode<'pr>) {
        if self.found.is_none() && self.receiver_matches_block_param(node.receiver()) {
            if let Some(attribute) = deprecated_attribute_name(node.read_name().as_slice()) {
                self.record_from_loc(
                    node.message_loc().unwrap_or(node.location()),
                    node.location(),
                    attribute,
                );
            }
        }
        if self.found.is_none() {
            ruby_prism::visit_call_operator_write_node(self, node);
        }
    }

    fn visit_call_or_write_node(&mut self, node: &ruby_prism::CallOrWriteNode<'pr>) {
        if self.found.is_none() && self.receiver_matches_block_param(node.receiver()) {
            if let Some(attribute) = deprecated_attribute_name(node.read_name().as_slice()) {
                self.record_from_loc(
                    node.message_loc().unwrap_or(node.location()),
                    node.location(),
                    attribute,
                );
            }
        }
        if self.found.is_none() {
            ruby_prism::visit_call_or_write_node(self, node);
        }
    }

    fn visit_call_and_write_node(&mut self, node: &ruby_prism::CallAndWriteNode<'pr>) {
        if self.found.is_none() && self.receiver_matches_block_param(node.receiver()) {
            if let Some(attribute) = deprecated_attribute_name(node.read_name().as_slice()) {
                self.record_from_loc(
                    node.message_loc().unwrap_or(node.location()),
                    node.location(),
                    attribute,
                );
            }
        }
        if self.found.is_none() {
            ruby_prism::visit_call_and_write_node(self, node);
        }
    }
}

fn block_parameter_name(block: &ruby_prism::BlockNode<'_>) -> Option<Vec<u8>> {
    let params = block.parameters()?;
    let block_params = params.as_block_parameters_node()?;
    let parameters = block_params.parameters()?;
    let required = parameters.requireds().iter().next()?;
    let required = required.as_required_parameter_node()?;
    Some(required.name().as_slice().to_vec())
}

fn deprecated_attribute_name(name: &[u8]) -> Option<&'static str> {
    if name == ATTR_TEST_FILES {
        return Some("test_files");
    }
    if name == ATTR_DATE {
        return Some("date");
    }
    if name == ATTR_SPEC_VERSION {
        return Some("specification_version");
    }
    if name == ATTR_RUBYGEMS_VERSION {
        return Some("rubygems_version");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        DeprecatedAttributeAssignment,
        "cops/gemspec/deprecated_attribute_assignment"
    );
    crate::cop_autocorrect_fixture_tests!(
        DeprecatedAttributeAssignment,
        "cops/gemspec/deprecated_attribute_assignment"
    );

    #[test]
    fn autocorrect_removes_deprecated_line() {
        let input = b"Gem::Specification.new do |s|\n  s.name = 'foo'\n  s.test_files = ['a']\n  s.version = '1.0'\nend\n";
        let (diags, corrections) =
            crate::testutil::run_cop_autocorrect(&DeprecatedAttributeAssignment, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(
            corrected,
            b"Gem::Specification.new do |s|\n  s.name = 'foo'\n  s.version = '1.0'\nend\n"
        );
    }
}
