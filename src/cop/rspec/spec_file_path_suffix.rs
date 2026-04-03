use crate::cop::shared::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, PROGRAM_NODE,
};
use crate::cop::shared::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-14)
///
/// FP=2: Two files with non-`_spec.rb` paths matched because the cop was not
/// checking the receiver of `describe`/`context` calls.
///
/// - `spec/support/analyzer/98_misc.rb` had `1.describe('...')` — a method call
///   on an integer literal, NOT an RSpec example group.
/// - `spec/dummy/config/events.rb` had `WebsocketRails::EventMap.describe` — a
///   method on a constant, NOT a bare RSpec describe.
///
/// Fix: added receiver check so only receiverless calls (or `RSpec.describe`) count
/// as example groups for this cop, matching RuboCop's behavior.
///
/// ## Corpus investigation (2026-03-18)
///
/// FN=16: Files with `describe` blocks wrapped inside `module` or `class`
/// declarations were not detected. RuboCop's `TopLevelGroup` mixin recursively
/// descends through `module`/`class` nodes to find top-level example groups,
/// but nitrocop only checked direct children of the program body.
///
/// Fix: added recursive `collect_top_level_nodes` to descend through
/// `ModuleNode`/`ClassNode` wrappers, matching RuboCop's `top_level_nodes`.
///
/// FP=6: Files with multiple top-level statements (e.g., `require` + class/module
/// wrapping describe) were incorrectly flagged. RuboCop's `top_level_nodes`
/// only recurses into class/module when there is a single top-level statement.
/// With multiple statements (`:begin` node in Parser), it returns direct children
/// only — so a describe nested inside a class is NOT found.
///
/// Fix: check `body.len()`. If single statement, recurse (has_example_group_in_nodes).
/// If multiple statements, only check direct children (has_direct_example_group).
pub struct SpecFilePathSuffix;

impl Cop for SpecFilePathSuffix {
    fn name(&self) -> &'static str {
        "RSpec/SpecFilePathSuffix"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            PROGRAM_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Only check ProgramNode (root)
        let program = match node.as_program_node() {
            Some(p) => p,
            None => return,
        };

        let stmts = program.statements();
        let body = stmts.body();

        // Match RuboCop's TopLevelGroup#top_level_nodes behavior:
        // - Single top-level statement: recurse into module/class wrappers
        // - Multiple top-level statements (:begin): only check direct children
        let has_example_group = if body.len() == 1 {
            has_example_group_in_nodes(body.iter())
        } else {
            has_direct_example_group(body.iter())
        };

        if !has_example_group {
            return;
        }

        let path = source.path_str();
        if path.ends_with("_spec.rb") {
            return;
        }

        // File-level offense — report at line 1, column 0
        diagnostics.push(self.diagnostic(
            source,
            1,
            0,
            "Spec path should end with `_spec.rb`.".to_string(),
        ));
    }
}

/// Check direct children only for example group calls (no class/module recursion).
/// Used when there are multiple top-level statements, matching RuboCop's
/// `TopLevelGroup#top_level_nodes` which returns `:begin` node children directly.
fn has_direct_example_group<'a>(nodes: impl Iterator<Item = ruby_prism::Node<'a>>) -> bool {
    for node in nodes {
        if let Some(call) = node.as_call_node() {
            if is_rspec_example_group_call(&call) {
                return true;
            }
        }
    }
    false
}

/// Recursively descend through `module`/`class` nodes to collect the
/// innermost statements, matching RuboCop's `TopLevelGroup#top_level_nodes`.
/// This ensures `describe` blocks wrapped in `module Foo; ... end` are found.
fn has_example_group_in_nodes<'a>(nodes: impl Iterator<Item = ruby_prism::Node<'a>>) -> bool {
    for node in nodes {
        if has_example_group_node(&node) {
            return true;
        }
    }
    false
}

fn has_example_group_node(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(module_node) = node.as_module_node() {
        if let Some(body) = module_node.body() {
            if let Some(stmts) = body.as_statements_node() {
                return has_example_group_in_nodes(stmts.body().iter());
            }
        }
        return false;
    }
    if let Some(class_node) = node.as_class_node() {
        if let Some(body) = class_node.body() {
            if let Some(stmts) = body.as_statements_node() {
                return has_example_group_in_nodes(stmts.body().iter());
            }
        }
        return false;
    }
    if let Some(call) = node.as_call_node() {
        return is_rspec_example_group_call(&call);
    }
    false
}

fn is_rspec_example_group_call(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    // Check receiver: must be None, or be RSpec/::RSpec
    let ok_receiver = match call.receiver() {
        None => true,
        Some(recv) => {
            if let Some(cr) = recv.as_constant_read_node() {
                cr.name().as_slice() == b"RSpec"
            } else if let Some(cp) = recv.as_constant_path_node() {
                cp.parent().is_none() && cp.name().is_some_and(|n| n.as_slice() == b"RSpec")
            } else {
                false
            }
        }
    };
    if ok_receiver
        && is_rspec_example_group(name)
        && name != b"shared_examples"
        && name != b"shared_examples_for"
        && name != b"shared_context"
    {
        return true;
    }
    // Also handle feature (receiverless only)
    if call.receiver().is_none() && name == b"feature" {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_scenario_fixture_tests!(
        SpecFilePathSuffix,
        "cops/rspec/spec_file_path_suffix",
        scenario_repeated_rb = "repeated_rb.rb",
        scenario_missing_spec = "missing_spec.rb",
        scenario_wrong_ext = "wrong_ext.rb",
        scenario_module_wrapped = "module_wrapped.rb",
        scenario_class_wrapped = "class_wrapped.rb",
    );

    #[test]
    fn integer_receiver_describe_not_flagged() {
        // FP fix: 1.describe('...') has a receiver (integer) — not an RSpec example group
        // File is in spec/ dir (matches **/spec/**/*) but path is not _spec.rb
        let source = b"1.describe('method call with an argument and a block') do\n  it { expect(true).to eq(true) }\nend\n";
        let diags = crate::testutil::run_cop_full_internal(
            &SpecFilePathSuffix,
            source,
            crate::cop::CopConfig::default(),
            "spec/support/analyzer/98_misc.rb",
        );
        assert_eq!(
            diags.len(),
            0,
            "1.describe should not trigger SpecFilePathSuffix: {:?}",
            diags
        );
    }

    #[test]
    fn multi_stmt_require_plus_class_with_describe_not_flagged() {
        // FP fix: when there are multiple top-level statements, RuboCop's
        // TopLevelGroup only checks direct children (not recursing into class/module).
        let source = b"require 'helper'\nclass TestHelpers\n  describe \"some method\" do\n    it { expect(true).to eq(true) }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_internal(
            &SpecFilePathSuffix,
            source,
            crate::cop::CopConfig::default(),
            "spec/treat.rb",
        );
        assert_eq!(
            diags.len(),
            0,
            "require + class wrapping describe should not trigger SpecFilePathSuffix: {:?}",
            diags
        );
    }

    #[test]
    fn multi_stmt_require_plus_module_with_rspec_describe_not_flagged() {
        // FP fix: require + module wrapping RSpec.describe — multiple top-level stmts
        let source = b"require 'rails_helper'\nmodule UserAssignments\n  RSpec.describe GenerateJob do\n    it { expect(true).to eq(true) }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_internal(
            &SpecFilePathSuffix,
            source,
            crate::cop::CopConfig::default(),
            "spec/jobs/user_assignments/generate_user_assignments_job.rb",
        );
        assert_eq!(
            diags.len(),
            0,
            "require + module wrapping RSpec.describe should not trigger SpecFilePathSuffix: {:?}",
            diags
        );
    }

    #[test]
    fn multi_stmt_require_plus_class_with_context_not_flagged() {
        // FP fix: require + class containing context — multiple top-level stmts
        let source = b"require 'test_helper'\nclass BackfillTaskTest\n  context 'when running' do\n    it { expect(true).to eq(true) }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_internal(
            &SpecFilePathSuffix,
            source,
            crate::cop::CopConfig::default(),
            "test/tasks/maintenance/backfill_task_test.rb",
        );
        assert_eq!(
            diags.len(),
            0,
            "require + class with context should not trigger SpecFilePathSuffix: {:?}",
            diags
        );
    }

    #[test]
    fn single_stmt_class_with_describe_still_flagged() {
        // Ensure single top-level class wrapping describe IS still flagged
        let source = b"class DocumentTest\n  describe Document do\n    it 'works' do\n      expect(true).to eq(true)\n    end\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_internal(
            &SpecFilePathSuffix,
            source,
            crate::cop::CopConfig::default(),
            "spec/entities/document.rb",
        );
        assert_eq!(
            diags.len(),
            1,
            "Single top-level class wrapping describe should still trigger SpecFilePathSuffix: {:?}",
            diags
        );
    }

    #[test]
    fn single_stmt_module_with_describe_still_flagged() {
        // Ensure single top-level module wrapping describe IS still flagged
        let source = b"module Specs\n  module Entities\n    describe Collection do\n      it 'works' do\n        expect(true).to eq(true)\n      end\n    end\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_internal(
            &SpecFilePathSuffix,
            source,
            crate::cop::CopConfig::default(),
            "spec/entities/collection.rb",
        );
        assert_eq!(
            diags.len(),
            1,
            "Single top-level module wrapping describe should still trigger SpecFilePathSuffix: {:?}",
            diags
        );
    }

    #[test]
    fn constant_receiver_describe_not_flagged() {
        // FP fix: SomeModule.describe has a receiver (constant) — not an RSpec example group
        let source =
            b"WebsocketRails::EventMap.describe do\n  subscribe :foo, to: SomeController\nend\n";
        let diags = crate::testutil::run_cop_full_internal(
            &SpecFilePathSuffix,
            source,
            crate::cop::CopConfig::default(),
            "spec/dummy/config/events.rb",
        );
        assert_eq!(
            diags.len(),
            0,
            "WebsocketRails::EventMap.describe should not trigger SpecFilePathSuffix: {:?}",
            diags
        );
    }
}
