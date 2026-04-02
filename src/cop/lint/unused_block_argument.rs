use crate::cop::variable_force::{self, DeclarationKind, Scope, VariableTable};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for unused block arguments.
///
/// Root causes of historical FPs/FNs (corpus: 144 FP, 937 FN at 89.4% match):
///
/// FN root causes:
/// - Lambda nodes (`-> (x) { ... }`) were not handled; only BlockNode was checked.
/// - Rest/splat parameters (`*args`) were not collected.
/// - Block-local variables (`|x; local|`) were not handled.
/// - `LocalVariableTargetNode` (multi-assign target) was incorrectly treated as
///   "referenced", masking cases where a param was only written but never read.
///
/// FP root causes:
/// - Zero-argument `binding` calls in the block body should suppress all offenses
///   even when they have a receiver (RuboCop's VariableForce treats all accessible
///   args as referenced for any `binding` send without arguments, since it may
///   capture the current local scope).
///
/// Fix: Rewrote to use `check_source` with a visitor that handles both BlockNode
/// and LambdaNode, collects rest params and block-local variables, detects
/// scope-capturing `binding` calls, and only counts actual reads (not write
/// targets) as references.
///
/// ## Corpus investigation (2026-03-11)
///
/// Corpus oracle reported FP=27, FN=5393.
///
/// FN=5393: The `VarRefFinder` used simple name-matching — it collected all
/// `LocalVariableReadNode` names in the block body without considering scope.
/// When a nested block redeclares a parameter with the same name (variable
/// shadowing), reads of that name inside the nested scope were incorrectly
/// counted as references to the outer parameter. Fixed by tracking shadowed
/// names in `VarRefFinder`: when entering a nested block/lambda, any params
/// that shadow outer names are pushed to a `shadowed` list, and reads of
/// those names inside the nested scope are excluded from collection.
///
/// FP=27→24: Operator-assign nodes (`x += 1`, `x ||= val`, `x &&= val`)
/// were not counted as references. Prism represents these as
/// `LocalVariableOperatorWriteNode`, `LocalVariableOrWriteNode`, and
/// `LocalVariableAndWriteNode` — none of which contain a child
/// `LocalVariableReadNode`, so the implicit read was missed. Fixed by
/// adding visit handlers for all three operator-write node types in
/// `VarRefFinder`. Remaining FPs may be from VariableForce sophistication
/// gaps (e.g., scope tracking differences).
///
/// ## Corpus investigation (2026-03-16)
///
/// Corpus oracle reported FP=13, FN=5320 (46.7% match).
///
/// FN=5320 root cause: `BlockVisitor` stopped at `def`/`class`/`module`
/// nodes (line `fn visit_def_node(&mut self, _node) {}`), meaning blocks
/// inside method bodies — which is essentially all real-world Ruby code —
/// were never visited. Fixed by making `BlockVisitor` recurse into
/// `def`/`class`/`module`/`singleton_class` bodies. The scope boundary
/// for variable references is correctly handled by `VarRefFinder`, not
/// `BlockVisitor`, so this is safe.
///
/// FP=13 root cause: `VarRefFinder` didn't recognize `def o.method` as a
/// use of block argument `o`. When a block argument is used as the receiver
/// of a singleton method definition (`def o.to_str; ...; end`), it IS a
/// reference to that variable, but `DefNode` was being skipped entirely by
/// `VarRefFinder`. Fixed by checking the receiver of `DefNode` — if it's a
/// `LocalVariableReadNode`, the name is counted as referenced.
///
/// After fix: FP=0, FN=114 (98.9% match). Remaining 114 FN were from three
/// missing parameter types:
///
/// ## Corpus investigation (2026-03-18)
///
/// FN=114 root causes:
/// - Destructured block params `|(a, b)|` represented as `MultiTargetNode`
///   in `requireds()` / `posts()` were not traversed (~42 FN).
/// - Block-pass params `&block` (`BlockParameterNode`) were not collected (~23 FN).
/// - Keyword rest params `**opts` (`KeywordRestParameterNode`) were not collected (~30 FN).
///   Fixed by adding `collect_multi_target_params` for destructured params, and
///   handling `params.keyword_rest()` and `params.block()` in both param collection
///   and shadowing name collection.
///
/// ## Corpus investigation (2026-03-19)
///
/// FN=3: Lambdas used as default parameter values in method definitions
/// (e.g., `def foo(scope: ->(row) { true })`) were not visited by
/// `BlockVisitor` because `visit_def_node` only recursed into the method
/// body, not the parameters. Lambda nodes in optional parameter defaults
/// are children of `OptionalKeywordParameterNode.value()` or
/// `OptionalParameterNode.value()` under `ParametersNode`. Fixed by
/// adding `self.visit_parameters_node(&params)` in `visit_def_node`.
///
/// ## Corpus investigation (2026-04-02)
///
/// FP=1 root cause: `VarRefFinder` only treated bare `binding` as scope-capturing,
/// but RuboCop's `VariableForce` suppresses unused-argument offenses for any
/// zero-argument `binding` send, including receiver-qualified calls like
/// `tp.binding.local_variable_get(name)`. Fixed by matching zero-arg `binding`
/// calls regardless of receiver, while still leaving `binding(:arg)` as an offense.
///
/// ## Migration to VariableForce
///
/// This cop was migrated from a 729-line standalone AST visitor to use the shared
/// VariableForce engine. The cop implements `VariableForceConsumer::before_leaving_scope`
/// to check each variable in Block scopes for unused arguments. The engine handles
/// binding() detection, scope tracking, shadowing, operator-assign references, and
/// singleton-method receiver references automatically.
pub struct UnusedBlockArgument;

impl Cop for UnusedBlockArgument {
    fn name(&self) -> &'static str {
        "Lint/UnusedBlockArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn as_variable_force_consumer(&self) -> Option<&dyn variable_force::VariableForceConsumer> {
        Some(self)
    }
}

impl variable_force::VariableForceConsumer for UnusedBlockArgument {
    fn before_leaving_scope(
        &self,
        scope: &Scope,
        _variable_table: &VariableTable,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Only check Block scopes (blocks and lambdas)
        if scope.kind != variable_force::ScopeKind::Block {
            return;
        }

        let ignore_empty = config.get_bool("IgnoreEmptyBlocks", true);
        if ignore_empty && scope.body_empty {
            return;
        }

        let allow_unused_keyword = config.get_bool("AllowUnusedKeywordArguments", false);

        for variable in scope.variables.values() {
            // Only check arguments and block-local variables
            if !variable.is_argument() && !variable.is_block_local() {
                continue;
            }

            // Skip underscore-prefixed (intentionally unused)
            if variable.should_be_unused() {
                continue;
            }

            // Skip keyword args when AllowUnusedKeywordArguments is true
            if allow_unused_keyword
                && matches!(
                    variable.declaration_kind,
                    DeclarationKind::KeywordArg
                        | DeclarationKind::OptionalKeywordArg
                        | DeclarationKind::KeywordRestArg
                )
            {
                continue;
            }

            // Block-local variables (ShadowArg) are considered "used" if assigned
            if variable.is_block_local() && !variable.assignments.is_empty() {
                continue;
            }

            // Check if the variable is referenced
            if variable.used() {
                continue;
            }

            let (line, column) = source.offset_to_line_col(variable.declaration_offset);
            let name = String::from_utf8_lossy(&variable.name);
            let display_name = name.trim_end_matches(':');
            let var_type = if variable.is_block_local() {
                "block local variable"
            } else {
                "block argument"
            };
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Unused {var_type} - `{display_name}`."),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnusedBlockArgument, "cops/lint/unused_block_argument");
}
