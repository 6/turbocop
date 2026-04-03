use crate::cop::shared::constant_predicates;
use crate::cop::shared::node_type::PROGRAM_NODE;
use crate::cop::shared::util::{
    RSPEC_DEFAULT_INCLUDE, is_rspec_example_group, is_rspec_shared_group,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-04): 784 FPs, 194 FNs.
///
/// Root causes:
/// 1. FPs: Bare `context` calls without blocks (e.g., `expect(context).to be_failure`)
///    were matched as example groups and counted toward nesting depth. Fix: require
///    a block (`do...end`) on the call node before counting it as an example group,
///    matching RuboCop's behavior where only `:block` children are recursed into.
/// 2. FNs: `RSpec.shared_examples`, `RSpec.shared_examples_for`, `RSpec.shared_context`
///    (with `RSpec` receiver) were not recognized as shared groups. Only receiverless
///    forms were handled. Fix: also check for `RSpec.` prefix on shared group methods.
/// 3. FNs: `RSpec.feature` and other example group methods with `RSpec.` prefix were
///    not recognized as example groups. Only `RSpec.describe` was handled. Fix: accept
///    any `is_rspec_example_group` method name with `RSpec` receiver.
///
/// ## Corpus investigation (2026-03-07)
///
/// Corpus oracle reported FP=218, FN=0.
///
/// FP=218 root cause: traversal descended into conditional wrappers (`if`, `unless`, etc.),
/// so example groups nested inside those branches were counted. RuboCop only recurses
/// through `:block` and `:begin` descendants for this cop, so groups under conditionals
/// are intentionally ignored.
///
/// Fix: replace generic AST visitor traversal with a RuboCop-aligned walker that
/// recurses only through call nodes with blocks (Prism equivalent of Parser `:block`)
/// and explicit `BeginNode` wrappers.
///
/// FN=0: no missed-detection change expected from this fix.
///
/// ## Corpus investigation (2026-03-20)
///
/// Corpus oracle reported FP=0, FN=4. All from DataDog/datadog-ci-rb.
///
/// Root cause: `walk_block_or_begin` only counted receiverless example groups
/// (`context`, `describe`) toward nesting depth, but NOT `RSpec.describe` or
/// `RSpec.context` (with `RSpec` receiver). In the corpus file, `RSpec.describe`
/// was used inside non-example-group blocks (e.g., `it` / helper method blocks),
/// and its depth contribution was skipped due to the `call.receiver().is_none()`
/// guard. RuboCop's `example_group?` recognizes both forms.
///
/// Fix: check for example groups with or without `RSpec` receiver when deciding
/// whether to increment nesting depth, matching the top-level detection logic.
pub struct NestedGroups;

impl Cop for NestedGroups {
    fn name(&self) -> &'static str {
        "RSpec/NestedGroups"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[PROGRAM_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let program = match node.as_program_node() {
            Some(p) => p,
            None => return,
        };

        let max = config.get_usize("Max", 3);
        let allowed_groups = config.get_string_array("AllowedGroups").unwrap_or_default();

        // Walk top-level statements to find top-level spec groups.
        // This mirrors RuboCop's TopLevelGroup#top_level_nodes which:
        // - For a single top-level statement: unwraps module/class/begin
        // - For multiple top-level statements: checks direct children only
        let stmts: Vec<_> = program.statements().body().iter().collect();
        if stmts.len() == 1 {
            // Single top-level statement: unwrap module/class wrappers
            self.check_top_level_node(source, &stmts[0], max, &allowed_groups, diagnostics);
        } else {
            // Multiple top-level statements (e.g., require + module):
            // only check direct children for spec groups, no unwrapping
            for stmt in &stmts {
                self.check_direct_spec_group(source, stmt, max, &allowed_groups, diagnostics);
            }
        }
    }
}

impl NestedGroups {
    /// Check a direct top-level statement for spec groups WITHOUT unwrapping
    /// module/class nodes. Used when there are multiple top-level statements
    /// (matching RuboCop's `:begin` branch in `top_level_nodes`).
    fn check_direct_spec_group(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        max: usize,
        allowed_groups: &[String],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Only check if this node is a spec group call — no module/class unwrapping
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        self.process_spec_group_call(source, &call, max, allowed_groups, diagnostics);
    }

    /// Check a top-level AST node for spec groups. Recurses into
    /// module/class wrappers to find describe/shared_examples at the
    /// logical top level, mirroring RuboCop's `TopLevelGroup#top_level_nodes`.
    fn check_top_level_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        max: usize,
        allowed_groups: &[String],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Recurse into module/class wrappers (RuboCop's top_level_nodes)
        if let Some(module_node) = node.as_module_node() {
            if let Some(body) = module_node.body() {
                if let Some(stmts) = body.as_statements_node() {
                    for stmt in stmts.body().iter() {
                        self.check_top_level_node(source, &stmt, max, allowed_groups, diagnostics);
                    }
                }
            }
            return;
        }
        if let Some(class_node) = node.as_class_node() {
            if let Some(body) = class_node.body() {
                if let Some(stmts) = body.as_statements_node() {
                    for stmt in stmts.body().iter() {
                        self.check_top_level_node(source, &stmt, max, allowed_groups, diagnostics);
                    }
                }
            }
            return;
        }

        // Check if this is a spec group call (describe, shared_examples, etc.)
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        self.process_spec_group_call(source, &call, max, allowed_groups, diagnostics);
    }

    /// Process a call node that may be a spec group (describe, shared_examples, etc.)
    /// and walk its block body for nested groups.
    fn process_spec_group_call<'pr>(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'pr>,
        max: usize,
        allowed_groups: &[String],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let method_name = call.name().as_slice();

        // Determine if this is a shared group or example group.
        // Shared groups are checked first because is_rspec_example_group also
        // matches shared group names.
        // Shared groups can be either receiverless (`shared_examples 'name' do`)
        // or with RSpec receiver (`RSpec.shared_examples 'name' do`).
        let is_shared_group = if call.receiver().is_none() {
            is_rspec_shared_group(method_name)
        } else {
            constant_predicates::constant_short_name(&call.receiver().unwrap())
                .is_some_and(|n| n == b"RSpec")
                && is_rspec_shared_group(method_name)
        };
        let is_example_group = if is_shared_group {
            false
        } else if let Some(recv) = call.receiver() {
            constant_predicates::constant_short_name(&recv).is_some_and(|n| n == b"RSpec")
                && is_rspec_example_group(method_name)
        } else {
            is_rspec_example_group(method_name)
        };

        if !is_example_group && !is_shared_group {
            return;
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        // Shared groups (shared_examples, shared_examples_for, shared_context)
        // do NOT count toward nesting depth — they define reusable groups.
        // RuboCop's `example_group?` returns false for shared groups, so the
        // nesting counter does not increment for the top-level shared group.
        let initial_depth = if is_shared_group { 0 } else { 1 };

        // Walk the block body looking for nested groups
        if let Some(body) = block.body() {
            let mut visitor = NestingVisitor {
                source,
                max,
                depth: initial_depth,
                diagnostics,
                cop: self,
                allowed_groups,
            };
            visitor.walk_nested_groups(&body);
        }
    }
}

struct NestingVisitor<'a> {
    source: &'a SourceFile,
    max: usize,
    depth: usize,
    diagnostics: &'a mut Vec<Diagnostic>,
    cop: &'a NestedGroups,
    allowed_groups: &'a [String],
}

impl NestingVisitor<'_> {
    fn walk_nested_groups<'pr>(&mut self, node: &ruby_prism::Node<'pr>) {
        if let Some(stmts) = node.as_statements_node() {
            for stmt in stmts.body().iter() {
                self.walk_block_or_begin(&stmt);
            }
            return;
        }

        self.walk_block_or_begin(node);
    }

    fn walk_block_or_begin<'pr>(&mut self, node: &ruby_prism::Node<'pr>) {
        if let Some(begin_node) = node.as_begin_node() {
            if let Some(stmts) = begin_node.statements() {
                for stmt in stmts.body().iter() {
                    self.walk_block_or_begin(&stmt);
                }
            }
            return;
        }

        let call = match node.as_call_node() {
            Some(call) => call,
            None => return,
        };
        let block = match call.block().and_then(|b| b.as_block_node()) {
            Some(block) => block,
            None => return,
        };

        let method_name = call.name().as_slice();
        let is_shared = if call.receiver().is_none() {
            is_rspec_shared_group(method_name)
        } else {
            constant_predicates::constant_short_name(&call.receiver().unwrap())
                .is_some_and(|n| n == b"RSpec")
                && is_rspec_shared_group(method_name)
        };

        // Non-shared example groups count toward nesting depth unless allowed.
        // This matches RuboCop's `count_up_nesting?` logic on block nodes.
        let mut next_depth = self.depth;
        if !is_shared {
            let is_allowed = self
                .allowed_groups
                .iter()
                .any(|group| group.as_bytes() == method_name);
            // Example groups with or without RSpec receiver count toward nesting.
            // RuboCop's `example_group?` matches both `context do` and `RSpec.describe do`.
            let is_example_group = if let Some(recv) = call.receiver() {
                constant_predicates::constant_short_name(&recv).is_some_and(|n| n == b"RSpec")
                    && is_rspec_example_group(method_name)
            } else {
                is_rspec_example_group(method_name)
            };
            if is_example_group && !is_allowed {
                next_depth += 1;
                if next_depth > self.max {
                    let loc = call.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        format!(
                            "Maximum example group nesting exceeded [{next_depth}/{}].",
                            self.max
                        ),
                    ));
                }
            }
        }

        // RuboCop recurses only through :block and :begin children.
        // In Prism, block statements are represented as call nodes with block bodies.
        if let Some(body) = block.body() {
            let old_depth = self.depth;
            self.depth = next_depth;
            self.walk_nested_groups(&body);
            self.depth = old_depth;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NestedGroups, "cops/rspec/nested_groups");

    #[test]
    fn allowed_groups_skips_matching() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                (
                    "Max".into(),
                    serde_yml::Value::Number(serde_yml::Number::from(1)),
                ),
                (
                    "AllowedGroups".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("context".into())]),
                ),
            ]),
            ..CopConfig::default()
        };
        // describe > context (allowed, not counted) — depth stays 1
        let source =
            b"describe Foo do\n  context 'bar' do\n    it 'works' do\n    end\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&NestedGroups, source, config);
        assert!(
            diags.is_empty(),
            "AllowedGroups should not count matching groups"
        );
    }

    #[test]
    fn module_with_require_sibling_is_not_unwrapped() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        // With Max=1, nesting of describe > context > context would be depth 3 (exceeding 1).
        // But when the module has a require sibling, the module should NOT be unwrapped,
        // so the describe inside is not detected as a top-level group at all.
        let config = CopConfig {
            options: HashMap::from([(
                "Max".into(),
                serde_yml::Value::Number(serde_yml::Number::from(1)),
            )]),
            ..CopConfig::default()
        };
        let source = b"require 'spec_helper'\nmodule Pod\n  describe Foo do\n    context 'bar' do\n      context 'baz' do\n        it 'works' do\n        end\n      end\n    end\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&NestedGroups, source, config);
        assert!(
            diags.is_empty(),
            "Module with require sibling should not be unwrapped for top-level group detection"
        );
    }

    #[test]
    fn reduced_fp_case() {
        // Reduced from corpus FP in light-service. This file has depth 3 nesting
        // (RSpec.describe > describe > context) which equals Max=3, NOT exceeding it.
        // nitrocop was reporting a false positive somewhere in this structure.
        let source = b"RSpec.describe LightService::Context do\n  describe \"can be made\" do\n    context \"with no arguments\" do\n      specify \"message is empty string\" do\n      end\n    end\n    context \"with a hash\" do\n    end\n    context \"with FAILURE\" do\n      it \"is failed\" do\n        expect(context).to be_failure\n      end\n    end\n  end\n  it \"can be pushed\" do\n    it \"uses localization\" do\n    end\n  end\n  it \"can set a flag\" do\n    let(:context) do\n      LightService::Context.make(\n      )\n    end\n    it \"contains the aliases\" do\n    end\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&NestedGroups, source);
        for d in &diags {
            eprintln!(
                "  offense at line {} col {}: {}",
                d.location.line, d.location.column, d.message
            );
        }
        assert!(
            diags.is_empty(),
            "Max nesting of 3 should not exceed Max=3; got {} offenses",
            diags.len()
        );
    }

    #[test]
    fn rspec_shared_examples_as_top_level_group() {
        // RSpec.shared_examples (with RSpec receiver) should be recognized as a
        // shared group — its block should be walked but its nesting should start at 0.
        // 4 levels inside: describe > context > context > context = depth 4 > Max 3
        let source = b"RSpec.shared_examples 'reusable' do\n  describe 'feature' do\n    context 'a' do\n      context 'b' do\n        context 'c' do\n          it 'works' do\n          end\n        end\n      end\n    end\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&NestedGroups, source);
        assert_eq!(
            diags.len(),
            1,
            "RSpec.shared_examples should be walked; depth 4 > Max 3 should fire"
        );
    }

    #[test]
    fn rspec_feature_as_top_level_group() {
        // RSpec.feature (with RSpec receiver) should be recognized as an example group.
        // RSpec.feature > describe > context > context = depth 4 > Max 3
        let source = b"RSpec.feature 'something', type: :feature do\n  describe 'foo' do\n    context 'bar' do\n      context 'baz' do\n        it 'works' do\n        end\n      end\n    end\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&NestedGroups, source);
        assert_eq!(
            diags.len(),
            1,
            "RSpec.feature should be a top-level group; depth 4 > Max 3 should fire"
        );
    }

    #[test]
    fn sole_module_is_still_unwrapped() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        // With Max=1, the sole module wrapper should be unwrapped, allowing describe
        // to be detected as a top-level group. Then describe > context = depth 2 > Max 1.
        let config = CopConfig {
            options: HashMap::from([(
                "Max".into(),
                serde_yml::Value::Number(serde_yml::Number::from(1)),
            )]),
            ..CopConfig::default()
        };
        let source = b"module MyModule\n  describe Foo do\n    context 'bar' do\n      it 'works' do\n      end\n    end\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&NestedGroups, source, config);
        assert_eq!(
            diags.len(),
            1,
            "Sole module should be unwrapped — nested context should exceed Max=1"
        );
    }
}
