//! Shared variable dataflow analysis engine.
//!
//! This module provides a VariableForce engine that performs a single AST walk
//! to build a complete variable-scope model, then delivers finished analysis
//! to cops through hook callbacks. This mirrors RuboCop's VariableForce
//! architecture but is adapted for Rust's ownership model and nitrocop's
//! `!Send` ParseResult constraint.
//!
//! ## Architecture
//!
//! - **Engine**: A Prism visitor that walks the AST, maintaining a VariableTable.
//! - **VariableTable**: Scope stack + variable lookup, respecting Ruby's scoping
//!   rules (hard vs twisted scope boundaries).
//! - **Consumers**: Cops implement `VariableForceConsumer` to receive hook
//!   callbacks at scope entry/exit and variable declaration events.
//!
//! ## Usage
//!
//! Cops opt into VariableForce by implementing `VariableForceConsumer` and
//! overriding `Cop::as_variable_force_consumer()`. The linter runs the engine
//! once per file when any active cop is a consumer, replacing up to 10
//! separate per-cop AST walks with a single shared traversal.

pub mod assignment;
pub mod engine;
pub mod reference;
pub mod scope;
pub mod variable;
pub mod variable_table;

pub use assignment::{Assignment, AssignmentKind};
pub use reference::Reference;
pub use scope::{Scope, ScopeKind};
pub use variable::{DeclarationKind, Variable};
pub use variable_table::VariableTable;

use crate::cop::CopConfig;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Trait for cops that consume VariableForce analysis.
///
/// Cops implement only the hooks they need — all have default no-op
/// implementations. This trait is separate from `Cop` to avoid adding
/// methods to all 900+ cops. Cops opt in by implementing both `Cop` and
/// `VariableForceConsumer`, then overriding `Cop::as_variable_force_consumer()`.
///
/// Hook methods receive the scope/variable being processed, the full
/// VariableTable for context, and the source/config/diagnostics needed to
/// emit offenses.
#[allow(unused_variables)]
pub trait VariableForceConsumer: Send + Sync {
    /// Called before entering a new scope.
    fn before_entering_scope(
        &self,
        scope: &Scope,
        variable_table: &VariableTable,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
    }

    /// Called after entering a new scope (scope is now on the stack).
    fn after_entering_scope(
        &self,
        scope: &Scope,
        variable_table: &VariableTable,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
    }

    /// Called before leaving a scope (scope is still on the stack).
    /// This is the most commonly used hook — iterate `scope.variables` to
    /// analyze variable lifetime within the scope being exited.
    fn before_leaving_scope(
        &self,
        scope: &Scope,
        variable_table: &VariableTable,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
    }

    /// Called after leaving a scope (scope has been popped from the stack).
    fn after_leaving_scope(
        &self,
        scope: &Scope,
        variable_table: &VariableTable,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
    }

    /// Called before declaring a variable in the current scope.
    /// Used by ShadowingOuterLocalVariable to check if the name shadows
    /// a variable from an outer scope.
    fn before_declaring_variable(
        &self,
        variable: &Variable,
        variable_table: &VariableTable,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
    }

    /// Called after declaring a variable in the current scope.
    fn after_declaring_variable(
        &self,
        variable: &Variable,
        variable_table: &VariableTable,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
    }
}
