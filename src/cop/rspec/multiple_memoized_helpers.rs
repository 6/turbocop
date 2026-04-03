use std::collections::HashSet;

use crate::cop::shared::util::{
    self, RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group, is_rspec_let,
    is_rspec_shared_group,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks if example groups contain too many `let` and `subject` calls.
///
/// ## Root cause of FNs (fixed, round 1)
///
/// The original implementation only looked at direct statements in the block body
/// (`collect_direct_helper_names`). Helpers nested inside control structures
/// (if/unless/case/begin/rescue) were missed. RuboCop's `ExampleGroup.find_all_in_scope()`
/// recursively walks the entire subtree, only stopping at scope boundaries (other
/// example groups, shared groups) and examples (it, specify, etc.). The fix replaces
/// the flat scan with a recursive depth-first walker that matches RuboCop's behavior.
///
/// ## Root cause of FPs (fixed, round 2)
///
/// 1. **Missing `Includes` scope boundaries**: RuboCop's `ExampleGroup.scope_change?`
///    stops at `it_behaves_like`, `it_should_behave_like`, `include_examples`, and
///    `include_context` blocks. nitrocop's `HelperCollector` was NOT stopping at these,
///    causing helpers defined inside include blocks to be counted toward the outer
///    group's total. Fix: added `is_rspec_include()` check to scope boundary detection.
///
/// 2. **Missing block requirement for let/subject**: RuboCop's `let?` pattern requires
///    a block (`(block (send ...))`) or block-pass (`(send ... block_pass)`). nitrocop
///    was counting bare `let(:foo)` calls without blocks. Fix: added `node.block().is_some()`
///    check before collecting helper names.
///
/// 3. **String argument support**: RuboCop's `variable_definition?` extracts names from
///    `{any_sym str dstr}` (symbols, strings, dynamic strings). nitrocop only handled
///    symbols. Fix: added `extract_name_from_arg()` that handles all three forms.
///
/// ## Root cause of FNs (fixed, round 3)
///
/// **Missing `RSpec.` receiver support for non-describe methods**: RuboCop's `spec_group?`
/// matches `(any_block (send #rspec? {#SharedGroups.all #ExampleGroups.all} ...) ...)` where
/// `#rspec?` accepts both `nil?` (no receiver) and `(const cbase :RSpec)`. nitrocop's
/// `is_example_group_call` only matched `describe` when the receiver was `RSpec`, missing
/// `RSpec.shared_context`, `RSpec.shared_examples`, `RSpec.context`, etc. This pattern is
/// very common in `spec/support/` and `spec/shared/` files (e.g., `RSpec.shared_context
/// 'movie class' do ... end`). Fix: `is_example_group_call` now checks all example group
/// and shared group methods when receiver is `RSpec`.
///
/// The same bug existed in `HelperCollector`'s scope boundary check — `RSpec.shared_context`
/// blocks were not treated as scope boundaries, causing helpers inside them to leak to the
/// parent group (producing FPs). Fix: scope boundary check now also accepts all group methods
/// with `RSpec` receiver.
///
/// ## `let (:foo)` with space before paren — intentionally not unwrapped
///
/// When a method call has a space before the argument parentheses (`let (:foo) { }` instead
/// of `let(:foo) { }`), Prism wraps the argument in a `ParenthesesNode` (in Parser gem, a
/// `(begin ...)` node). RuboCop's `variable_definition?` NodePattern only matches bare
/// `{any_sym str dstr}`, so `let (:foo)` returns `nil` from name extraction. RuboCop then
/// counts all such nil values as ONE collective helper via `.uniq.count`.
///
/// We match this behavior: `extract_name_from_arg` returns `None` for `ParenthesesNode`,
/// and the `HelperCollector` inserts a fixed sentinel value so all space-paren lets collapse
/// to a single entry. This means `let (:a)` + `let (:b)` + `let (:c)` count as 1 helper,
/// not 3.
///
/// **History:** Round 4 originally unwrapped `ParenthesesNode` to individually count these
/// calls, but this caused 55 FPs (puppet: 36, openproject: 13, dragonfly: 3, omnibus: 2,
/// ifme: 1) because nitrocop overcounted relative to RuboCop. Reverted to match RuboCop's
/// nil-dedup behavior.
///
/// ## Root cause of FNs (fixed, round 5)
///
/// **Missing helper inheritance from non-spec-group ancestor blocks**: RuboCop's
/// `all_helpers` uses `node.each_ancestor(:block)` which walks ALL ancestor block nodes,
/// not just spec groups. It calls `helpers(ancestor)` on each, which finds `let/subject`
/// calls via `ExampleGroup.new(ancestor).lets`. This means helpers defined in custom
/// wrapper methods (like karafka's `RSpec.describe_current`) propagate to nested spec
/// groups even though the wrapper itself is not a recognized example group.
///
/// nitrocop's `MemoizedHelperVisitor` only pushed helpers onto the ancestor stack for
/// recognized spec groups (`is_example_group_call`). Non-spec-group blocks like
/// `describe_current do...end` or `my_helper do...end` were visited via pass-through
/// recursion, so their `let` calls were never added to the ancestor context. This caused
/// 722 FNs across 3 corpus repos (karafka: 522, openproject: 139,
/// rspec_api_documentation: 61).
///
/// Fix: the visitor now collects helpers and pushes them onto the ancestor stack for ALL
/// call nodes with blocks, not just spec groups. Only spec groups emit diagnostics.
/// The push/pop ensures helpers are scoped to descendants only (no sibling leakage).
pub struct MultipleMemoizedHelpers;

impl Cop for MultipleMemoizedHelpers {
    fn name(&self) -> &'static str {
        "RSpec/MultipleMemoizedHelpers"
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
        let max = config.get_usize("Max", 5);
        let allow_subject = config.get_bool("AllowSubject", true);

        let mut visitor = MemoizedHelperVisitor {
            cop: self,
            source,
            max,
            allow_subject,
            // Stack of ancestor helper name sets (each entry is the set of names for that group)
            ancestor_names: Vec::new(),
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MemoizedHelperVisitor<'a> {
    cop: &'a MultipleMemoizedHelpers,
    source: &'a SourceFile,
    max: usize,
    allow_subject: bool,
    /// Stack of helper name sets for each ancestor example group.
    /// Each entry contains the names defined directly in that group.
    ancestor_names: Vec<HashSet<Vec<u8>>>,
    diagnostics: Vec<Diagnostic>,
}

/// Extract the variable name from the first argument node.
/// Handles symbol (`:foo`), string (`"foo"`), and dynamic string (`"foo#{bar}"`) forms,
/// matching RuboCop's `variable_definition?` pattern: `$({any_sym str dstr} ...)`.
///
/// **Important:** Does NOT unwrap `ParenthesesNode`. When there is a space before the
/// argument paren (`let (:foo) { }` vs `let(:foo) { }`), Prism wraps the argument in a
/// `ParenthesesNode`. In the Parser gem, this produces `(begin (sym :foo))` instead of
/// bare `(sym :foo)`. RuboCop's `variable_definition?` NodePattern only matches bare
/// `{any_sym str dstr}`, so `let (:foo)` returns `nil` from `variable_definition?`.
/// RuboCop then counts all such nil values as ONE collective helper via `.uniq.count`.
/// We match this behavior by returning `None` here, and the caller inserts a fixed
/// sentinel value so all space-paren lets collapse to a single entry.
fn extract_name_from_arg(arg: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
    if let Some(sym) = arg.as_symbol_node() {
        return Some(sym.unescaped().to_vec());
    }
    if let Some(s) = arg.as_string_node() {
        return Some(s.unescaped().to_vec());
    }
    if let Some(ds) = arg.as_interpolated_string_node() {
        // For dynamic strings, use the raw source as the name (can't evaluate at lint time)
        let loc = ds.location();
        return Some(loc.as_slice().to_vec());
    }
    // ParenthesesNode (from `let (:foo)` with space) is intentionally NOT unwrapped.
    // See doc comment above for rationale.
    None
}

/// Extract the helper name from a let/let!/subject/subject! call.
/// For `let(:foo) { ... }` or `let("foo") { ... }`, returns "foo".
/// For `subject(:bar) { ... }`, returns "bar".
/// For bare `subject { ... }`, returns "subject".
fn extract_helper_name(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let method_name = call.name().as_slice();

    // For subject/subject! without args, the name is "subject"
    if util::is_rspec_subject(method_name) {
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if let Some(first) = arg_list.first() {
                if let Some(name) = extract_name_from_arg(first) {
                    return Some(name);
                }
            }
        }
        // Bare subject/subject! — use "subject" as the name
        return Some(b"subject".to_vec());
    }

    // For let/let!, extract the name from the first argument
    if is_rspec_let(method_name) {
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if let Some(first) = arg_list.first() {
                return extract_name_from_arg(first);
            }
        }
    }

    None
}

/// RSpec include methods that act as scope boundaries.
/// Matches RuboCop's `Includes.all`: `it_behaves_like`, `it_should_behave_like`,
/// `include_examples`, `include_context`.
fn is_rspec_include(name: &[u8]) -> bool {
    matches!(
        name,
        b"it_behaves_like" | b"it_should_behave_like" | b"include_examples" | b"include_context"
    )
}

/// Inner visitor that recursively collects helper names within a scope.
///
/// Matches RuboCop's `ExampleGroup.find_all_in_scope()` behavior:
/// - Traverses the entire subtree using the Visit trait
/// - Collects all let/let!/subject/subject! calls found anywhere
/// - Stops recursion at scope boundaries (other example groups, shared groups, includes)
/// - Stops recursion at examples (it, specify, etc.)
struct HelperCollector {
    allow_subject: bool,
    names: HashSet<Vec<u8>>,
}

impl<'pr> Visit<'pr> for HelperCollector {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();
        let has_block = node.block().is_some_and(|b| b.as_block_node().is_some());

        // Stop at scope boundaries: example groups, shared groups, and includes with blocks.
        // Matches RuboCop's ExampleGroup.scope_change? which stops at:
        //   (block (send #rspec? {#SharedGroups.all #ExampleGroups.all} ...) ...)
        //   (block (send nil? #Includes.all ...) ...)
        if has_block {
            let is_scope_boundary = if let Some(recv) = node.receiver() {
                util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                    && (is_rspec_example_group(method_name) || is_rspec_shared_group(method_name))
            } else {
                is_rspec_example_group(method_name)
                    || is_rspec_shared_group(method_name)
                    || is_rspec_include(method_name)
            };
            if is_scope_boundary {
                return;
            }
        }

        // Stop at examples (it, specify, etc.) — helpers inside examples don't count
        if node.receiver().is_none() && is_rspec_example(method_name) {
            return;
        }

        // Collect helper names from let/let!/subject/subject! calls.
        // Only count calls that have a block or block-pass (matching RuboCop's `let?` pattern).
        if node.receiver().is_none()
            && node.block().is_some()
            && (is_rspec_let(method_name)
                || (!self.allow_subject && util::is_rspec_subject(method_name)))
        {
            if let Some(name) = extract_helper_name(node) {
                self.names.insert(name);
            } else {
                // When the name can't be extracted (e.g., `let (:foo)` with a space before
                // paren — Prism wraps the arg in ParenthesesNode), insert a fixed sentinel.
                // This matches RuboCop's behavior: `variable_definition?` returns nil for
                // these cases, and `.uniq.count` collapses all nils into ONE entry.
                self.names.insert(b"__unknown_variable__".to_vec());
            }
        }

        // Continue recursing into children
        ruby_prism::visit_call_node(self, node);
    }
}

impl<'a> MemoizedHelperVisitor<'a> {
    /// Check if a call node is a spec group (example group or shared group).
    /// Matches RuboCop's `spec_group?`:
    ///   (any_block (send #rspec? {#SharedGroups.all #ExampleGroups.all} ...) ...)
    /// where `#rspec?` matches both `nil?` (no receiver) and `(const cbase :RSpec)`.
    fn is_example_group_call(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        let method_name = call.name().as_slice();
        if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                && (is_rspec_example_group(method_name) || is_rspec_shared_group(method_name))
        } else {
            is_rspec_example_group(method_name)
        }
    }

    /// Collect all helper names within a block's scope using recursive depth-first search.
    fn collect_helper_names_in_scope(&self, block: &ruby_prism::BlockNode<'_>) -> HashSet<Vec<u8>> {
        let mut collector = HelperCollector {
            allow_subject: self.allow_subject,
            names: HashSet::new(),
        };
        if let Some(body) = block.body() {
            collector.visit(&body);
        }
        collector.names
    }
}

impl<'pr> Visit<'pr> for MemoizedHelperVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let is_spec_group = self.is_example_group_call(node);

        let block = match node.block() {
            Some(b) => b.as_block_node(),
            None => None,
        };

        // If there's no block, just recurse into children
        let Some(block) = block else {
            ruby_prism::visit_call_node(self, node);
            return;
        };

        // Collect helper names in this block's scope (recursive walk).
        // This is done for ALL blocks, not just spec groups, matching RuboCop's
        // `each_ancestor(:block)` behavior: helpers from non-spec-group ancestor
        // blocks (e.g., `RSpec.describe_current`, custom wrappers) are included
        // when counting a nested spec group's total helpers.
        let direct_names = self.collect_helper_names_in_scope(&block);

        // Only report offenses on spec groups (example groups / shared groups)
        if is_spec_group {
            // Total = union of all ancestor names + this group's names
            // Overrides (same name in child) don't increase the count.
            let mut all_names: HashSet<Vec<u8>> = HashSet::new();
            for ancestor_set in &self.ancestor_names {
                for name in ancestor_set {
                    all_names.insert(name.clone());
                }
            }
            for name in &direct_names {
                all_names.insert(name.clone());
            }
            let total = all_names.len();

            if total > self.max {
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!(
                        "Example group has too many memoized helpers [{total}/{}]",
                        self.max
                    ),
                ));
            }
        }

        // Push this block's direct names onto the ancestor stack and recurse.
        // Done for ALL blocks so nested spec groups inherit helpers from
        // non-spec-group ancestor blocks.
        self.ancestor_names.push(direct_names);
        ruby_prism::visit_call_node(self, node);
        self.ancestor_names.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        MultipleMemoizedHelpers,
        "cops/rspec/multiple_memoized_helpers"
    );

    #[test]
    fn allow_subject_false_counts_subject() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("AllowSubject".into(), serde_yml::Value::Bool(false)),
                (
                    "Max".into(),
                    serde_yml::Value::Number(serde_yml::Number::from(2)),
                ),
            ]),
            ..CopConfig::default()
        };
        // 2 lets + 1 subject = 3 helpers, max is 2
        let source =
            b"describe Foo do\n  subject(:bar) { 1 }\n  let(:a) { 1 }\n  let(:b) { 2 }\nend\n";
        let diags =
            crate::testutil::run_cop_full_with_config(&MultipleMemoizedHelpers, source, config);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn allow_subject_true_does_not_count_subject() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("AllowSubject".into(), serde_yml::Value::Bool(true)),
                (
                    "Max".into(),
                    serde_yml::Value::Number(serde_yml::Number::from(2)),
                ),
            ]),
            ..CopConfig::default()
        };
        // 2 lets + 1 subject = 2 counted helpers (subject excluded), max is 2
        let source =
            b"describe Foo do\n  subject(:bar) { 1 }\n  let(:a) { 1 }\n  let(:b) { 2 }\nend\n";
        let diags =
            crate::testutil::run_cop_full_with_config(&MultipleMemoizedHelpers, source, config);
        assert!(diags.is_empty());
    }

    #[test]
    fn nested_context_inherits_parent_lets() {
        // Parent has 4 lets, nested context has 2 lets = 6 total, exceeds max of 5
        let source = b"describe Foo do\n  let(:a) { 1 }\n  let(:b) { 2 }\n  let(:c) { 3 }\n  let(:d) { 4 }\n\n  context 'nested' do\n    let(:e) { 5 }\n    let(:f) { 6 }\n    it { expect(true).to be true }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        // The nested context should fire because 4 + 2 = 6 > 5
        // The parent describe should NOT fire (4 <= 5)
        assert_eq!(
            diags.len(),
            1,
            "Should fire on nested context with 6 total helpers"
        );
        assert!(diags[0].message.contains("[6/5]"));
    }

    #[test]
    fn overriding_lets_in_child_do_not_increase_count() {
        // Parent has 5 lets at the limit. Child overrides 2 of them.
        // Total unique names = 5 (not 7), so no offense.
        let source = b"describe Foo do\n  let(:a) { 1 }\n  let(:b) { 2 }\n  let(:c) { 3 }\n  let(:d) { 4 }\n  let(:e) { 5 }\n\n  context 'overrides' do\n    let(:a) { 10 }\n    let(:b) { 20 }\n    it { expect(a).to eq(10) }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        assert!(
            diags.is_empty(),
            "Overriding lets should not increase count: {:?}",
            diags
        );
    }

    #[test]
    fn helpers_nested_in_if_are_counted() {
        // 3 direct lets + 3 inside if = 6, exceeds max of 5
        let source = b"describe Foo do\n  let(:a) { 1 }\n  let(:b) { 2 }\n  let(:c) { 3 }\n\n  if ENV['CI']\n    let(:d) { 4 }\n    let(:e) { 5 }\n    let(:f) { 6 }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        assert_eq!(
            diags.len(),
            1,
            "Should fire when helpers are nested in if: {:?}",
            diags
        );
        assert!(diags[0].message.contains("[6/5]"));
    }

    #[test]
    fn shared_examples_are_detected() {
        let source = b"shared_examples 'too many helpers' do\n  let(:a) { 1 }\n  let(:b) { 2 }\n  let(:c) { 3 }\n  let(:d) { 4 }\n  let(:e) { 5 }\n  let(:f) { 6 }\n  it { expect(a).to eq(1) }\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        assert_eq!(
            diags.len(),
            1,
            "Should fire on shared_examples with 6 helpers: {:?}",
            diags
        );
        assert!(diags[0].message.contains("[6/5]"));
    }

    #[test]
    fn block_pass_form_is_counted() {
        let source = b"describe Foo do\n  let(:a, &method(:something_a))\n  let(:b, &method(:something_b))\n  let(:c, &method(:something_c))\n  let(:d, &method(:something_d))\n  let(:e, &method(:something_e))\n  let(:f, &method(:something_f))\n  it { expect(a).to eq(1) }\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        assert_eq!(
            diags.len(),
            1,
            "Should fire on block-pass lets with 6 helpers: {:?}",
            diags
        );
        assert!(diags[0].message.contains("[6/5]"));
    }

    #[test]
    fn rspec_shared_context_is_detected() {
        // RSpec.shared_context (with receiver) should be treated as a spec group
        let source = b"RSpec.shared_context 'movie class' do\n  let(:a) { 1 }\n  let(:b) { 2 }\n  let(:c) { 3 }\n  let(:d) { 4 }\n  let(:e) { 5 }\n  let(:f) { 6 }\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        assert_eq!(
            diags.len(),
            1,
            "Should fire on RSpec.shared_context with 6 helpers: {:?}",
            diags
        );
        assert!(diags[0].message.contains("[6/5]"));
    }

    #[test]
    fn rspec_shared_examples_is_detected() {
        // RSpec.shared_examples (with receiver) should be treated as a spec group
        let source = b"RSpec.shared_examples 'helpers' do\n  let(:a) { 1 }\n  let(:b) { 2 }\n  let(:c) { 3 }\n  let(:d) { 4 }\n  let(:e) { 5 }\n  let(:f) { 6 }\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        assert_eq!(
            diags.len(),
            1,
            "Should fire on RSpec.shared_examples with 6 helpers: {:?}",
            diags
        );
        assert!(diags[0].message.contains("[6/5]"));
    }

    #[test]
    fn rspec_context_is_detected() {
        // RSpec.context (with receiver) should be treated as a spec group
        let source = b"RSpec.context 'something' do\n  let(:a) { 1 }\n  let(:b) { 2 }\n  let(:c) { 3 }\n  let(:d) { 4 }\n  let(:e) { 5 }\n  let(:f) { 6 }\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        assert_eq!(
            diags.len(),
            1,
            "Should fire on RSpec.context with 6 helpers: {:?}",
            diags
        );
        assert!(diags[0].message.contains("[6/5]"));
    }

    #[test]
    fn rspec_shared_context_is_scope_boundary() {
        // RSpec.shared_context is a scope boundary: its helpers don't count toward the parent.
        // Parent has 5 lets at the limit. Inner shared_context has 2 helpers, but they
        // don't leak to the parent. However, the inner shared_context inherits from parent
        // (5 + 2 = 7 > 5), so it fires on the shared_context itself.
        let source = b"describe Foo do\n  let(:a) { 1 }\n  let(:b) { 2 }\n  let(:c) { 3 }\n  let(:d) { 4 }\n  let(:e) { 5 }\n\n  RSpec.shared_context 'inner' do\n    let(:f) { 6 }\n    let(:g) { 7 }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        // Parent has 5 lets (at limit, no offense on parent)
        // Inner RSpec.shared_context inherits 5 + 2 own = 7 > 5, offense on inner
        assert_eq!(
            diags.len(),
            1,
            "Should fire on inner RSpec.shared_context, not on parent: {:?}",
            diags
        );
        assert!(diags[0].message.contains("[7/5]"));
    }

    #[test]
    fn let_with_space_before_paren_counts_as_one() {
        // `let (:foo)` (space before paren) is treated differently by RuboCop:
        // Parser gem wraps the arg in `(begin ...)`, so `variable_definition?` returns nil.
        // All such nil values collapse to ONE via `.uniq.count`.
        // Here: 6 space-paren lets = 1 collective helper ≤ 5. No offense.
        let source = b"describe Foo do\n  let (:a) { 1 }\n  let (:b) { 2 }\n  let (:c) { 3 }\n  let (:d) { 4 }\n  let (:e) { 5 }\n  let (:f) { 6 }\n  it { expect(true).to be true }\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        assert!(
            diags.is_empty(),
            "Space-paren lets should collapse to 1 helper (matching RuboCop): {:?}",
            diags
        );
    }

    #[test]
    fn let_with_space_mixed_with_normal_lets() {
        // 5 normal lets + 2 space-paren lets = 5 + 1 (nil group) = 6 > 5. Offense.
        let source = b"describe Foo do\n  let(:a) { 1 }\n  let(:b) { 2 }\n  let(:c) { 3 }\n  let(:d) { 4 }\n  let(:e) { 5 }\n  let (:f) { 6 }\n  let (:g) { 7 }\n  it { expect(true).to be true }\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        assert_eq!(
            diags.len(),
            1,
            "5 named + 1 nil group = 6 > 5, should fire: {:?}",
            diags
        );
        assert!(diags[0].message.contains("[6/5]"));
    }

    #[test]
    fn helpers_nested_in_begin_rescue_are_counted() {
        // 6 lets inside begin/rescue = 6, exceeds max of 5
        let source = b"describe Foo do\n  begin\n    let(:a) { 1 }\n    let(:b) { 2 }\n    let(:c) { 3 }\n    let(:d) { 4 }\n    let(:e) { 5 }\n    let(:f) { 6 }\n  rescue StandardError\n    nil\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        assert_eq!(
            diags.len(),
            1,
            "Should fire when helpers are in begin/rescue: {:?}",
            diags
        );
        assert!(diags[0].message.contains("[6/5]"));
    }

    #[test]
    fn helpers_in_non_spec_group_ancestor_block_are_inherited() {
        // Custom wrapper method (like `describe_current`) is not a recognized spec group,
        // but its `let` calls should still count toward nested spec groups.
        // RuboCop's `each_ancestor(:block)` walks ALL ancestor blocks, not just spec groups.
        // Here: outer wrapper has 3 lets, inner context has 3 lets = 6 total > 5.
        let source = b"RSpec.describe_current do\n  let(:a) { 1 }\n  let(:b) { 2 }\n  let(:c) { 3 }\n\n  context 'nested' do\n    let(:d) { 4 }\n    let(:e) { 5 }\n    let(:f) { 6 }\n    it { expect(true).to be true }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        assert_eq!(
            diags.len(),
            1,
            "Should fire on nested context inheriting from non-spec-group ancestor: {:?}",
            diags
        );
        assert!(diags[0].message.contains("[6/5]"));
    }

    #[test]
    fn non_spec_group_ancestor_does_not_fire_itself() {
        // The non-spec-group wrapper should NOT report an offense itself,
        // only spec groups inside it should.
        let source = b"custom_wrapper do\n  let(:a) { 1 }\n  let(:b) { 2 }\n  let(:c) { 3 }\n  let(:d) { 4 }\n  let(:e) { 5 }\n  let(:f) { 6 }\n  it { expect(true).to be true }\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        assert!(
            diags.is_empty(),
            "Non-spec-group wrapper should not fire: {:?}",
            diags
        );
    }

    #[test]
    fn helpers_from_sibling_blocks_do_not_leak() {
        // Helpers from sibling blocks should NOT leak to each other.
        let source = b"custom_wrapper do\n  let(:a) { 1 }\n  let(:b) { 2 }\n  let(:c) { 3 }\nend\n\ndescribe Bar do\n  let(:d) { 4 }\n  let(:e) { 5 }\n  it { expect(true).to be true }\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        assert!(
            diags.is_empty(),
            "Sibling block helpers should not leak: {:?}",
            diags
        );
    }
}
