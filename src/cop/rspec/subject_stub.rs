use crate::cop::shared::method_dispatch_predicates;
use crate::cop::shared::util::{
    RSPEC_DEFAULT_INCLUDE, is_rspec_example_group, is_rspec_shared_group,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Flags stubbing methods on `subject`. The object under test should not be stubbed.
/// Detects: allow(subject_name).to receive(...), expect(subject_name).to receive(...)
///
/// Investigation notes (corpus FP=571→24→0, FN=27→18→0):
///
/// Round 1 FP fixes (571→24):
/// 1. Missing TopLevelGroup scoping: RuboCop's `TopLevelGroup#top_level_nodes` only
///    processes describe/context blocks at the file's top level. When `require "spec_helper"`
///    appears alongside a `module Foo` wrapper, the AST root is a `begin` node whose children
///    are `[require_call, module_node]`. The module is not a spec group, so it's skipped.
///    Our cop was processing ALL describe/context blocks regardless of nesting.
/// 2. Local variable reads: RuboCop's `(send nil? %)` pattern only matches method calls,
///    not local variable reads. When `subject = Foo.new` shadows the RSpec subject,
///    `allow(subject).to receive(...)` uses a local variable, not the subject method.
///    Our `extract_simple_name` was matching both CallNode and LocalVariableReadNode.
///
/// Round 2 FP fix (24→0):
/// 3. `let(:name)` redefining a subject name: When `let(:foo)` appears in the same or child
///    scope where `subject(:foo)` was defined, it shadows the subject. RuboCop's cop tracks
///    this via `let_definitions` and removes the name from the subject names list. All 24 FPs
///    were from this pattern (chef/chef 10, solidus 6, postal 3, truemail 2, etc.).
///
/// Round 2 FN fix (18→0):
/// 4. `do...end` blocks on receive chains followed by chain methods like `.at_least(:once)`,
///    `.twice`, `.and_return(...)`. Ruby's `do...end` binds to `.to`, making the outermost
///    AST node the chain method (e.g., `.at_least`), not `.to`. Fixed by walking the receiver
///    chain to find `.to`/`.not_to`/`.to_not` when the outermost call doesn't match.
/// 5. Explicit parens on `.to(receive(...))` followed by chain: `.to(receive(:bar)).and_return(baz)`
///    makes `.and_return` the outermost call. Same fix as #4.
///
/// Round 3 FP fix (3→?):
/// 6. Singleton method definitions (`def self.foo`): RuboCop's `find_subject_expectations`
///    recurses into `:def` child nodes but NOT `:defs` (singleton method definitions).
///    In Prism, both `def foo` and `def self.foo` are `DefNode`; the difference is
///    `def_node.receiver().is_some()`. Our code was recursing into all DefNodes, which
///    caused FPs when `allow(subject).to receive(...)` appeared inside `def self.cmds(...)`.
///    Fixed by skipping DefNode when `receiver().is_some()`.
///    Example: travis-ci/dpl `spec/dpl/ctx/bash_spec.rb` — `def self.cmds(cmds)` contains
///    `before { allow(bash).to receive(...)  }`.
///    FP=2 fix: ubicloud host_nexus_spec.rb:173,196 used Ruby 3.4 `it` keyword inside
///    `do...end` blocks on receive chains. RuboCop's parser gem produces `itblock` nodes
///    for these blocks, and `find_subject_expectations` only recurses into `:send, :def,
///    :block, :begin` — NOT `:itblock` or `:numblock`. In Prism, these are still
///    `BlockNode` but with `ItParametersNode`/`NumberedParametersNode` parameters.
///    Fixed by skipping block body recursion when parameters match these types.
///
/// Round 4 FP fix (4→0):
/// 6b. `let(:name)` inside shared groups (`shared_context`, `shared_examples`)
///     shadows parent subject names. RuboCop's `example_group?` matcher excludes
///     shared groups, so `find_all_explicit` associates `let` definitions inside
///     shared groups with the nearest ancestor example group (the parent), not the
///     shared group itself. In nitrocop, `is_rspec_example_group` includes shared
///     groups, so `let` names were scoped only to the shared group — not bubbling
///     up to shadow the parent's subject name. Fixed by collecting `let` names from
///     shared_group children during the first pass and applying them at the parent
///     scope level. All 4 alaveteli FPs were this pattern: `subject(:project)` at
///     parent level, `let(:project)` inside `shared_context 'project with resources'`,
///     and `allow(project).to receive(...)` in a sibling `describe` block.
///
/// Round 4 FN fix (1→0) [original round 4]:
/// 7. Subject stubs inside blocks on intermediate calls in a receiver chain: when
///    `expect(Thread).to receive(:new) do |&block| expect(subject).to receive(:method)
///    end.and_return(...)`, the `do...end` block is attached to `.to` but `.and_return`
///    is the outermost AST node. The code only checked blocks on the outermost call,
///    missing blocks on intermediate calls in the chain. Fixed by walking the receiver
///    chain and recursing into blocks on each intermediate call node.
///    Example: DataDog/dd-trace-rb configuration_spec.rb:612.
///
/// Round 5 FP/FN fix (17 FP, 5 FN):
/// 8. Shared groups (`shared_examples`, `shared_examples_for`, `shared_context`) do
///    not contribute named subjects for SubjectStub. RuboCop still flags
///    `allow(subject)`, but it does not treat `subject(:adapter)` in a shared group
///    as making `allow(adapter)` an offense in that shared group or nested describes.
///    Our code inherited named subjects from shared groups, causing the discourse
///    `spec/support/shared_examples/web_server_adapter.rb` false positives.
/// 9. Returning immediately after flagging an outer stub skipped nested stubs in the
///    same block, e.g. `expect(subject).to receive(:fork) do ... expect(subject).to
///    receive(:open!) ... end`. Fixed by continuing to recurse into the block after
///    reporting the outer offense.
///
/// Round 6 FN fix (1→0):
/// 10. Shared groups nested under a real example group DO contribute named subjects.
///     RuboCop associates `subject(:release)` inside nested `shared_examples` with the
///     nearest non-shared example-group ancestor, so `expect(release).to receive(...)`
///     is still an offense there. Top-level shared groups must continue to ignore named
///     subjects to preserve the `adapter` no-offense cases. Fixed by letting nested
///     shared groups inherit the parent scope's named-subject tracking flag.
pub struct SubjectStub;

impl Cop for SubjectStub {
    fn name(&self) -> &'static str {
        "RSpec/SubjectStub"
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let program = match parse_result.node().as_program_node() {
            Some(p) => p,
            None => return,
        };
        let body = program.statements();
        let stmts: Vec<_> = body.body().iter().collect();

        if stmts.len() == 1 {
            // Single top-level statement: unwrap module/class wrappers to find spec groups.
            self.find_top_level_groups(&stmts[0], source, diagnostics);
        } else {
            // Multiple top-level statements (e.g., `require "spec_helper"` + `describe Foo`):
            // Only check direct children for spec groups, do NOT unwrap modules/classes.
            // This matches RuboCop's TopLevelGroup `:begin` branch.
            for stmt in &stmts {
                self.check_direct_spec_group(stmt, source, diagnostics);
            }
        }
    }
}

impl SubjectStub {
    /// Check if a single node is a top-level spec group and process it.
    /// Does NOT recurse into module/class — used for the `:begin` case.
    fn check_direct_spec_group(
        &self,
        node: &ruby_prism::Node<'_>,
        source: &SourceFile,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if let Some(call) = node.as_call_node() {
            if let Some(block) = call.block() {
                if let Some(bn) = block.as_block_node() {
                    let name = call.name().as_slice();
                    let is_eg = call.receiver().is_none() && is_rspec_example_group(name);
                    let is_rspec_describe =
                        is_rspec_receiver(&call) && is_rspec_example_group(name);
                    if is_eg || is_rspec_describe {
                        let mut subject_names: Vec<Vec<u8>> = Vec::new();
                        subject_names.push(b"subject".to_vec());
                        let track_named_subjects = !is_rspec_shared_group(name);
                        collect_subject_stub_offenses(
                            source,
                            bn,
                            &mut subject_names,
                            track_named_subjects,
                            diagnostics,
                            self,
                        );
                    }
                }
            }
        }
    }

    /// Recursively find top-level spec groups, unwrapping module/class/begin nodes.
    /// Used when there's a single top-level construct.
    fn find_top_level_groups(
        &self,
        node: &ruby_prism::Node<'_>,
        source: &SourceFile,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Check if this node is a spec group call
        if let Some(call) = node.as_call_node() {
            if let Some(block) = call.block() {
                if let Some(bn) = block.as_block_node() {
                    let name = call.name().as_slice();
                    let is_eg = call.receiver().is_none() && is_rspec_example_group(name);
                    let is_rspec_describe =
                        is_rspec_receiver(&call) && is_rspec_example_group(name);
                    if is_eg || is_rspec_describe {
                        let mut subject_names: Vec<Vec<u8>> = Vec::new();
                        subject_names.push(b"subject".to_vec());
                        let track_named_subjects = !is_rspec_shared_group(name);
                        collect_subject_stub_offenses(
                            source,
                            bn,
                            &mut subject_names,
                            track_named_subjects,
                            diagnostics,
                            self,
                        );
                        return;
                    }
                }
            }
        }

        // Unwrap module nodes
        if let Some(module_node) = node.as_module_node() {
            if let Some(body) = module_node.body() {
                if let Some(stmts) = body.as_statements_node() {
                    for child in stmts.body().iter() {
                        self.find_top_level_groups(&child, source, diagnostics);
                    }
                }
            }
            return;
        }

        // Unwrap class nodes
        if let Some(class_node) = node.as_class_node() {
            if let Some(body) = class_node.body() {
                if let Some(stmts) = body.as_statements_node() {
                    for child in stmts.body().iter() {
                        self.find_top_level_groups(&child, source, diagnostics);
                    }
                }
            }
            return;
        }

        // Unwrap begin nodes
        if let Some(begin_node) = node.as_begin_node() {
            if let Some(stmts) = begin_node.statements() {
                for child in stmts.body().iter() {
                    self.find_top_level_groups(&child, source, diagnostics);
                }
            }
        }
    }
}

fn collect_subject_stub_offenses(
    source: &SourceFile,
    block: ruby_prism::BlockNode<'_>,
    subject_names: &mut Vec<Vec<u8>>,
    track_named_subjects: bool,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &SubjectStub,
) {
    let body = match block.body() {
        Some(b) => b,
        None => return,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return,
    };

    // First pass: collect subject names and let overrides defined in this scope
    let scope_start = subject_names.len();
    let mut let_names: Vec<Vec<u8>> = Vec::new();
    if track_named_subjects {
        for stmt in stmts.body().iter() {
            if let Some(call) = stmt.as_call_node() {
                let name = call.name().as_slice();
                if (name == b"subject" || name == b"subject!") && call.receiver().is_none() {
                    // Check if it has a name argument: subject(:foo)
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if !arg_list.is_empty() {
                            if let Some(sym) = arg_list[0].as_symbol_node() {
                                subject_names.push(sym.unescaped().to_vec());
                            }
                        }
                    }
                }
                // Track let(:name) definitions that shadow subject names
                if (name == b"let" || name == b"let!") && call.receiver().is_none() {
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if !arg_list.is_empty() {
                            if let Some(sym) = arg_list[0].as_symbol_node() {
                                let_names.push(sym.unescaped().to_vec());
                            }
                        }
                    }
                }
                // RuboCop's example_group? excludes shared groups (shared_context,
                // shared_examples, shared_examples_for). So let() definitions inside
                // shared groups are associated with the nearest ancestor example group,
                // not the shared group. Collect let names from shared group children
                // to apply them at this scope level.
                if is_rspec_shared_group(name) && call.receiver().is_none() {
                    if let Some(block_arg) = call.block() {
                        if let Some(bn) = block_arg.as_block_node() {
                            collect_let_names_from_shared_group(&bn, &mut let_names);
                        }
                    }
                }
            }
        }
    }

    // Remove subject names that are shadowed by let definitions in this scope
    if !let_names.is_empty() {
        subject_names.retain(|s| !let_names.contains(s));
        // Always keep implicit "subject" — let(:subject) is extremely rare and
        // unlikely to shadow the implicit subject
        if !subject_names.contains(&b"subject".to_vec()) {
            subject_names.push(b"subject".to_vec());
        }
    }

    // Second pass: check for stubs on subject names and recurse into nested groups
    for stmt in stmts.body().iter() {
        check_for_subject_stubs(
            source,
            &stmt,
            subject_names,
            track_named_subjects,
            diagnostics,
            cop,
        );
    }

    // Restore subject names for this scope (don't leak child-scope subjects to siblings)
    subject_names.truncate(scope_start);
    // But re-add the implicit "subject"
    if !subject_names.contains(&b"subject".to_vec()) {
        subject_names.push(b"subject".to_vec());
    }
}

fn check_for_subject_stubs(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    subject_names: &[Vec<u8>],
    track_named_subjects: bool,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &SubjectStub,
) {
    if let Some(call) = node.as_call_node() {
        // Check for allow(subject_name).to receive(...) or expect(subject_name).to receive(...)
        // Also handles chained calls after .to (e.g., .to(...).at_least(:once) or
        // .to(receive(...)).and_return(baz)) where .to is buried in the receiver chain.
        check_stub_expression(&call, node, source, subject_names, diagnostics, cop);

        // Recurse into nested blocks (before, it, context, etc.)
        // Check blocks on the outermost call AND on calls in the receiver chain.
        // This handles cases like: expect(Thread).to receive(:new) do |&block|
        //   expect(subject).to receive(:method)  # block is on .to, not .and_return
        // end.and_return(fake_thread)
        recurse_into_call_blocks(
            &call,
            source,
            subject_names,
            track_named_subjects,
            diagnostics,
            cop,
        );
    }

    // Check instance method def nodes for subject stubs too.
    // Skip singleton method definitions (def self.foo / def obj.foo) — RuboCop's
    // find_subject_expectations recurses into :def but not :defs (singleton defs).
    if let Some(def_node) = node.as_def_node() {
        if def_node.receiver().is_some() {
            return;
        }
        if let Some(body) = def_node.body() {
            if let Some(stmts) = body.as_statements_node() {
                for s in stmts.body().iter() {
                    check_for_subject_stubs(
                        source,
                        &s,
                        subject_names,
                        track_named_subjects,
                        diagnostics,
                        cop,
                    );
                }
            }
        }
    }
}

/// Recurse into blocks attached to a call node and any calls in its receiver chain.
/// This ensures we find subject stubs inside blocks on intermediate calls, e.g.:
///   expect(Thread).to receive(:new) do |&block|
///     expect(subject).to receive(:shutdown!)  # block is on .to, buried in receiver chain
///   end.and_return(fake_thread)
fn recurse_into_call_blocks(
    call: &ruby_prism::CallNode<'_>,
    source: &SourceFile,
    subject_names: &[Vec<u8>],
    track_named_subjects: bool,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &SubjectStub,
) {
    // Check block on this call
    if let Some(block) = call.block() {
        if let Some(bn) = block.as_block_node() {
            let call_name = call.name().as_slice();
            if is_rspec_example_group(call_name) {
                // Nested example group — create new scope with inherited subject names
                let mut child_names = subject_names.to_vec();
                let child_tracks_named_subjects =
                    !is_rspec_shared_group(call_name) || track_named_subjects;
                collect_subject_stub_offenses(
                    source,
                    bn,
                    &mut child_names,
                    child_tracks_named_subjects,
                    diagnostics,
                    cop,
                );
            } else {
                // Skip blocks with ItParametersNode or NumberedParametersNode —
                // RuboCop's parser gem produces `itblock`/`numblock` node types
                // for these, and find_subject_expectations doesn't traverse them.
                let skip_body = bn.parameters().is_some_and(|p| {
                    p.as_it_parameters_node().is_some() || p.as_numbered_parameters_node().is_some()
                });
                if !skip_body {
                    // Non-example-group block (before, it, specify, def, etc.)
                    if let Some(body) = bn.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            for s in stmts.body().iter() {
                                check_for_subject_stubs(
                                    source,
                                    &s,
                                    subject_names,
                                    track_named_subjects,
                                    diagnostics,
                                    cop,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // Also recurse into blocks on calls in the receiver chain.
    // This handles chained calls where a block is on an intermediate call
    // (e.g., .to has a do...end block, but .and_return is the outermost call).
    if let Some(recv) = call.receiver() {
        if let Some(recv_call) = recv.as_call_node() {
            recurse_into_call_blocks(
                &recv_call,
                source,
                subject_names,
                track_named_subjects,
                diagnostics,
                cop,
            );
        }
    }
}

/// Check if a call expression (possibly chained) is a subject stub.
/// Handles both direct `.to` calls and chained expressions where `.to` is
/// in the receiver chain (e.g., `.to(...).at_least(:once)` or do...end chains).
/// Returns true if an offense was reported.
fn check_stub_expression(
    call: &ruby_prism::CallNode<'_>,
    node: &ruby_prism::Node<'_>,
    source: &SourceFile,
    subject_names: &[Vec<u8>],
    diagnostics: &mut Vec<Diagnostic>,
    cop: &SubjectStub,
) -> bool {
    let method = call.name().as_slice();
    let is_to = method == b"to" || method == b"not_to" || method == b"to_not";

    // Skip .to calls wrapped in itblock/numblock — RuboCop's parser gem produces
    // opaque itblock/numblock nodes that hide the inner send from traversal.
    if is_to && has_itblock_or_numblock(call) {
        return false;
    }

    // If this is not a .to call, check if it's a chain after .to
    if !is_to && !has_to_in_receiver_chain(call) {
        return false;
    }

    // Check if the expression involves `receive` — check the entire call tree
    if !has_receive_in_tree(call) {
        return false;
    }

    // Extract the subject name from the allow/expect receiver of the .to call.
    // Returns Some(SubjectMatch) if this is a subject stub.
    let subject_match = extract_subject_from_chain(call, subject_names);
    match subject_match {
        SubjectMatch::IsExpected | SubjectMatch::NamedSubject => {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(cop.diagnostic(
                source,
                line,
                column,
                "Do not stub methods of the object under test.".to_string(),
            ));
            true
        }
        SubjectMatch::None => false,
    }
}

enum SubjectMatch {
    IsExpected,
    NamedSubject,
    None,
}

/// Walk the call chain to find .to/.not_to/.to_not and extract the subject info
/// from its allow/expect receiver.
fn extract_subject_from_chain(
    call: &ruby_prism::CallNode<'_>,
    subject_names: &[Vec<u8>],
) -> SubjectMatch {
    let method = call.name().as_slice();
    let is_to = method == b"to" || method == b"not_to" || method == b"to_not";

    if is_to {
        // Check receiver of .to for allow/expect(subject_name) or is_expected
        if let Some(recv) = call.receiver() {
            if let Some(recv_call) = recv.as_call_node() {
                let recv_method = recv_call.name().as_slice();
                if recv_method == b"is_expected" && recv_call.receiver().is_none() {
                    return SubjectMatch::IsExpected;
                }
                if (recv_method == b"allow" || recv_method == b"expect")
                    && recv_call.receiver().is_none()
                {
                    if let Some(args) = recv_call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if !arg_list.is_empty() {
                            let arg_name = extract_method_name(&arg_list[0]);
                            if let Some(name) = arg_name {
                                if subject_names.iter().any(|s| s == &name) {
                                    return SubjectMatch::NamedSubject;
                                }
                            }
                        }
                    }
                }
            }
        }
        return SubjectMatch::None;
    }

    // Not a .to call — walk receiver chain
    if let Some(recv) = call.receiver() {
        if let Some(recv_call) = recv.as_call_node() {
            return extract_subject_from_chain(&recv_call, subject_names);
        }
    }
    SubjectMatch::None
}

/// Check if the receiver chain contains a .to/.not_to/.to_not call.
/// Skips calls wrapped in itblock/numblock — RuboCop's parser gem makes these
/// opaque `itblock`/`numblock` nodes that hide the inner send from traversal.
fn has_to_in_receiver_chain(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(recv) = call.receiver() {
        if let Some(recv_call) = recv.as_call_node() {
            if has_itblock_or_numblock(&recv_call) {
                return false;
            }
            let name = recv_call.name().as_slice();
            if name == b"to" || name == b"not_to" || name == b"to_not" {
                return true;
            }
            return has_to_in_receiver_chain(&recv_call);
        }
    }
    false
}

/// Check if any node in the call tree contains a receive/receive_messages/receive_message_chain
/// or have_received call. Checks arguments and receiver chain recursively.
fn has_receive_in_tree(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    if (name == b"receive" || name == b"receive_messages" || name == b"receive_message_chain")
        && call.receiver().is_none()
    {
        return true;
    }
    if name == b"have_received" && call.receiver().is_none() {
        return true;
    }
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            if contains_receive_call(&arg) || contains_have_received_call(&arg) {
                return true;
            }
        }
    }
    if let Some(recv) = call.receiver() {
        if let Some(recv_call) = recv.as_call_node() {
            if has_receive_in_tree(&recv_call) {
                return true;
            }
        }
    }
    false
}

fn contains_receive_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if (name == b"receive" || name == b"receive_messages" || name == b"receive_message_chain")
            && call.receiver().is_none()
        {
            return true;
        }
        if let Some(recv) = call.receiver() {
            return contains_receive_call(&recv);
        }
        // Check arguments too (e.g., `all(receive(:baz))`)
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if contains_receive_call(&arg) {
                    return true;
                }
            }
        }
    }
    false
}

fn contains_have_received_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if method_dispatch_predicates::is_command(&call, b"have_received") {
            return true;
        }
        if let Some(recv) = call.receiver() {
            return contains_have_received_call(&recv);
        }
    }
    false
}

/// Collect let(:name) definitions from inside a shared group block.
/// RuboCop's `example_group?` excludes shared groups, so `find_all_explicit`
/// associates `let` definitions inside shared groups with the parent example
/// group (the nearest ancestor that IS an example_group?). This function
/// recursively collects let names from a shared group's body and any nested
/// shared groups within it, so they can be applied at the parent scope level.
fn collect_let_names_from_shared_group(
    block: &ruby_prism::BlockNode<'_>,
    let_names: &mut Vec<Vec<u8>>,
) {
    let body = match block.body() {
        Some(b) => b,
        None => return,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return,
    };
    for stmt in stmts.body().iter() {
        if let Some(call) = stmt.as_call_node() {
            let name = call.name().as_slice();
            if (name == b"let" || name == b"let!") && call.receiver().is_none() {
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if !arg_list.is_empty() {
                        if let Some(sym) = arg_list[0].as_symbol_node() {
                            let_names.push(sym.unescaped().to_vec());
                        }
                    }
                }
            }
            // Recurse into nested shared groups (they also don't count as example_group?)
            if is_rspec_shared_group(name) && call.receiver().is_none() {
                if let Some(block_arg) = call.block() {
                    if let Some(bn) = block_arg.as_block_node() {
                        collect_let_names_from_shared_group(&bn, let_names);
                    }
                }
            }
        }
    }
}

/// Extract the name of a receiverless method call. Only matches `CallNode` with
/// no receiver and no arguments (i.e., `(send nil? :name)` in RuboCop terms).
/// Does NOT match local variable reads — RuboCop's message_expectation? matcher
/// uses `(send nil? %)` which only matches method sends, not lvar reads.
fn extract_method_name(node: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none() && call.arguments().is_none() {
            return Some(call.name().as_slice().to_vec());
        }
    }
    None
}

/// Check if a call node has a block with ItParametersNode or NumberedParametersNode.
/// These correspond to RuboCop's `itblock`/`numblock` AST node types which are
/// opaque to the cop's `find_subject_expectations` traversal.
fn has_itblock_or_numblock(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(block) = call.block() {
        if let Some(bn) = block.as_block_node() {
            if let Some(params) = bn.parameters() {
                return params.as_it_parameters_node().is_some()
                    || params.as_numbered_parameters_node().is_some();
            }
        }
    }
    false
}

/// Check if the receiver of a CallNode is `RSpec` (simple constant) or `::RSpec`.
fn is_rspec_receiver(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(recv) = call.receiver() {
        if let Some(cr) = recv.as_constant_read_node() {
            return cr.name().as_slice() == b"RSpec";
        }
        if let Some(cp) = recv.as_constant_path_node() {
            if cp.parent().is_none() {
                if let Some(name) = cp.name() {
                    return name.as_slice() == b"RSpec";
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SubjectStub, "cops/rspec/subject_stub");
}
