use std::collections::HashMap;
use std::collections::hash_map::Entry;

use ruby_prism::Visit;

use crate::cop::shared::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DuplicatedGem;

/// ## Corpus investigation (2026-03-20)
///
/// ### Round 8 — Extended corpus FP=0, FN=1
///
/// **FN=1**: `gem 'rom' do...end` in if branch (rom-rb/rom-rails). In Parser gem,
/// `gem 'x' do...end` is `(block (send ...) ...)` — the send node's first non-begin
/// ancestor is `:block`, not `:if`. So `conditional_declaration?` returns false and
/// all duplicates are flagged. Fixed by incrementing `blocks_above_conditional` when
/// the CallNode has a block child (`node.block().is_some()`).
///
/// ### Round 7 — Standard corpus FP=30
///
/// **FP=30**: All 30 FPs from gems in multi-branch if/elsif/else conditionals
/// (3+ branches). Root cause: `collect_if_matchable_sources` only processed one
/// level of the elsif chain (if_body + first elsif's child_nodes), missing bodies
/// of subsequent elsifs. RuboCop's `IfNode#branches` iterates the ENTIRE chain,
/// collecting each body separately. For multi-statement bodies (begin nodes),
/// `branch.child_nodes.include?(node)` finds individual statements. Fixed by
/// rewriting to iterate the full chain, also handling `else { single if }` which
/// Parser gem merges into the elsif chain (identical AST to `elsif`).
///
/// Round 6 was incorrect: it treated multi-statement elsif bodies as non-matchable
/// (begin.child_nodes of the IfNode, not the branch body). The key insight is that
/// `branches` returns the BODY of each branch, not the IfNode itself.
///
/// ### Round 5 — Extended corpus FP=3, FN=7
///
/// **FP=3**: case/when/else with nested if/else inside else. Fixed by per-gem
/// source matching against branch members.
///
/// **FN=7**: Two root causes: (1) gems inside blocks within conditionals were
/// exempted, (2) modifier if/unless treated as transparent.
///
/// ### Round 4 — FP=0, FN=0 (standard corpus)
///
/// Fixed the Autolab FN: structural equality used `any_conditional` but RuboCop
/// checks `nodes[0]`'s ancestor first.
impl Cop for DuplicatedGem {
    fn name(&self) -> &'static str {
        "Bundler/DuplicatedGem"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemfile", "**/Gemfile", "**/gems.rb"]
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
        let mut visitor = GemDeclarationVisitor {
            source,
            declarations: Vec::new(),
            ancestors: Vec::new(),
            next_conditional_root_id: 1,
            pending_elsif_root: None,
            root_matchable_sources: HashMap::new(),
        };
        visitor.visit(&parse_result.node());

        let mut grouped: HashMap<Vec<u8>, Vec<GemDeclaration>> = HashMap::new();
        for declaration in visitor.declarations {
            match grouped.entry(declaration.gem_name.clone()) {
                Entry::Occupied(mut occupied) => occupied.get_mut().push(declaration),
                Entry::Vacant(vacant) => {
                    vacant.insert(vec![declaration]);
                }
            }
        }

        for declarations in grouped.into_values() {
            if declarations.len() < 2 {
                continue;
            }

            let first = &declarations[0];

            // RuboCop's `conditional_declaration?` requires that nodes[0]'s first
            // non-begin ancestor is an `:if` or `:when` node. In Prism terms, this
            // means the first gem must have blocks_above_conditional == 0 and be
            // inside a conditional root. Gems inside blocks (path/git/source/group)
            // have blocks_above > 0 and are NOT considered conditional.
            let first_root = match first.conditional_root {
                Some(root) if first.blocks_above_conditional == 0 => root,
                _ => {
                    // First gem is not directly in a conditional — flag all duplicates.
                    let gem_name = String::from_utf8_lossy(&first.gem_name);
                    for duplicate in declarations.iter().skip(1) {
                        diagnostics.push(self.diagnostic(
                            source,
                            duplicate.line,
                            duplicate.column,
                            format!(
                                "Gem `{}` requirements already given on line {} of the Gemfile.",
                                gem_name, first.line
                            ),
                        ));
                    }
                    continue;
                }
            };

            // RuboCop's `within_conditional?` checks each gem against the root
            // conditional's branches: `branch == node || branch.child_nodes.include?(node)`
            // using structural equality. This only goes ONE level deep:
            // - For `if/elsif`: branches = [if_body, elsif_IfNode]. The elsif's
            //   child_nodes include [predicate, if_body, else_body] — single-statement
            //   bodies are direct children, multi-statement bodies are begin-wrapped
            //   and their individual statements are NOT direct children.
            // - For `case/when`: each when body and else body is a branch, and
            //   begin.child_nodes includes all statements.
            let all_within_conditional =
                if let Some(matchable) = visitor.root_matchable_sources.get(&first_root) {
                    declarations
                        .iter()
                        .all(|decl| matchable.iter().any(|s| s == &decl.call_source))
                } else {
                    false
                };

            if all_within_conditional {
                continue;
            }

            let gem_name = String::from_utf8_lossy(&first.gem_name);
            for duplicate in declarations.iter().skip(1) {
                diagnostics.push(self.diagnostic(
                    source,
                    duplicate.line,
                    duplicate.column,
                    format!(
                        "Gem `{}` requirements already given on line {} of the Gemfile.",
                        gem_name, first.line
                    ),
                ));
            }
        }
    }
}

#[derive(Clone, Copy)]
enum AncestorKind {
    /// Opaque block — breaks direct-child relationship for conditional exemption.
    /// Used for CallNode, BlockNode with multi-statement body, and similar.
    Block,
    /// Transparent wrapper — does not break the conditional ancestor chain.
    /// Used for StatementsNode, BeginNode, ElseNode, ProgramNode, single-stmt BlockNode.
    BeginLike,
    If {
        root_id: usize,
    },
    Case {
        root_id: usize,
    },
    When {
        root_id: usize,
    },
}

struct AncestorFrame {
    kind: AncestorKind,
}

struct GemDeclaration {
    gem_name: Vec<u8>,
    line: usize,
    column: usize,
    conditional_root: Option<usize>,
    /// Number of opaque Block frames between this gem and its nearest conditional root.
    /// Must be 0 for conditional exemption (matches RuboCop's direct-child check).
    blocks_above_conditional: usize,
    /// Full source bytes of the CallNode (e.g., `gem "redcarpet"`). Used to replicate
    /// RuboCop's AST structural equality in `within_conditional?` where `branch == node`
    /// compares by structure, not identity.
    call_source: Vec<u8>,
}

struct GemDeclarationVisitor<'a> {
    source: &'a SourceFile,
    declarations: Vec<GemDeclaration>,
    ancestors: Vec<AncestorFrame>,
    next_conditional_root_id: usize,
    pending_elsif_root: Option<usize>,
    /// Per conditional root ID, the set of source byte slices that are "matchable"
    /// via RuboCop's `branch == node || branch.child_nodes.include?(node)`.
    /// This is the set of node sources at the top 2 levels of the conditional's branches.
    root_matchable_sources: HashMap<usize, Vec<Vec<u8>>>,
}

impl GemDeclarationVisitor<'_> {
    /// Find the nearest conditional root and count opaque Block frames.
    fn conditional_info(&self) -> (Option<usize>, usize) {
        let ancestors = self
            .ancestors
            .get(..self.ancestors.len().saturating_sub(1))
            .unwrap_or(&[]);
        let mut blocks_above = 0;
        for frame in ancestors.iter().rev() {
            match frame.kind {
                AncestorKind::BeginLike => continue,
                AncestorKind::Block => {
                    blocks_above += 1;
                    continue;
                }
                AncestorKind::If { root_id }
                | AncestorKind::When { root_id }
                | AncestorKind::Case { root_id } => {
                    return (Some(root_id), blocks_above);
                }
            }
        }
        (None, 0)
    }

    fn allocate_conditional_root_id(&mut self) -> usize {
        let id = self.next_conditional_root_id;
        self.next_conditional_root_id += 1;
        id
    }
}

/// Collect the set of source byte slices that RuboCop's `within_conditional?`
/// would match against for an `if` root conditional.
///
/// RuboCop's `IfNode#branches` iterates the entire if/elsif/else chain,
/// collecting each body as a separate branch. `within_conditional?` then
/// checks `branch == node || branch.child_nodes.include?(node)`:
/// - Single-statement body: the statement IS the branch → `branch == node` matches
/// - Multi-statement body (begin): individual statements are `child_nodes` → matched
///
/// In Parser gem, `else { single if }` is merged into the elsif chain
/// (identical AST to `elsif`), so we unwrap through it the same way.
fn collect_if_matchable_sources(bytes: &[u8], if_node: &ruby_prism::IfNode<'_>) -> Vec<Vec<u8>> {
    let mut matchable = Vec::new();

    // if body
    add_all_branch_statements(bytes, if_node.statements(), &mut matchable);

    // Walk the elsif/else chain
    let mut subsequent = if_node.subsequent();
    while let Some(sub) = subsequent {
        if let Some(elsif_node) = sub.as_if_node() {
            add_all_branch_statements(bytes, elsif_node.statements(), &mut matchable);
            subsequent = elsif_node.subsequent();
        } else if let Some(else_node) = sub.as_else_node() {
            if let Some(stmts) = else_node.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                // In Parser gem, else { single if } merges into the elsif chain
                if body.len() == 1 {
                    if let Some(nested_if) = body[0].as_if_node() {
                        add_all_branch_statements(bytes, nested_if.statements(), &mut matchable);
                        subsequent = nested_if.subsequent();
                        continue;
                    }
                }
                // Regular else body: all statements are matchable
                for stmt in body {
                    let loc = stmt.location();
                    matchable.push(bytes[loc.start_offset()..loc.end_offset()].to_vec());
                }
            }
            break;
        } else {
            break;
        }
    }

    matchable
}

fn add_all_branch_statements(
    bytes: &[u8],
    stmts: Option<ruby_prism::StatementsNode<'_>>,
    matchable: &mut Vec<Vec<u8>>,
) {
    if let Some(stmts) = stmts {
        for stmt in stmts.body().iter() {
            let loc = stmt.location();
            matchable.push(bytes[loc.start_offset()..loc.end_offset()].to_vec());
        }
    }
}

/// Collect matchable sources for an `unless` root conditional.
/// In Parser gem, `unless cond; body; else; else_body; end` becomes
/// `(if cond else_body body)`, so branches = [else_body, body].
fn collect_unless_matchable_sources(
    bytes: &[u8],
    unless_node: &ruby_prism::UnlessNode<'_>,
) -> Vec<Vec<u8>> {
    let mut matchable = Vec::new();

    // Branch 0: unless body (mapped to else_branch in Parser)
    if let Some(stmts) = unless_node.statements() {
        for stmt in stmts.body().iter() {
            let loc = stmt.location();
            matchable.push(bytes[loc.start_offset()..loc.end_offset()].to_vec());
        }
    }

    // Branch 1: else clause (mapped to if_branch in Parser)
    if let Some(else_clause) = unless_node.else_clause() {
        if let Some(stmts) = else_clause.statements() {
            for stmt in stmts.body().iter() {
                let loc = stmt.location();
                matchable.push(bytes[loc.start_offset()..loc.end_offset()].to_vec());
            }
        }
    }

    matchable
}

/// Collect matchable sources for a `case` root conditional.
/// In Parser gem, `case` branches include all when bodies and the else body.
/// For each branch (which is a begin for multi-statement or the stmt for single),
/// `begin.child_nodes` includes all statements. So all statements from all
/// when/else bodies are matchable.
fn collect_case_matchable_sources(
    bytes: &[u8],
    case_node: &ruby_prism::CaseNode<'_>,
) -> Vec<Vec<u8>> {
    let mut matchable = Vec::new();

    for condition in case_node.conditions().iter() {
        if let Some(when_node) = condition.as_when_node() {
            if let Some(stmts) = when_node.statements() {
                for stmt in stmts.body().iter() {
                    let loc = stmt.location();
                    matchable.push(bytes[loc.start_offset()..loc.end_offset()].to_vec());
                }
            }
        }
    }

    if let Some(else_clause) = case_node.else_clause() {
        if let Some(stmts) = else_clause.statements() {
            for stmt in stmts.body().iter() {
                let loc = stmt.location();
                matchable.push(bytes[loc.start_offset()..loc.end_offset()].to_vec());
            }
        }
    }

    matchable
}

fn gem_name_from_call(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    if call.receiver().is_some() || call.name().as_slice() != b"gem" {
        return None;
    }
    let first_arg = util::first_positional_arg(call)?;
    util::string_value(&first_arg)
}

/// Check if a node is a "transparent" wrapper that should not create an
/// opaque block frame.
///
/// **Why CallNode is transparent:** In Parser gem's AST, a method call with a
/// block (e.g., `group :dev do gem "x" end`) is represented as a single
/// `(block (send ...) (args) body)` node. The `send` node is a child of the
/// `block` node, not a parent. In Prism, the structure is inverted: CallNode
/// contains a BlockNode child. Making CallNode transparent ensures that the
/// opaque/transparent decision is made at the BlockNode level (matching
/// Parser gem's structure).
///
/// **BlockNode is opaque:** In Parser gem, `block` type is NOT `begin_type?`,
/// so it stops the ancestor walk in `each_ancestor.find { |a| !a.begin_type? }`.
/// This means gems inside ANY block (even single-statement) have `:block` as
/// their first non-begin ancestor, NOT `:if`/`:when`. RuboCop's structural
/// equality (`child_nodes.include?`) still finds gems through blocks, but
/// that's handled separately via the `call_source` matching in Path 1.
fn is_transparent_node(node: &ruby_prism::Node<'_>) -> bool {
    node.as_statements_node().is_some()
        || node.as_begin_node().is_some()
        || node.as_else_node().is_some()
        || node.as_program_node().is_some()
        || node.as_call_node().is_some()
        || node.as_arguments_node().is_some()
}

impl<'pr> Visit<'pr> for GemDeclarationVisitor<'_> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        // Transparent wrappers (StatementsNode, BeginNode, ElseNode, ProgramNode,
        // single-statement BlockNode) get BeginLike. Everything else gets Block
        // (opaque). Conditional nodes override their frame in specific visit methods.
        let kind = if is_transparent_node(&node) {
            AncestorKind::BeginLike
        } else {
            AncestorKind::Block
        };
        self.ancestors.push(AncestorFrame { kind });
    }

    fn visit_branch_node_leave(&mut self) {
        self.ancestors.pop();
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        // Both modifier and block `if` are conditional roots. In Parser gem,
        // modifier `if` produces the same `(if ...)` AST node, and the gem's
        // `each_ancestor.find { |a| !a.begin_type? }` stops at the `if` in
        // both cases. Using `take()` to consume pending_elsif_root prevents
        // it from leaking into nested ifs inside elsif/else bodies.
        let is_new_root = self.pending_elsif_root.is_none();
        let root_id = self
            .pending_elsif_root
            .take()
            .unwrap_or_else(|| self.allocate_conditional_root_id());
        if let Some(frame) = self.ancestors.last_mut() {
            frame.kind = AncestorKind::If { root_id };
        }

        // Collect matchable sources only for new root conditionals (not inherited elsifs)
        if is_new_root {
            let matchable = collect_if_matchable_sources(self.source.as_bytes(), node);
            self.root_matchable_sources.insert(root_id, matchable);
        }

        self.visit(&node.predicate());
        if let Some(statements) = node.statements() {
            for statement in statements.body().iter() {
                self.visit(&statement);
            }
        }
        if let Some(subsequent) = node.subsequent() {
            let previous = self.pending_elsif_root;
            if subsequent.as_if_node().is_some() {
                self.pending_elsif_root = Some(root_id);
            } else {
                // Clear pending_elsif_root when entering an else clause to prevent
                // it from leaking into nested if statements inside the else body.
                self.pending_elsif_root = None;
            }
            self.visit(&subsequent);
            self.pending_elsif_root = previous;
        }
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        // Both modifier and block `unless` are conditional roots, matching
        // Parser gem behavior where the gem's ancestor walk stops at `if`
        // (unless is represented as if with inverted condition in Parser).
        let root_id = self.allocate_conditional_root_id();
        if let Some(frame) = self.ancestors.last_mut() {
            frame.kind = AncestorKind::If { root_id };
        }

        let matchable = collect_unless_matchable_sources(self.source.as_bytes(), node);
        self.root_matchable_sources.insert(root_id, matchable);

        self.visit(&node.predicate());
        if let Some(statements) = node.statements() {
            for statement in statements.body().iter() {
                self.visit(&statement);
            }
        }
        if let Some(else_clause) = node.else_clause() {
            if let Some(statements) = else_clause.statements() {
                for statement in statements.body().iter() {
                    self.visit(&statement);
                }
            }
        }
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        let root_id = self.allocate_conditional_root_id();
        if let Some(frame) = self.ancestors.last_mut() {
            frame.kind = AncestorKind::Case { root_id };
        }

        let matchable = collect_case_matchable_sources(self.source.as_bytes(), node);
        self.root_matchable_sources.insert(root_id, matchable);

        ruby_prism::visit_case_node(self, node);
    }

    fn visit_when_node(&mut self, node: &ruby_prism::WhenNode<'pr>) {
        let case_root_id = self
            .ancestors
            .iter()
            .rev()
            .find_map(|frame| match frame.kind {
                AncestorKind::Case { root_id } => Some(root_id),
                _ => None,
            });
        if let Some(frame) = self.ancestors.last_mut() {
            frame.kind = case_root_id
                .map(|root_id| AncestorKind::When { root_id })
                .unwrap_or(AncestorKind::Block);
        }
        ruby_prism::visit_when_node(self, node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(gem_name) = gem_name_from_call(node) {
            let loc = node.message_loc().unwrap_or(node.location());
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            let (conditional_root, mut blocks_above_conditional) = self.conditional_info();
            // In Parser gem, `gem 'x' do...end` is `(block (send ...) ...)` — the
            // send is inside a block node. So the first non-begin ancestor of the
            // send is `:block`, not `:if`. This means `conditional_declaration?`
            // returns false. Reflect this by incrementing blocks_above_conditional
            // when the call has a block attached.
            if node.block().is_some() {
                blocks_above_conditional += 1;
            }
            let call_loc = node.location();
            let call_source =
                self.source.as_bytes()[call_loc.start_offset()..call_loc.end_offset()].to_vec();
            self.declarations.push(GemDeclaration {
                gem_name,
                line,
                column,
                conditional_root,
                blocks_above_conditional,
                call_source,
            });
        }
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicatedGem, "cops/bundler/duplicated_gem");
}
