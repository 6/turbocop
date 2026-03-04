use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig, util};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-03)
///
/// Corpus oracle (run 22651309591) reported FP=0, FN=0. 100% conformance.
pub struct DeprecatedAttributeAssignment;

const ATTR_TEST_FILES: &[u8] = b"test_files";
const ATTR_DATE: &[u8] = b"date";
const ATTR_SPEC_VERSION: &[u8] = b"specification_version";
const ATTR_RUBYGEMS_VERSION: &[u8] = b"rubygems_version";

impl Cop for DeprecatedAttributeAssignment {
    fn name(&self) -> &'static str {
        "Gemspec/DeprecatedAttributeAssignment"
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = GemspecVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct GemspecVisitor<'a> {
    cop: &'a DeprecatedAttributeAssignment,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
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
        util::constant_name(&parent) == Some(b"Gem")
    }
}

impl<'pr> Visit<'pr> for GemspecVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"new"
            && node
                .receiver()
                .is_some_and(|recv| Self::is_gem_specification_receiver(&recv))
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

    fn record_from_loc(&mut self, loc: ruby_prism::Location<'_>, attribute: &'static str) {
        if self.found.is_some() {
            return;
        }
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.found = Some(FoundAssignment {
            line,
            column,
            attribute,
        });
    }
}

impl<'pr> Visit<'pr> for DeprecatedAssignmentFinder<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.found.is_none() && self.receiver_matches_block_param(node.receiver()) {
            if let Some(method_name) = node.name().as_slice().strip_suffix(b"=") {
                if let Some(attribute) = deprecated_attribute_name(method_name) {
                    self.record_from_loc(node.message_loc().unwrap_or(node.location()), attribute);
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
                self.record_from_loc(node.message_loc().unwrap_or(node.location()), attribute);
            }
        }
        if self.found.is_none() {
            ruby_prism::visit_call_operator_write_node(self, node);
        }
    }

    fn visit_call_or_write_node(&mut self, node: &ruby_prism::CallOrWriteNode<'pr>) {
        if self.found.is_none() && self.receiver_matches_block_param(node.receiver()) {
            if let Some(attribute) = deprecated_attribute_name(node.read_name().as_slice()) {
                self.record_from_loc(node.message_loc().unwrap_or(node.location()), attribute);
            }
        }
        if self.found.is_none() {
            ruby_prism::visit_call_or_write_node(self, node);
        }
    }

    fn visit_call_and_write_node(&mut self, node: &ruby_prism::CallAndWriteNode<'pr>) {
        if self.found.is_none() && self.receiver_matches_block_param(node.receiver()) {
            if let Some(attribute) = deprecated_attribute_name(node.read_name().as_slice()) {
                self.record_from_loc(node.message_loc().unwrap_or(node.location()), attribute);
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
}
