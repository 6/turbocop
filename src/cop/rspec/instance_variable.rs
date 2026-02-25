use std::collections::HashSet;

use crate::cop::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example_group};
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
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Config: AssignmentOnly — when true, only flag reads of ivars that are
        // also assigned within the same top-level example group. When false (default),
        // flag ALL ivar reads. Writes/assignments are never flagged (matching RuboCop).
        let assignment_only = config.get_bool("AssignmentOnly", false);

        // Pre-compute which call nodes are top-level spec groups.
        // RuboCop's TopLevelGroup only processes describe/context blocks at the
        // file's top level (unwrapping begin, module, and class nodes). Describe
        // blocks nested inside `if` statements, method calls, iterators, etc. are
        // NOT treated as top-level groups and are ignored by this cop.
        let root = parse_result.node();
        let top_level_offsets = root
            .as_program_node()
            .map(|prog| find_top_level_group_offsets(&prog))
            .unwrap_or_default();

        // When AssignmentOnly is true, first pass: collect all assigned ivar names
        // within example groups. RuboCop's ivar_assigned? searches the entire subtree
        // of the top-level group (including defs, classes, etc.) — only excluding
        // nothing. We match that behavior.
        let assigned_names = if assignment_only {
            let mut collector = IvarAssignmentCollector {
                in_example_group: false,
                top_level_offsets: &top_level_offsets,
                assigned_names: HashSet::new(),
            };
            collector.visit(&parse_result.node());
            collector.assigned_names
        } else {
            HashSet::new()
        };

        let mut visitor = IvarChecker {
            source,
            cop: self,
            in_example_group: false,
            in_dynamic_class: false,
            in_custom_matcher: false,
            assignment_only,
            assigned_names,
            top_level_offsets: &top_level_offsets,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// First-pass visitor: collect all ivar names that are assigned within example groups.
/// Matches RuboCop's `ivar_assigned?` which searches the entire subtree without
/// excluding defs, classes, or modules.
struct IvarAssignmentCollector<'a> {
    in_example_group: bool,
    top_level_offsets: &'a HashSet<usize>,
    assigned_names: HashSet<Vec<u8>>,
}

impl IvarAssignmentCollector<'_> {
    fn record_assignment(&mut self, name: &[u8]) {
        if self.in_example_group {
            self.assigned_names.insert(name.to_vec());
        }
    }
}

impl<'pr> Visit<'pr> for IvarAssignmentCollector<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Only enter example group mode for top-level spec groups
        let enters_example_group = self
            .top_level_offsets
            .contains(&node.location().start_offset());

        let was_eg = self.in_example_group;
        if enters_example_group {
            self.in_example_group = true;
        }
        ruby_prism::visit_call_node(self, node);
        self.in_example_group = was_eg;
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        self.record_assignment(node.name().as_slice());
    }

    fn visit_instance_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOperatorWriteNode<'pr>,
    ) {
        self.record_assignment(node.name().as_slice());
    }

    fn visit_instance_variable_or_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOrWriteNode<'pr>,
    ) {
        self.record_assignment(node.name().as_slice());
    }

    fn visit_instance_variable_and_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableAndWriteNode<'pr>,
    ) {
        self.record_assignment(node.name().as_slice());
    }
}

struct IvarChecker<'a> {
    source: &'a SourceFile,
    cop: &'a InstanceVariable,
    in_example_group: bool,
    in_dynamic_class: bool,
    in_custom_matcher: bool,
    assignment_only: bool,
    assigned_names: HashSet<Vec<u8>>,
    top_level_offsets: &'a HashSet<usize>,
    diagnostics: Vec<Diagnostic>,
}

impl IvarChecker<'_> {
    fn should_flag(&self) -> bool {
        self.in_example_group && !self.in_dynamic_class && !self.in_custom_matcher
    }

    fn flag_ivar_read(&mut self, name: &[u8], loc: &ruby_prism::Location<'_>) {
        if !self.should_flag() {
            return;
        }
        // In AssignmentOnly mode, only flag reads where the same ivar is also
        // assigned somewhere in the example group.
        if self.assignment_only && !self.assigned_names.contains(name) {
            return;
        }
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

/// Find the byte offsets of all top-level spec group call nodes.
///
/// Matches RuboCop's `TopLevelGroup#top_level_nodes` which:
/// - If the root is a `:begin` node (multiple top-level statements): returns
///   direct children WITHOUT unwrapping modules/classes
/// - If the root is a `:module` or `:class` (sole top-level construct):
///   recursively unwraps into the body
/// - Otherwise: returns the node as-is
///
/// In Prism, `ProgramNode` always wraps statements in a `StatementsNode`.
/// When there is exactly one top-level statement, we apply module/class
/// unwrapping. When there are multiple statements (like `require` + `module`),
/// we only check direct children for spec groups without unwrapping.
fn find_top_level_group_offsets(program: &ruby_prism::ProgramNode<'_>) -> HashSet<usize> {
    let mut offsets = HashSet::new();
    let body = program.statements();
    let stmts: Vec<_> = body.body().iter().collect();

    if stmts.len() == 1 {
        // Single top-level statement: mirror RuboCop's module/class/else branches.
        // Unwrap module/class/begin, or check if it's a spec group directly.
        collect_top_level_groups(&stmts[0], &mut offsets);
    } else {
        // Multiple top-level statements (like `require 'spec_helper'` + `module Pod`):
        // mirror RuboCop's `:begin` branch — return direct children without unwrapping.
        // Only check if each child is a spec group call directly.
        for stmt in &stmts {
            check_direct_spec_group(stmt, &mut offsets);
        }
    }
    offsets
}

/// Check if a single node is a spec group call and record its offset.
/// Does NOT recurse into module/class/begin — used when there are multiple
/// top-level statements (the `:begin` case in RuboCop).
fn check_direct_spec_group(node: &ruby_prism::Node<'_>, offsets: &mut HashSet<usize>) {
    if let Some(call) = node.as_call_node() {
        if call.block().is_some() {
            let name = call.name().as_slice();
            let is_eg = call.receiver().is_none() && is_rspec_example_group(name);
            let is_rspec_describe = is_rspec_receiver(&call) && name == b"describe";
            if is_eg || is_rspec_describe {
                offsets.insert(call.location().start_offset());
            }
        }
    }
}

/// Recursively collect top-level spec group offsets, unwrapping begin/module/class nodes.
/// Only used when the node is the sole top-level construct (matching RuboCop's
/// `:module`/`:class` branch in `top_level_nodes`).
fn collect_top_level_groups(node: &ruby_prism::Node<'_>, offsets: &mut HashSet<usize>) {
    // Check if this node is a spec group call (describe/context/etc with a block)
    if let Some(call) = node.as_call_node() {
        if call.block().is_some() {
            let name = call.name().as_slice();
            let is_eg = call.receiver().is_none() && is_rspec_example_group(name);
            let is_rspec_describe = is_rspec_receiver(&call) && name == b"describe";
            if is_eg || is_rspec_describe {
                offsets.insert(call.location().start_offset());
                return;
            }
        }
    }

    // Unwrap module nodes (sole top-level module — recurse into body)
    if let Some(module_node) = node.as_module_node() {
        if let Some(body) = module_node.body() {
            if let Some(stmts) = body.as_statements_node() {
                for child in stmts.body().iter() {
                    collect_top_level_groups(&child, offsets);
                }
            }
        }
        return;
    }

    // Unwrap class nodes (sole top-level class — recurse into body)
    if let Some(class_node) = node.as_class_node() {
        if let Some(body) = class_node.body() {
            if let Some(stmts) = body.as_statements_node() {
                for child in stmts.body().iter() {
                    collect_top_level_groups(&child, offsets);
                }
            }
        }
        return;
    }

    // Unwrap begin nodes (explicit begin..end)
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for child in stmts.body().iter() {
                collect_top_level_groups(&child, offsets);
            }
        }
    }
}

/// Check if the receiver of a CallNode is `RSpec` (simple constant) or `::RSpec`
/// (constant path with cbase). Matches RuboCop's `(const {nil? cbase} :RSpec)`.
fn is_rspec_receiver(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(recv) = call.receiver() {
        // Simple `RSpec` constant
        if let Some(cr) = recv.as_constant_read_node() {
            return cr.name().as_slice() == b"RSpec";
        }
        // Qualified `::RSpec` constant path
        if let Some(cp) = recv.as_constant_path_node() {
            if let Some(name) = cp.name() {
                if name.as_slice() == b"RSpec" {
                    // Parent must be nil (cbase ::RSpec) — no deeper nesting
                    return cp.parent().is_none();
                }
            }
        }
    }
    false
}

/// Check if a call is `Class.new` — the only dynamic class pattern RuboCop excludes.
/// RuboCop's pattern: `(block (send (const nil? :Class) :new ...) ...)`
/// This matches only `Class.new`, not `Struct.new`, `Module.new`, etc.
fn is_dynamic_class_call(call: &ruby_prism::CallNode<'_>) -> bool {
    let method = call.name().as_slice();
    if method != b"new" {
        return false;
    }
    if let Some(recv) = call.receiver() {
        if let Some(cr) = recv.as_constant_read_node() {
            return cr.name().as_slice() == b"Class";
        }
    }
    false
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
        let has_block = node.block().is_some();

        // Only enter example group mode for top-level spec groups.
        // RuboCop's TopLevelGroup only processes describe/context blocks at the
        // file's top level. Nested describe/context within a top-level group
        // are already inside in_example_group and don't need separate detection.
        let enters_example_group = self
            .top_level_offsets
            .contains(&node.location().start_offset());
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

    // RuboCop's ivar_usage search descends into def, class, and module nodes.
    // We do NOT override visit_def_node, visit_class_node, or visit_module_node
    // so that the default visitor descends into them, matching RuboCop behavior.

    fn visit_instance_variable_read_node(
        &mut self,
        node: &ruby_prism::InstanceVariableReadNode<'pr>,
    ) {
        self.flag_ivar_read(node.name().as_slice(), &node.location());
        ruby_prism::visit_instance_variable_read_node(self, node);
    }

    // Instance variable writes/assignments are never flagged by this cop.
    // RuboCop's RSpec/InstanceVariable only flags reads (ivar nodes),
    // not assignments (ivasgn nodes).
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InstanceVariable, "cops/rspec/instance_variable");

    #[test]
    fn assignment_only_skips_reads_without_assignment() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("AssignmentOnly".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        // @bar is read but never assigned — should not be flagged in AssignmentOnly mode
        let source = b"describe Foo do\n  it 'reads' do\n    @bar\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&InstanceVariable, source, config);
        assert!(
            diags.is_empty(),
            "AssignmentOnly should skip reads when ivar is not assigned"
        );
    }

    #[test]
    fn assignment_only_flags_reads_with_assignment() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("AssignmentOnly".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        // @foo is assigned in before and read in it — should be flagged
        let source =
            b"describe Foo do\n  before { @foo = [] }\n  it { expect(@foo).to be_empty }\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&InstanceVariable, source, config);
        assert_eq!(
            diags.len(),
            1,
            "AssignmentOnly should flag reads when ivar is also assigned"
        );
    }

    #[test]
    fn writes_are_never_flagged() {
        // Instance variable writes (assignments) should never be flagged
        let source = b"describe Foo do\n  before { @bar = 1 }\nend\n";
        let diags = crate::testutil::run_cop_full(&InstanceVariable, source);
        assert!(
            diags.is_empty(),
            "Writes/assignments should never be flagged"
        );
    }

    #[test]
    fn ivar_read_inside_def_is_flagged() {
        // RuboCop flags ivar reads inside def methods within describe blocks
        let source = b"describe Foo do\n  def helper\n    @bar\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&InstanceVariable, source);
        assert_eq!(
            diags.len(),
            1,
            "Instance variable read inside def within describe should be flagged"
        );
    }

    #[test]
    fn class_new_block_is_excluded() {
        // Class.new blocks are excluded (dynamic class)
        let source = b"describe Foo do\n  let(:klass) do\n    Class.new do\n      def init\n        @x = 1\n      end\n      def val\n        @x\n      end\n    end\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&InstanceVariable, source);
        assert!(
            diags.is_empty(),
            "Instance variables inside Class.new blocks should not be flagged"
        );
    }

    #[test]
    fn struct_new_block_is_not_excluded() {
        // Struct.new blocks are NOT excluded (only Class.new is)
        let source = b"describe Foo do\n  let(:klass) do\n    Struct.new(:name) do\n      def val\n        @x\n      end\n    end\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&InstanceVariable, source);
        assert_eq!(
            diags.len(),
            1,
            "Instance variables inside Struct.new blocks should be flagged"
        );
    }

    #[test]
    fn cbase_rspec_describe_is_recognized() {
        // ::RSpec.describe should be recognized as an example group
        let source = b"::RSpec.describe Foo do\n  it { @bar }\nend\n";
        let diags = crate::testutil::run_cop_full(&InstanceVariable, source);
        assert_eq!(
            diags.len(),
            1,
            "::RSpec.describe should be recognized as an example group"
        );
    }

    #[test]
    fn describe_inside_if_is_not_flagged() {
        // RuboCop's TopLevelGroup only recognizes describe at the file top level.
        // Describe blocks nested inside `if` statements are not top-level groups.
        let source = b"if defined?(SomeGem)\n  describe Foo do\n    before { @x = 1 }\n    it { @x }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&InstanceVariable, source);
        assert!(
            diags.is_empty(),
            "Instance variable read inside describe wrapped in if should not be flagged"
        );
    }

    #[test]
    fn describe_inside_block_is_not_flagged() {
        // Describe inside a non-RSpec method block is not a top-level group
        let source = b"some_method do\n  describe Foo do\n    it { @bar }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&InstanceVariable, source);
        assert!(
            diags.is_empty(),
            "Instance variable read inside describe wrapped in a block should not be flagged"
        );
    }

    #[test]
    fn describe_inside_module_is_flagged() {
        // RuboCop's TopLevelGroup unwraps module nodes — describe inside module IS top-level
        let source = b"module MyModule\n  describe Foo do\n    it { @bar }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&InstanceVariable, source);
        assert_eq!(
            diags.len(),
            1,
            "Instance variable read inside describe wrapped in module should be flagged"
        );
    }

    #[test]
    fn describe_inside_class_is_flagged() {
        // RuboCop's TopLevelGroup unwraps class nodes — describe inside class IS top-level
        let source = b"class MyClass\n  describe Foo do\n    it { @bar }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&InstanceVariable, source);
        assert_eq!(
            diags.len(),
            1,
            "Instance variable read inside describe wrapped in class should be flagged"
        );
    }

    #[test]
    fn module_with_require_sibling_is_not_top_level() {
        // RuboCop's TopLevelGroup only unwraps module/class when it is the SOLE
        // top-level construct. When other statements exist (like require), modules
        // are treated as opaque and NOT unwrapped.
        let source =
            b"require 'spec_helper'\nmodule Pod\n  describe Foo do\n    it { @bar }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&InstanceVariable, source);
        assert!(
            diags.is_empty(),
            "Module with require sibling should not be treated as top-level group"
        );
    }

    #[test]
    fn class_with_require_sibling_is_not_top_level() {
        // Same as above but for class nodes
        let source = b"require 'spec_helper'\nclass MySpec\n  describe Foo do\n    it { @bar }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&InstanceVariable, source);
        assert!(
            diags.is_empty(),
            "Class with require sibling should not be treated as top-level group"
        );
    }

    #[test]
    fn sole_module_is_still_unwrapped() {
        // When module is the sole top-level construct, it should still be unwrapped
        let source = b"module MyModule\n  describe Foo do\n    it { @bar }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&InstanceVariable, source);
        assert_eq!(
            diags.len(),
            1,
            "Sole module should still be unwrapped as top-level group"
        );
    }

    #[test]
    fn describe_with_require_sibling_is_still_detected() {
        // When describe is a direct top-level statement alongside require,
        // it should still be detected as a top-level group
        let source = b"require 'spec_helper'\ndescribe Foo do\n  it { @bar }\nend\n";
        let diags = crate::testutil::run_cop_full(&InstanceVariable, source);
        assert_eq!(
            diags.len(),
            1,
            "Direct describe alongside require should still be detected"
        );
    }
}
