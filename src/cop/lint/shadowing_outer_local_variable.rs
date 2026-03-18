use std::collections::{HashMap, HashSet};

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for block parameters or block-local variables that shadow outer local variables.
///
/// ## Root causes of historical FP/FN (corpus conformance ~57%):
///
/// 1. **FP: Variable added to scope before RHS visited.** `visit_local_variable_write_node`
///    called `add_local` before visiting the value child. This caused `foo = bar { |foo| ... }`
///    to incorrectly flag `foo` as shadowing, because the LHS `foo` was already in scope when
///    the block was processed. RuboCop's VariableForce processes the RHS before declaring the
///    variable, so `foo` isn't in scope yet. Fix: visit the value first, then add to scope.
///
/// 2. **FN: Overly aggressive conditional suppression.** The `is_different_conditional_branch`
///    function had a `(None, Some(_)) => true` case that suppressed ALL shadowing when the
///    block was inside any conditional but the outer var was not. Per RuboCop, suppression
///    only applies when BOTH the outer var and the block are in different branches of the
///    SAME conditional node. Fix: remove the incorrect `(None, Some(_))` case.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=19, FN=51.
///
/// FP:
/// - Method parameters were predeclared before any default expression ran, so later
///   parameters leaked into earlier lambda defaults like
///   `outer: ->(cursor) { ... }, cursor: nil`.
/// - Class/module/singleton class bodies only pushed a nested scope, so top-level locals
///   leaked into class-body procs and lambdas.
///
/// FN:
/// - `params.posts()` and `params.keyword_rest()` were not checked or collected, so shadowing
///   was missed for post-splat params and `**kwargs`.
/// - Lambda/block body scopes also omitted some parameter kinds, so nested blocks could miss
///   outer block params.
///
/// ## Corpus investigation (2026-03-14)
///
/// Corpus oracle reported FP=10, FN=42.
///
/// FP fixes applied:
/// - **Ractor.new block detection**: `is_ractor_new_call` detects `Ractor.new(...)` calls
///   and handles their blocks with an isolated scope (no shadowing check). RuboCop
///   explicitly skips Ractor blocks because Ractors cannot access outer scope, so
///   shadowing is intentional. Previously `is_ractor_new_block` was stubbed out.
///   Implementation uses `visit_call_node` override since Prism's BlockNode lacks
///   parent pointers.
/// - **When-condition assignment suppression**: Variables assigned in `when` conditions
///   (e.g., `when decl = env.fetch(...)`) are now marked with `when_condition_of_case`.
///   Blocks in the same `when` body that reuse the variable name are suppressed.
///   This matches RuboCop's `same_conditions_node_different_branch?` logic where both
///   the block and the outer variable resolve to the same conditional (case) node.
///
/// ## Corpus investigation (2026-03-17)
///
/// Corpus oracle reported FP=4, FN=26.
///
/// FN fixes applied:
/// - **Predicate vs body distinction for adjacent-elsif suppression**: The
///   conditional_branch_stack now tracks an `is_body` flag. The adjacent-elsif
///   suppression only applies when the block is in a body, not in a condition
///   expression. Fixes `elsif entry = list.find { |entry| }` where block is in
///   the elsif condition (e.g., danbooru upload.rb lines 106/108).
///
/// Remaining FP (4): Thread.new(val) { |val| } suppression needs
/// `variable_used_in_declaration_of_outer?` which requires AST parent pointers.
/// opf/openproject and trogdoro/xiki FPs require VariableForce-level analysis.
///
/// Remaining FN (18): Most require `variable_used_in_declaration_of_outer?` or
/// deeper VariableForce tracking that nitrocop doesn't implement.
///
/// ## Corpus fix (2026-03-18): FP=4→2, FN=19 unchanged
///
/// Fixed 2 of 4 FPs via two new suppression mechanisms:
///
/// 1. **Adjacent-elsif suppression extended to predicates**: Previously
///    Check 2 only fired when `block_is_in_body=true`. Changed to
///    `block_is_in_body || !outer_info.is_condition_var`, which allows
///    predicate blocks to be suppressed when the outer var was body-assigned.
///    Fixes FP in molybdenum-99/reality where `list` assigned in one elsif
///    body and block `{ |list| }` is in the next elsif's condition.
///
/// 2. **Same-conditional-node suppression (Check 3)**: When the outer var
///    was assigned in a conditional's predicate (`is_condition_var=true`)
///    and the block is in the same conditional's body, suppress. Matches
///    RuboCop's `variable_node == outer_local_variable_node` check. Fixes
///    FP in opf/openproject where `if item = page.menu_item` followed by
///    `item.tap { |item| }` in the then-body.
///
/// Remaining FP (2): opal/opal Thread.new(value) { |value| } and
/// trogdoro/xiki nested block shadowing require deeper analysis —
/// possibly VariableForce-level semantics or a corpus oracle anomaly.
///
/// Remaining FN (19): Require `variable_used_in_declaration_of_outer?`
/// or deeper VariableForce tracking that nitrocop doesn't implement.
pub struct ShadowingOuterLocalVariable;

impl Cop for ShadowingOuterLocalVariable {
    fn name(&self) -> &'static str {
        "Lint/ShadowingOuterLocalVariable"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    /// This cop is disabled by default in RuboCop (Enabled: false).
    fn default_enabled(&self) -> bool {
        false
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
        let mut visitor = ShadowVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            scopes: Vec::new(),
            conditional_branch_stack: Vec::new(),
            when_condition_case_offset: None,
            in_when_body_of_case: None,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Info about where a variable was declared.
#[derive(Clone, Debug)]
struct VarInfo {
    /// If the variable was declared inside a `when`/`if`/`else` branch,
    /// this is the (cond_node_offset, branch_offset) pair for the NEAREST
    /// conditional. Used to skip shadowing when block and outer var are in
    /// different branches of the same conditional.
    conditional_branch: Option<(usize, usize)>,
    /// The offset of the nearest conditional's subsequent/else clause.
    /// Used to check adjacent elsif suppression: if a block's nearest
    /// conditional starts at this offset, the block is in the immediate
    /// else/elsif of the variable's conditional → suppress.
    cond_subsequent_offset: Option<usize>,
    /// If the variable was assigned inside a `when` condition (not body),
    /// this is the case node offset. Used to suppress shadowing when a
    /// block in the same `when` body reuses the variable name — matching
    /// RuboCop's VariableForce behavior where both resolve to the same
    /// conditional (case) node.
    when_condition_of_case: Option<usize>,
    /// True if the variable was assigned in a conditional's predicate/condition
    /// (is_body=false), e.g., `if item = get_item`. Used for the "same
    /// conditional node" suppression: when both the outer variable and the
    /// block resolve to the same conditional node, RuboCop suppresses.
    is_condition_var: bool,
    /// True if the outer variable's conditional is an if/unless (not case).
    /// Used to determine suppression behavior: if-type else clauses always
    /// suppress regardless of statement count, while case/when branches
    /// only suppress for single-statement bodies.
    is_if_type_cond: bool,
}

/// Context about the block being checked for shadowing, bundled to avoid
/// threading many separate parameters through the call chain.
#[derive(Clone, Copy)]
struct BlockContext {
    cond_branch: Option<(usize, usize)>,
    is_in_body: bool,
    single_stmt: bool,
    in_when_body_of_case: Option<usize>,
}

/// Entry in the conditional branch stack tracking current conditional context.
#[derive(Clone, Copy)]
struct CondBranchEntry {
    /// Offset of the conditional node (if/unless/case).
    cond_offset: usize,
    /// Offset of the current branch body within the conditional.
    branch_offset: usize,
    /// Offset of the conditional's subsequent (else/elsif) clause, if any.
    subsequent_offset: Option<usize>,
    /// True if this is a body visit (then/else), false for predicate.
    is_body: bool,
    /// True if the conditional is an if or unless node (not case/when).
    is_if_type: bool,
    /// True if the branch body has a single statement.
    single_stmt: bool,
}

struct ShadowVisitor<'a, 'src> {
    cop: &'a ShadowingOuterLocalVariable,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Stack of maps of local variable names -> declaration info.
    scopes: Vec<HashMap<String, VarInfo>>,
    /// Stack of conditional branch contexts.
    conditional_branch_stack: Vec<CondBranchEntry>,
    /// When visiting a `when` condition, the case node offset.
    /// Variables assigned while this is Some are marked as when-condition vars.
    when_condition_case_offset: Option<usize>,
    /// When inside a `when` body, the case node offset for suppression checks.
    in_when_body_of_case: Option<usize>,
}

impl ShadowVisitor<'_, '_> {
    fn current_locals(&self) -> HashMap<String, VarInfo> {
        let mut all = HashMap::new();
        for scope in &self.scopes {
            for (name, info) in scope {
                all.insert(name.clone(), info.clone());
            }
        }
        all
    }

    fn add_local(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            // Only set VarInfo on first declaration. Reassignments inside
            // nested conditionals (e.g., `x = x.next while cond`) would
            // overwrite the original conditional context, breaking
            // same-branch detection for case/when etc. RuboCop's
            // VariableForce uses the FIRST declaration's location for
            // `find_conditional_node_from_ascendant`.
            if scope.contains_key(name) {
                return;
            }
            let last = self.conditional_branch_stack.last();
            let is_condition_var = matches!(last, Some(e) if !e.is_body);
            let info = VarInfo {
                conditional_branch: last.map(|e| (e.cond_offset, e.branch_offset)),
                cond_subsequent_offset: last.and_then(|e| e.subsequent_offset),
                when_condition_of_case: self.when_condition_case_offset,
                is_condition_var,
                is_if_type_cond: last.is_some_and(|e| e.is_if_type),
            };
            scope.insert(name.to_string(), info);
        }
    }

    fn current_conditional_branch(&self) -> Option<(usize, usize)> {
        self.conditional_branch_stack
            .last()
            .map(|e| (e.cond_offset, e.branch_offset))
    }

    /// Returns true if the current innermost conditional context is a body
    /// (then/else) rather than a predicate/condition.
    fn current_is_body(&self) -> bool {
        matches!(self.conditional_branch_stack.last(), Some(e) if e.is_body)
    }

    /// Returns true if the current branch is a single-statement body.
    fn current_is_single_stmt(&self) -> bool {
        matches!(self.conditional_branch_stack.last(), Some(e) if e.single_stmt)
    }

    /// Visit an if/elsif/else node. Each IfNode uses its own offset as the
    /// conditional identity. The then-body and else/subsequent share this offset
    /// but with different branch_offsets. This matches RuboCop's Parser-gem
    /// behavior where each elsif is a nested if node.
    ///
    /// Adjacent elsif suppression is handled via `cond_subsequent_offset`:
    /// variables in the then-body record the subsequent's offset so that
    /// blocks in the immediate next elsif can be recognized as "adjacent."
    fn visit_if_node_impl(&mut self, node: &ruby_prism::IfNode<'_>) {
        let if_offset = node.location().start_offset();
        let subsequent_offset = node.subsequent().map(|s| s.location().start_offset());

        // Compute then-body branch offset for predicate context.
        let then_branch_offset = node
            .statements()
            .map(|s| s.location().start_offset())
            .unwrap_or(if_offset);

        let then_single_stmt = node.statements().map_or(true, |s| s.body().len() <= 1);

        // Visit predicate with the then-body's conditional context (is_body=false).
        self.conditional_branch_stack.push(CondBranchEntry {
            cond_offset: if_offset,
            branch_offset: then_branch_offset,
            subsequent_offset,
            is_body: false,
            is_if_type: true,
            single_stmt: then_single_stmt,
        });
        self.visit(&node.predicate());
        self.conditional_branch_stack.pop();

        // Visit then-body with the same branch tracking (is_body=true).
        if let Some(stmts) = node.statements() {
            self.conditional_branch_stack.push(CondBranchEntry {
                cond_offset: if_offset,
                branch_offset: then_branch_offset,
                subsequent_offset,
                is_body: true,
                is_if_type: true,
                single_stmt: then_single_stmt,
            });
            self.visit_statements_node(&stmts);
            self.conditional_branch_stack.pop();
        }

        // Visit else/elsif
        if let Some(subsequent) = node.subsequent() {
            if let Some(elsif_node) = subsequent.as_if_node() {
                // elsif — push this if's else context, then visit the elsif
                // which will push its own context on top
                let branch_offset = subsequent.location().start_offset();
                self.conditional_branch_stack.push(CondBranchEntry {
                    cond_offset: if_offset,
                    branch_offset,
                    subsequent_offset: None,
                    is_body: true,
                    is_if_type: true,
                    single_stmt: false, // not directly relevant; elsif pushes its own entry
                });
                self.visit_if_node_impl(&elsif_node);
                self.conditional_branch_stack.pop();
            } else {
                // else clause
                let branch_offset = subsequent.location().start_offset();
                let else_single_stmt = subsequent
                    .as_else_node()
                    .and_then(|e| e.statements())
                    .map_or(true, |s| s.body().len() <= 1);
                self.conditional_branch_stack.push(CondBranchEntry {
                    cond_offset: if_offset,
                    branch_offset,
                    subsequent_offset: None,
                    is_body: true,
                    is_if_type: true,
                    single_stmt: else_single_stmt,
                });
                self.visit(&subsequent);
                self.conditional_branch_stack.pop();
            }
        }
    }

    /// Visit a when node, tracking when-condition vs when-body context.
    /// Variables assigned in when conditions are marked with `when_condition_of_case`
    /// so that blocks in the same when body don't report false-positive shadowing.
    fn visit_when_node_with_case_offset(
        &mut self,
        node: &ruby_prism::WhenNode<'_>,
        case_offset: usize,
    ) {
        // Visit when conditions with when_condition_case_offset set
        let saved = self.when_condition_case_offset;
        self.when_condition_case_offset = Some(case_offset);
        for condition in node.conditions().iter() {
            self.visit(&condition);
        }
        self.when_condition_case_offset = saved;

        // Visit when body with in_when_body_of_case set
        if let Some(stmts) = node.statements() {
            let saved_body = self.in_when_body_of_case;
            self.in_when_body_of_case = Some(case_offset);
            self.visit_statements_node(&stmts);
            self.in_when_body_of_case = saved_body;
        }
    }

    fn visit_def_parameters_in_order(&mut self, params: &ruby_prism::ParametersNode<'_>) {
        for param in params.requireds().iter() {
            self.declare_parameter_node(&param);
        }

        for param in params.optionals().iter() {
            if let Some(optional) = param.as_optional_parameter_node() {
                self.visit(&optional.value());
                if let Ok(name) = std::str::from_utf8(optional.name().as_slice()) {
                    self.add_local(name);
                }
            }
        }

        if let Some(rest) = params.rest() {
            if let Some(rest_param) = rest.as_rest_parameter_node() {
                if let Some(name) = rest_param.name() {
                    if let Ok(name) = std::str::from_utf8(name.as_slice()) {
                        self.add_local(name);
                    }
                }
            }
        }

        for param in params.posts().iter() {
            self.declare_parameter_node(&param);
        }

        for param in params.keywords().iter() {
            if let Some(keyword) = param.as_required_keyword_parameter_node() {
                if let Ok(name) = std::str::from_utf8(keyword.name().as_slice()) {
                    self.add_local(name.trim_end_matches(':'));
                }
            } else if let Some(keyword) = param.as_optional_keyword_parameter_node() {
                self.visit(&keyword.value());
                if let Ok(name) = std::str::from_utf8(keyword.name().as_slice()) {
                    self.add_local(name.trim_end_matches(':'));
                }
            }
        }

        if let Some(keyword_rest) = params.keyword_rest() {
            if let Some(keyword_rest) = keyword_rest.as_keyword_rest_parameter_node() {
                if let Some(name) = keyword_rest.name() {
                    if let Ok(name) = std::str::from_utf8(name.as_slice()) {
                        self.add_local(name);
                    }
                }
            }
        }

        if let Some(block) = params.block() {
            if let Some(name) = block.name() {
                if let Ok(name) = std::str::from_utf8(name.as_slice()) {
                    self.add_local(name);
                }
            }
        }
    }

    fn declare_parameter_node(&mut self, node: &ruby_prism::Node<'_>) {
        if let Some(required) = node.as_required_parameter_node() {
            if let Ok(name) = std::str::from_utf8(required.name().as_slice()) {
                self.add_local(name);
            }
            return;
        }

        if let Some(multi_target) = node.as_multi_target_node() {
            let mut names = HashSet::new();
            collect_multi_target_names(&multi_target, &mut names);
            for name in names {
                self.add_local(&name);
            }
            return;
        }

        if let Some(keyword_rest) = node.as_keyword_rest_parameter_node() {
            if let Some(name) = keyword_rest.name() {
                if let Ok(name) = std::str::from_utf8(name.as_slice()) {
                    self.add_local(name);
                }
            }
        }
    }
}

impl<'pr> Visit<'pr> for ShadowVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(receiver) = node.receiver() {
            self.visit(&receiver);
        }

        // def creates a completely new scope — save and replace the entire scope
        // stack. RuboCop's VariableForce treats method definitions as scope
        // barriers: class/module-level variables are NOT visible inside methods.
        let saved_scopes = std::mem::take(&mut self.scopes);
        let saved_cond = std::mem::take(&mut self.conditional_branch_stack);
        self.scopes.push(HashMap::new());
        if let Some(params) = node.parameters() {
            self.visit_def_parameters_in_order(&params);
        }
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scopes = saved_scopes;
        self.conditional_branch_stack = saved_cond;
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        self.visit(&node.constant_path());
        if let Some(superclass) = node.superclass() {
            self.visit(&superclass);
        }
        let saved_scopes = std::mem::take(&mut self.scopes);
        self.scopes.push(HashMap::new());
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scopes = saved_scopes;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        self.visit(&node.constant_path());
        let saved_scopes = std::mem::take(&mut self.scopes);
        self.scopes.push(HashMap::new());
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scopes = saved_scopes;
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        // Visit the value (RHS) BEFORE adding the variable to scope.
        // This matches RuboCop's VariableForce which processes the RHS before
        // declaring the LHS variable. Without this ordering, patterns like
        // `foo = bar { |foo| baz(foo) }` would incorrectly flag `foo` as
        // shadowing because the LHS `foo` would already be in scope.
        self.visit(&node.value());
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.add_local(&name);
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        self.visit(&node.value());
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.add_local(&name);
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        self.visit(&node.value());
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.add_local(&name);
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        self.visit(&node.value());
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.add_local(&name);
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.add_local(&name);
    }

    fn visit_multi_write_node(&mut self, node: &ruby_prism::MultiWriteNode<'pr>) {
        // Visit the value (RHS) first before adding targets to scope
        self.visit(&node.value());
        // Then add all target locals to scope
        for target in node.lefts().iter() {
            if let Some(local) = target.as_local_variable_target_node() {
                let name = std::str::from_utf8(local.name().as_slice())
                    .unwrap_or("")
                    .to_string();
                self.add_local(&name);
            }
        }
        if let Some(rest) = node.rest() {
            if let Some(splat) = rest.as_splat_node() {
                if let Some(expr) = splat.expression() {
                    if let Some(local) = expr.as_local_variable_target_node() {
                        let name = std::str::from_utf8(local.name().as_slice())
                            .unwrap_or("")
                            .to_string();
                        self.add_local(&name);
                    }
                }
            }
        }
        for target in node.rights().iter() {
            if let Some(local) = target.as_local_variable_target_node() {
                let name = std::str::from_utf8(local.name().as_slice())
                    .unwrap_or("")
                    .to_string();
                self.add_local(&name);
            }
        }
    }

    // Singleton class (class << self) creates a new scope
    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        self.visit(&node.expression());
        let saved_scopes = std::mem::take(&mut self.scopes);
        self.scopes.push(HashMap::new());
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scopes = saved_scopes;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Ractor.new blocks have an isolated scope (no shadowing check).
        // RuboCop explicitly skips Ractor because Ractor cannot access
        // outer variables by design.
        if is_ractor_new_call(node) {
            // Visit receiver and arguments normally
            if let Some(receiver) = node.receiver() {
                self.visit(&receiver);
            }
            if let Some(arguments) = node.arguments() {
                self.visit_arguments_node(&arguments);
            }
            // Visit block with isolated scope (no shadowing check)
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
                    self.scopes.push(HashMap::new());
                    ruby_prism::visit_block_node(self, &block_node);
                    self.scopes.pop();
                }
            }
            return;
        }
        // Default call node visiting
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let outer_locals = self.current_locals();
        let bctx = BlockContext {
            cond_branch: self.current_conditional_branch(),
            is_in_body: self.current_is_body(),
            single_stmt: self.current_is_single_stmt(),

            in_when_body_of_case: self.in_when_body_of_case,
        };

        // Check block parameters against outer locals
        if let Some(params_node) = node.parameters() {
            if let Some(block_params) = params_node.as_block_parameters_node() {
                check_block_parameters_shadow(
                    self.cop,
                    self.source,
                    &block_params,
                    &outer_locals,
                    &bctx,
                    &mut self.diagnostics,
                );
            }
        }

        // Push a new scope for the block body that includes the block parameters.
        // This ensures inner blocks can see outer block params for shadowing detection.
        // Do NOT merge back into the outer scope — block-internal variables are
        // local to the block, not visible to sibling blocks.
        let mut body_scope = HashMap::new();
        if let Some(params_node) = node.parameters() {
            if let Some(block_params) = params_node.as_block_parameters_node() {
                body_scope = build_block_body_scope(&block_params);
            }
        }
        self.scopes.push(body_scope);
        ruby_prism::visit_block_node(self, node);
        self.scopes.pop();
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // Lambdas behave like blocks for shadowing purposes
        let outer_locals = self.current_locals();
        let bctx = BlockContext {
            cond_branch: self.current_conditional_branch(),
            is_in_body: self.current_is_body(),
            single_stmt: self.current_is_single_stmt(),

            in_when_body_of_case: self.in_when_body_of_case,
        };

        if let Some(params_node) = node.parameters() {
            if let Some(block_params) = params_node.as_block_parameters_node() {
                check_block_parameters_shadow(
                    self.cop,
                    self.source,
                    &block_params,
                    &outer_locals,
                    &bctx,
                    &mut self.diagnostics,
                );
            }
        }

        // Lambda creates an isolated scope — params do NOT persist to the
        // enclosing scope (unlike blocks). Lambda parameters are method-like.
        let mut body_scope = HashMap::new();
        if let Some(params_node) = node.parameters() {
            if let Some(block_params) = params_node.as_block_parameters_node() {
                body_scope = build_block_body_scope(&block_params);
            }
        }
        self.scopes.push(body_scope);
        ruby_prism::visit_lambda_node(self, node);
        self.scopes.pop();
    }

    // Handle top-level assignments (outside any method)
    fn visit_program_node(&mut self, node: &ruby_prism::ProgramNode<'pr>) {
        self.scopes.push(HashMap::new());
        ruby_prism::visit_program_node(self, node);
        self.scopes.pop();
    }

    // Track unless/else branches for the same_conditions_node_different_branch check.
    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let unless_offset = node.location().start_offset();
        let else_offset = node.else_clause().map(|e| e.location().start_offset());

        let body_single_stmt = node.statements().map_or(true, |s| s.body().len() <= 1);

        // Visit predicate normally
        self.visit(&node.predicate());

        // Visit body (the unless-true branch) with branch tracking
        if let Some(stmts) = node.statements() {
            let branch_offset = stmts.location().start_offset();
            self.conditional_branch_stack.push(CondBranchEntry {
                cond_offset: unless_offset,
                branch_offset,
                subsequent_offset: else_offset,
                is_body: true,
                is_if_type: true,
                single_stmt: body_single_stmt,
            });
            self.visit_statements_node(&stmts);
            self.conditional_branch_stack.pop();
        }

        // Visit else clause with branch tracking
        if let Some(else_clause) = node.else_clause() {
            let branch_offset = else_clause.location().start_offset();
            self.conditional_branch_stack.push(CondBranchEntry {
                cond_offset: unless_offset,
                branch_offset,
                subsequent_offset: None,
                is_body: true,
                is_if_type: true,
                single_stmt: false, // not used for else of unless
            });
            self.visit_else_node(&else_clause);
            self.conditional_branch_stack.pop();
        }
    }

    // Handle for loops and while/until which share scope
    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'pr>) {
        ruby_prism::visit_for_node(self, node);
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        // While loops are conditionals in RuboCop's model.
        let while_offset = node.location().start_offset();
        let pred_offset = node.predicate().location().start_offset();
        self.conditional_branch_stack.push(CondBranchEntry {
            cond_offset: while_offset,
            branch_offset: pred_offset,
            subsequent_offset: None,
            is_body: true,
            is_if_type: false,
            single_stmt: false,
        });
        ruby_prism::visit_while_node(self, node);
        self.conditional_branch_stack.pop();
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        let until_offset = node.location().start_offset();
        let pred_offset = node.predicate().location().start_offset();
        self.conditional_branch_stack.push(CondBranchEntry {
            cond_offset: until_offset,
            branch_offset: pred_offset,
            subsequent_offset: None,
            is_body: true,
            is_if_type: false,
            single_stmt: false,
        });
        ruby_prism::visit_until_node(self, node);
        self.conditional_branch_stack.pop();
    }

    // Track case/when branches for the same_conditions_node_different_branch check.
    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        let case_offset = node.location().start_offset();

        // Visit the predicate (expression after `case`) with its own branch context.
        if let Some(pred) = node.predicate() {
            let pred_offset = pred.location().start_offset();
            self.conditional_branch_stack.push(CondBranchEntry {
                cond_offset: case_offset,
                branch_offset: pred_offset,
                subsequent_offset: None,
                is_body: false,
                is_if_type: false,
                single_stmt: true,
            });
            self.visit(&pred);
            self.conditional_branch_stack.pop();
        }

        // Visit each when clause with branch tracking
        for condition in node.conditions().iter() {
            let branch_offset = condition.location().start_offset();
            let when_single_stmt = condition
                .as_when_node()
                .and_then(|w| w.statements())
                .map_or(true, |s| s.body().len() <= 1);
            self.conditional_branch_stack.push(CondBranchEntry {
                cond_offset: case_offset,
                branch_offset,
                subsequent_offset: None,
                is_body: true,
                is_if_type: false,
                single_stmt: when_single_stmt,
            });
            if let Some(when_node) = condition.as_when_node() {
                self.visit_when_node_with_case_offset(&when_node, case_offset);
            } else {
                self.visit(&condition);
            }
            self.conditional_branch_stack.pop();
        }

        // Visit the else clause (consequent) with its own branch
        if let Some(else_clause) = node.else_clause() {
            let branch_offset = else_clause.location().start_offset();
            let else_single_stmt = else_clause
                .statements()
                .map_or(true, |s| s.body().len() <= 1);
            self.conditional_branch_stack.push(CondBranchEntry {
                cond_offset: case_offset,
                branch_offset,
                subsequent_offset: None,
                is_body: true,
                is_if_type: false,
                single_stmt: else_single_stmt,
            });
            self.visit_else_node(&else_clause);
            self.conditional_branch_stack.pop();
        }
    }

    // Track if/unless branches for the same_conditions_node_different_branch check.
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        self.visit_if_node_impl(node);
    }
}

/// Check if the outer variable and block are in different branches of the same
/// conditional, meaning they can never both be in scope. Returns true if
/// shadowing should be suppressed.
///
/// This approximates RuboCop's `same_conditions_node_different_branch?` logic,
/// which depends on Parser gem AST parent pointers. In RuboCop:
/// - `variable_node(variable)` returns the block's parent in the AST
/// - Suppression fires when `variable_node == outer_local_variable_node` or
///   `variable_node == outer.else_branch`
///
/// In practice, this means:
/// - For if/unless: always suppress different branches (Check 1 covers if/else,
///   Check 2 covers if/elsif). In Parser gem, the else_branch of an if always
///   matches the block's parent regardless of statement count.
/// - For case/when: only suppress when the block is in a single-statement branch,
///   because multi-statement branches wrap in `begin` which doesn't match `case`.
/// - For elsif (via Check 2): only suppress when the block is in a single-statement
///   elsif body, because multi-statement bodies use `begin` which doesn't match.
fn is_different_conditional_branch(
    outer_info: &VarInfo,
    block_branch: Option<(usize, usize)>,
    block_is_in_body: bool,
    block_single_stmt: bool,
) -> bool {
    let Some(block_branch) = block_branch else {
        return false;
    };
    // Check 1: same conditional node, different branch.
    // For if/unless: always suppress (RuboCop's else_branch check covers all
    // statement counts — block.parent for if branch is always the if node).
    // For case/when: outer always resolves to case (find_conditional_node
    // skips when nodes). Block resolves to case only when single-stmt
    // (block.parent = when → case). Multi-stmt → block.parent = begin.
    if let Some((outer_cond, outer_branch)) = outer_info.conditional_branch {
        if outer_cond == block_branch.0 && outer_branch != block_branch.1 {
            if outer_info.is_if_type_cond || block_single_stmt {
                return true;
            }
        }
    }
    // Check 2: adjacent elsif suppression.
    // Only suppress when the block is in a single-statement branch body.
    // In Parser gem, multi-statement elsif bodies wrap in `begin` which
    // doesn't match the elsif node (outer.else_branch), so RuboCop doesn't
    // suppress. Single-statement bodies have the block's parent = elsif node
    // which matches outer.else_branch.
    //
    // Exception: condition-assigned vars where the block is also in a
    // predicate must NOT be suppressed.
    if block_single_stmt && (block_is_in_body || !outer_info.is_condition_var) {
        if let Some(subsequent_offset) = outer_info.cond_subsequent_offset {
            if block_branch.0 == subsequent_offset {
                return true;
            }
        }
    }
    // Check 3: same conditional node suppression.
    // When the outer variable was assigned in a conditional's predicate
    // (e.g., `if item = get_item`) and the block is in the same
    // conditional's then-body, suppress.
    if outer_info.is_condition_var {
        if let Some((outer_cond, _)) = outer_info.conditional_branch {
            if outer_cond == block_branch.0 {
                return true;
            }
        }
    }
    false
}

/// Check if a CallNode is `Ractor.new(...)` or `::Ractor.new(...)`.
fn is_ractor_new_call(node: &ruby_prism::CallNode<'_>) -> bool {
    let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
    if name != "new" {
        return false;
    }
    if let Some(receiver) = node.receiver() {
        if let Some(constant) = receiver.as_constant_read_node() {
            let const_name = std::str::from_utf8(constant.name().as_slice()).unwrap_or("");
            return const_name == "Ractor";
        }
        if let Some(path) = receiver.as_constant_path_node() {
            if let Some(child) = path.name() {
                let const_name = std::str::from_utf8(child.as_slice()).unwrap_or("");
                return const_name == "Ractor";
            }
        }
    }
    false
}

/// Check multi-target (destructured) block params for shadowing.
/// E.g., `|(theme_id, upload_id, sprite)|`
fn check_multi_target_shadow(
    cop: &ShadowingOuterLocalVariable,
    source: &SourceFile,
    mt: &ruby_prism::MultiTargetNode<'_>,
    outer_locals: &HashMap<String, VarInfo>,
    bctx: &BlockContext,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for target in mt.lefts().iter() {
        if let Some(req) = target.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(
                cop,
                source,
                &name,
                req.location(),
                outer_locals,
                bctx,
                diagnostics,
            );
        } else if let Some(inner) = target.as_multi_target_node() {
            check_multi_target_shadow(cop, source, &inner, outer_locals, bctx, diagnostics);
        }
    }
    // Rest param (*splat) inside destructuring
    if let Some(rest) = mt.rest() {
        if let Some(splat) = rest.as_splat_node() {
            if let Some(expr) = splat.expression() {
                if let Some(req) = expr.as_required_parameter_node() {
                    let name = std::str::from_utf8(req.name().as_slice())
                        .unwrap_or("")
                        .to_string();
                    check_shadow(
                        cop,
                        source,
                        &name,
                        req.location(),
                        outer_locals,
                        bctx,
                        diagnostics,
                    );
                }
            }
        }
    }
    for target in mt.rights().iter() {
        if let Some(req) = target.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(
                cop,
                source,
                &name,
                req.location(),
                outer_locals,
                bctx,
                diagnostics,
            );
        }
    }
}

fn check_block_parameters_shadow(
    cop: &ShadowingOuterLocalVariable,
    source: &SourceFile,
    block_params: &ruby_prism::BlockParametersNode<'_>,
    outer_locals: &HashMap<String, VarInfo>,
    bctx: &BlockContext,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(inner_params) = block_params.parameters() {
        check_block_params_shadow(cop, source, &inner_params, outer_locals, bctx, diagnostics);

        for param in inner_params.requireds().iter() {
            if let Some(multi_target) = param.as_multi_target_node() {
                check_multi_target_shadow(
                    cop,
                    source,
                    &multi_target,
                    outer_locals,
                    bctx,
                    diagnostics,
                );
            }
        }

        for param in inner_params.posts().iter() {
            if let Some(multi_target) = param.as_multi_target_node() {
                check_multi_target_shadow(
                    cop,
                    source,
                    &multi_target,
                    outer_locals,
                    bctx,
                    diagnostics,
                );
            }
        }
    }

    for local in block_params.locals().iter() {
        let name = std::str::from_utf8(
            local
                .as_block_local_variable_node()
                .map_or(&[][..], |node| node.name().as_slice()),
        )
        .unwrap_or("")
        .to_string();
        check_shadow(
            cop,
            source,
            &name,
            local.location(),
            outer_locals,
            bctx,
            diagnostics,
        );
    }
}

fn check_block_params_shadow(
    cop: &ShadowingOuterLocalVariable,
    source: &SourceFile,
    params: &ruby_prism::ParametersNode<'_>,
    outer_locals: &HashMap<String, VarInfo>,
    bctx: &BlockContext,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for p in params.requireds().iter() {
        if let Some(req) = p.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(
                cop,
                source,
                &name,
                req.location(),
                outer_locals,
                bctx,
                diagnostics,
            );
        }
    }
    for p in params.optionals().iter() {
        if let Some(opt) = p.as_optional_parameter_node() {
            let name = std::str::from_utf8(opt.name().as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(
                cop,
                source,
                &name,
                opt.location(),
                outer_locals,
                bctx,
                diagnostics,
            );
        }
    }
    for p in params.posts().iter() {
        if let Some(req) = p.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(
                cop,
                source,
                &name,
                req.location(),
                outer_locals,
                bctx,
                diagnostics,
            );
        }
    }
    for p in params.keywords().iter() {
        if let Some(keyword) = p.as_required_keyword_parameter_node() {
            let name = std::str::from_utf8(keyword.name().as_slice())
                .unwrap_or("")
                .trim_end_matches(':')
                .to_string();
            check_shadow(
                cop,
                source,
                &name,
                keyword.location(),
                outer_locals,
                bctx,
                diagnostics,
            );
        } else if let Some(keyword) = p.as_optional_keyword_parameter_node() {
            let name = std::str::from_utf8(keyword.name().as_slice())
                .unwrap_or("")
                .trim_end_matches(':')
                .to_string();
            check_shadow(
                cop,
                source,
                &name,
                keyword.location(),
                outer_locals,
                bctx,
                diagnostics,
            );
        }
    }
    if let Some(rest) = params.rest() {
        if let Some(rest_param) = rest.as_rest_parameter_node() {
            if let Some(name_const) = rest_param.name() {
                let name = std::str::from_utf8(name_const.as_slice())
                    .unwrap_or("")
                    .to_string();
                check_shadow(
                    cop,
                    source,
                    &name,
                    rest_param.location(),
                    outer_locals,
                    bctx,
                    diagnostics,
                );
            }
        }
    }
    if let Some(keyword_rest) = params.keyword_rest() {
        if let Some(keyword_rest) = keyword_rest.as_keyword_rest_parameter_node() {
            if let Some(name) = keyword_rest.name() {
                let name = std::str::from_utf8(name.as_slice())
                    .unwrap_or("")
                    .to_string();
                check_shadow(
                    cop,
                    source,
                    &name,
                    keyword_rest.location(),
                    outer_locals,
                    bctx,
                    diagnostics,
                );
            }
        }
    }
    if let Some(block) = params.block() {
        if let Some(name_const) = block.name() {
            let name = std::str::from_utf8(name_const.as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(
                cop,
                source,
                &name,
                block.location(),
                outer_locals,
                bctx,
                diagnostics,
            );
        }
    }
}

fn build_block_body_scope(
    block_params: &ruby_prism::BlockParametersNode<'_>,
) -> HashMap<String, VarInfo> {
    let mut scope = HashMap::new();

    if let Some(params) = block_params.parameters() {
        let mut param_names = HashSet::new();
        collect_param_names_into(&params, &mut param_names);
        for name in param_names {
            scope.insert(
                name,
                VarInfo {
                    conditional_branch: None,
                    cond_subsequent_offset: None,
                    when_condition_of_case: None,
                    is_condition_var: false,
                    is_if_type_cond: false,
                },
            );
        }
        collect_multi_target_names_from_params(&params, &mut scope);
    }

    for local in block_params.locals().iter() {
        let Some(local) = local.as_block_local_variable_node() else {
            continue;
        };
        if let Ok(name) = std::str::from_utf8(local.name().as_slice()) {
            scope.insert(
                name.to_string(),
                VarInfo {
                    conditional_branch: None,
                    cond_subsequent_offset: None,
                    when_condition_of_case: None,
                    is_condition_var: false,
                    is_if_type_cond: false,
                },
            );
        }
    }

    scope
}

fn check_shadow(
    cop: &ShadowingOuterLocalVariable,
    source: &SourceFile,
    name: &str,
    loc: ruby_prism::Location<'_>,
    outer_locals: &HashMap<String, VarInfo>,
    bctx: &BlockContext,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if name.is_empty() || name.starts_with('_') {
        return;
    }
    if let Some(info) = outer_locals.get(name) {
        if is_different_conditional_branch(
            info,
            bctx.cond_branch,
            bctx.is_in_body,
            bctx.single_stmt,
        ) {
            return;
        }
        if let (Some(var_case), Some(block_case)) =
            (info.when_condition_of_case, bctx.in_when_body_of_case)
        {
            if var_case == block_case {
                return;
            }
        }
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Shadowing outer local variable - `{}`.", name),
        ));
    }
}

/// Collect names from MultiTargetNode entries in block parameters (destructuring).
/// E.g., `|(a, b, c)|` creates a MultiTargetNode with lefts [a, b, c].
fn collect_multi_target_names(node: &ruby_prism::MultiTargetNode<'_>, names: &mut HashSet<String>) {
    for target in node.lefts().iter() {
        if let Some(local) = target.as_required_parameter_node() {
            if let Ok(s) = std::str::from_utf8(local.name().as_slice()) {
                names.insert(s.to_string());
            }
        } else if let Some(inner) = target.as_multi_target_node() {
            collect_multi_target_names(&inner, names);
        }
    }
    if let Some(rest) = node.rest() {
        if let Some(splat) = rest.as_splat_node() {
            if let Some(expr) = splat.expression() {
                if let Some(local) = expr.as_required_parameter_node() {
                    if let Ok(s) = std::str::from_utf8(local.name().as_slice()) {
                        names.insert(s.to_string());
                    }
                }
            }
        }
    }
    for target in node.rights().iter() {
        if let Some(local) = target.as_required_parameter_node() {
            if let Ok(s) = std::str::from_utf8(local.name().as_slice()) {
                names.insert(s.to_string());
            }
        }
    }
}

/// Extract names from multi-target (destructured) params and add to scope.
fn collect_multi_target_names_from_params(
    params: &ruby_prism::ParametersNode<'_>,
    scope: &mut HashMap<String, VarInfo>,
) {
    for p in params.requireds().iter() {
        if let Some(mt) = p.as_multi_target_node() {
            let mut names = HashSet::new();
            collect_multi_target_names(&mt, &mut names);
            for name in names {
                scope.insert(
                    name,
                    VarInfo {
                        conditional_branch: None,
                        when_condition_of_case: None,
                        cond_subsequent_offset: None,
                        is_condition_var: false,
                        is_if_type_cond: false,
                    },
                );
            }
        }
    }

    for p in params.posts().iter() {
        if let Some(mt) = p.as_multi_target_node() {
            let mut names = HashSet::new();
            collect_multi_target_names(&mt, &mut names);
            for name in names {
                scope.insert(
                    name,
                    VarInfo {
                        conditional_branch: None,
                        when_condition_of_case: None,
                        cond_subsequent_offset: None,
                        is_condition_var: false,
                        is_if_type_cond: false,
                    },
                );
            }
        }
    }
}

fn collect_param_names_into(params: &ruby_prism::ParametersNode<'_>, scope: &mut HashSet<String>) {
    for p in params.requireds().iter() {
        if let Some(req) = p.as_required_parameter_node() {
            if let Ok(s) = std::str::from_utf8(req.name().as_slice()) {
                scope.insert(s.to_string());
            }
        }
    }
    for p in params.optionals().iter() {
        if let Some(opt) = p.as_optional_parameter_node() {
            if let Ok(s) = std::str::from_utf8(opt.name().as_slice()) {
                scope.insert(s.to_string());
            }
        }
    }
    if let Some(rest) = params.rest() {
        if let Some(rest_param) = rest.as_rest_parameter_node() {
            if let Some(name) = rest_param.name() {
                if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                    scope.insert(s.to_string());
                }
            }
        }
    }
    for p in params.posts().iter() {
        if let Some(req) = p.as_required_parameter_node() {
            if let Ok(s) = std::str::from_utf8(req.name().as_slice()) {
                scope.insert(s.to_string());
            }
        } else if let Some(kw_rest) = p.as_keyword_rest_parameter_node() {
            if let Some(name) = kw_rest.name() {
                if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                    scope.insert(s.to_string());
                }
            }
        }
    }
    for p in params.keywords().iter() {
        if let Some(kw) = p.as_required_keyword_parameter_node() {
            if let Ok(s) = std::str::from_utf8(kw.name().as_slice()) {
                scope.insert(s.trim_end_matches(':').to_string());
            }
        } else if let Some(kw) = p.as_optional_keyword_parameter_node() {
            if let Ok(s) = std::str::from_utf8(kw.name().as_slice()) {
                scope.insert(s.trim_end_matches(':').to_string());
            }
        }
    }
    if let Some(keyword_rest) = params.keyword_rest() {
        if let Some(keyword_rest) = keyword_rest.as_keyword_rest_parameter_node() {
            if let Some(name) = keyword_rest.name() {
                if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                    scope.insert(s.to_string());
                }
            }
        }
    }
    if let Some(block) = params.block() {
        if let Some(name) = block.name() {
            if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                scope.insert(s.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ShadowingOuterLocalVariable,
        "cops/lint/shadowing_outer_local_variable"
    );
}
