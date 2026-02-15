use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct InstanceVariable;

impl Cop for InstanceVariable {
    fn name(&self) -> &'static str {
        "RSpec/InstanceVariable"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Config: AssignmentOnly — only flag assignments, not reads
        let assignment_only = config.get_bool("AssignmentOnly", false);
        let mut visitor = IvarChecker {
            source,
            cop: self,
            in_def: false,
            assignment_only,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct IvarChecker<'a> {
    source: &'a SourceFile,
    cop: &'a InstanceVariable,
    in_def: bool,
    assignment_only: bool,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for IvarChecker<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let was = self.in_def;
        self.in_def = true;
        ruby_prism::visit_def_node(self, node);
        self.in_def = was;
    }

    fn visit_instance_variable_read_node(
        &mut self,
        node: &ruby_prism::InstanceVariableReadNode<'pr>,
    ) {
        if !self.in_def && !self.assignment_only {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Avoid instance variables - use let, a method call, or a local variable (if possible)."
                    .to_string(),
            ));
        }
        ruby_prism::visit_instance_variable_read_node(self, node);
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        if !self.in_def {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Avoid instance variables - use let, a method call, or a local variable (if possible)."
                    .to_string(),
            ));
        }
        ruby_prism::visit_instance_variable_write_node(self, node);
    }

    fn visit_instance_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOperatorWriteNode<'pr>,
    ) {
        if !self.in_def {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Avoid instance variables - use let, a method call, or a local variable (if possible)."
                    .to_string(),
            ));
        }
        ruby_prism::visit_instance_variable_operator_write_node(self, node);
    }

    fn visit_instance_variable_or_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOrWriteNode<'pr>,
    ) {
        if !self.in_def {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Avoid instance variables - use let, a method call, or a local variable (if possible)."
                    .to_string(),
            ));
        }
        ruby_prism::visit_instance_variable_or_write_node(self, node);
    }

    fn visit_instance_variable_and_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableAndWriteNode<'pr>,
    ) {
        if !self.in_def {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Avoid instance variables - use let, a method call, or a local variable (if possible)."
                    .to_string(),
            ));
        }
        ruby_prism::visit_instance_variable_and_write_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InstanceVariable, "cops/rspec/instance_variable");

    #[test]
    fn assignment_only_skips_reads() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AssignmentOnly".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };
        let source = b"describe Foo do\n  it 'reads' do\n    @bar\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&InstanceVariable, source, config);
        assert!(diags.is_empty(), "AssignmentOnly should skip reads");
    }

    #[test]
    fn assignment_only_still_flags_writes() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AssignmentOnly".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };
        let source = b"describe Foo do\n  before { @bar = 1 }\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&InstanceVariable, source, config);
        assert_eq!(diags.len(), 1);
    }
}
