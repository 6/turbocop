use crate::cop::util::{is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
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
            in_example_group: false,
            in_def: false,
            in_dynamic_class: false,
            in_custom_matcher: false,
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
    in_example_group: bool,
    in_def: bool,
    in_dynamic_class: bool,
    in_custom_matcher: bool,
    assignment_only: bool,
    diagnostics: Vec<Diagnostic>,
}

impl IvarChecker<'_> {
    fn should_flag(&self) -> bool {
        self.in_example_group && !self.in_def && !self.in_dynamic_class && !self.in_custom_matcher
    }

    fn flag_ivar(&mut self, loc: &ruby_prism::Location<'_>) {
        if self.should_flag() {
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Avoid instance variables - use let, a method call, or a local variable (if possible)."
                    .to_string(),
            ));
        }
    }
}

/// Check if a call is `Class.new`, `Module.new`, `Struct.new`, or `Data.define`
fn is_dynamic_class_call(call: &ruby_prism::CallNode<'_>) -> bool {
    let method = call.name().as_slice();
    if let Some(recv) = call.receiver() {
        if let Some(cr) = recv.as_constant_read_node() {
            let name = cr.name().as_slice();
            if (name == b"Class" || name == b"Module" || name == b"Struct") && method == b"new" {
                return true;
            }
            if name == b"Data" && method == b"define" {
                return true;
            }
        }
    }
    // Also check class_eval, module_eval, instance_eval, *_exec
    matches!(
        method,
        b"class_eval" | b"module_eval" | b"instance_eval"
            | b"class_exec" | b"module_exec" | b"instance_exec"
    )
}

/// Check if a call is `RSpec::Matchers.define :name` or `matcher :name`
fn is_custom_matcher_call(call: &ruby_prism::CallNode<'_>) -> bool {
    let method = call.name().as_slice();
    if method == b"matcher" && call.receiver().is_none() {
        return true;
    }
    if method == b"define" {
        if let Some(recv) = call.receiver() {
            if let Some(cp) = recv.as_constant_path_node() {
                if let Some(name) = cp.name() {
                    if name.as_slice() == b"Matchers" {
                        if let Some(parent) = cp.parent() {
                            if let Some(cr) = parent.as_constant_read_node() {
                                return cr.name().as_slice() == b"RSpec";
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

impl<'pr> Visit<'pr> for IvarChecker<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();
        let has_block = node.block().is_some();

        // Check if this is an example group call with a block
        let is_eg = has_block && node.receiver().is_none() && is_rspec_example_group(name);
        let is_rspec_describe = has_block
            && node.receiver().is_some_and(|r| {
                r.as_constant_read_node()
                    .is_some_and(|c| c.name().as_slice() == b"RSpec")
            })
            && name == b"describe";

        let enters_example_group = is_eg || is_rspec_describe;
        let enters_dynamic_class = has_block && is_dynamic_class_call(node);
        let enters_custom_matcher = has_block && is_custom_matcher_call(node);

        let was_eg = self.in_example_group;
        let was_dc = self.in_dynamic_class;
        let was_cm = self.in_custom_matcher;

        if enters_example_group {
            self.in_example_group = true;
        }
        if enters_dynamic_class {
            self.in_dynamic_class = true;
        }
        if enters_custom_matcher {
            self.in_custom_matcher = true;
        }

        ruby_prism::visit_call_node(self, node);

        self.in_example_group = was_eg;
        self.in_dynamic_class = was_dc;
        self.in_custom_matcher = was_cm;
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let was = self.in_def;
        self.in_def = true;
        ruby_prism::visit_def_node(self, node);
        self.in_def = was;
    }

    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {
        // Don't descend into class definitions — scope changes
    }

    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {
        // Don't descend into module definitions — scope changes
    }

    fn visit_instance_variable_read_node(
        &mut self,
        node: &ruby_prism::InstanceVariableReadNode<'pr>,
    ) {
        if !self.assignment_only {
            self.flag_ivar(&node.location());
        }
        ruby_prism::visit_instance_variable_read_node(self, node);
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        self.flag_ivar(&node.location());
        ruby_prism::visit_instance_variable_write_node(self, node);
    }

    fn visit_instance_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOperatorWriteNode<'pr>,
    ) {
        self.flag_ivar(&node.location());
        ruby_prism::visit_instance_variable_operator_write_node(self, node);
    }

    fn visit_instance_variable_or_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOrWriteNode<'pr>,
    ) {
        self.flag_ivar(&node.location());
        ruby_prism::visit_instance_variable_or_write_node(self, node);
    }

    fn visit_instance_variable_and_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableAndWriteNode<'pr>,
    ) {
        self.flag_ivar(&node.location());
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
