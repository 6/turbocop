use super::assignment::Assignment;
use super::reference::Reference;
use super::scope::{Scope, ScopeKind};
use super::variable::{DeclarationKind, Variable};

/// Manages the stack of scopes and variable lookup during AST traversal.
///
/// The scope stack grows as the engine enters nested scopes (def, block, class,
/// etc.) and shrinks as it leaves them. Variable lookup walks the stack from
/// innermost to outermost, stopping at hard scope boundaries.
#[derive(Default)]
pub struct VariableTable {
    scope_stack: Vec<Scope>,
}

impl VariableTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new scope onto the stack.
    pub fn push_scope(&mut self, kind: ScopeKind, start_offset: usize, end_offset: usize) {
        self.scope_stack
            .push(Scope::new(kind, start_offset, end_offset));
    }

    /// Pop the innermost scope from the stack and return it.
    pub fn pop_scope(&mut self) -> Scope {
        self.scope_stack.pop().expect("scope stack underflow")
    }

    /// The current (innermost) scope.
    pub fn current_scope(&self) -> &Scope {
        self.scope_stack.last().expect("no current scope")
    }

    /// The current (innermost) scope, mutably.
    pub fn current_scope_mut(&mut self) -> &mut Scope {
        self.scope_stack.last_mut().expect("no current scope")
    }

    /// The depth of the current scope stack (number of scopes).
    pub fn scope_depth(&self) -> usize {
        self.scope_stack.len()
    }

    /// The index of the current scope in the stack.
    pub fn current_scope_index(&self) -> usize {
        self.scope_stack.len().saturating_sub(1)
    }

    /// Declare a variable in the current scope. If the variable already exists
    /// in the current scope, this is a no-op (the existing variable is used).
    /// Returns whether a new variable was created.
    pub fn declare_variable(
        &mut self,
        name: Vec<u8>,
        declaration_offset: usize,
        kind: DeclarationKind,
    ) -> bool {
        let scope_index = self.current_scope_index();
        let scope = self.current_scope_mut();
        if scope.variables.contains_key(&name) {
            return false;
        }
        scope.variables.insert(
            name.clone(),
            Variable::new(name, declaration_offset, kind, scope_index),
        );
        true
    }

    /// Record an assignment to a variable. If the variable doesn't exist in any
    /// accessible scope, it is declared in the current scope first.
    /// If the variable belongs to a different scope than the current one,
    /// `in_branch` is forced to true (block may not execute).
    pub fn assign_to_variable(&mut self, name: &[u8], mut assignment: Assignment) {
        let current_index = self.current_scope_index();
        // Check if variable exists in accessible scopes
        if let Some(var) = self.find_variable_mut(name) {
            if var.scope_index != current_index {
                var.captured_by_block = true;
                assignment.in_branch = true;
            }
            var.assign(assignment);
        } else {
            // New variable — declare in current scope
            let offset = assignment.node_offset;
            self.declare_variable(name.to_vec(), offset, DeclarationKind::Assignment);
            if let Some(var) = self.current_scope_mut().variables.get_mut(name) {
                var.assign(assignment);
            }
        }
    }

    /// Record a reference to a variable. Finds the variable in accessible
    /// scopes and records the reference.
    pub fn reference_variable(&mut self, name: &[u8], reference: Reference) {
        let current_index = self.current_scope_index();
        if let Some(var) = self.find_variable_mut(name) {
            if var.scope_index != current_index {
                var.captured_by_block = true;
            }
            var.reference(reference);
        }
        // If variable not found, it's a reference to an undefined variable
        // (e.g., from eval or dynamic scope). Silently ignore.
    }

    /// Find a variable by name, walking the scope stack from innermost to
    /// outermost. Stops at hard scope boundaries (def, class, module).
    ///
    /// Block scopes (twisted) allow looking through to outer scopes.
    pub fn find_variable(&self, name: &[u8]) -> Option<&Variable> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(var) = scope.variables.get(name) {
                return Some(var);
            }
            // Hard scopes block visibility to outer scopes
            if scope.kind.is_hard() {
                return None;
            }
        }
        None
    }

    /// Mutable version of `find_variable`.
    pub fn find_variable_mut(&mut self, name: &[u8]) -> Option<&mut Variable> {
        for scope in self.scope_stack.iter_mut().rev() {
            if let Some(var) = scope.variables.get_mut(name) {
                return Some(var);
            }
            if scope.kind.is_hard() {
                return None;
            }
        }
        None
    }

    /// Whether a variable with the given name exists in any accessible scope.
    pub fn variable_exists(&self, name: &[u8]) -> bool {
        self.find_variable(name).is_some()
    }

    /// All scopes accessible from the current scope, walking the stack from
    /// innermost to outermost and stopping at hard scope boundaries.
    /// Used by cops that need to check variables across closure chains.
    pub fn accessible_scopes(&self) -> Vec<&Scope> {
        let mut result = Vec::new();
        for scope in self.scope_stack.iter().rev() {
            result.push(scope);
            if scope.kind.is_hard() {
                break;
            }
        }
        result
    }

    /// All variables accessible from the current scope (for `binding()`/`super`
    /// which implicitly reference all accessible variables).
    pub fn accessible_variables_mut(&mut self) -> Vec<&mut Variable> {
        let mut result = Vec::new();
        for scope in self.scope_stack.iter_mut().rev() {
            result.extend(scope.variables.values_mut());
            if scope.kind.is_hard() {
                break;
            }
        }
        result
    }
}
