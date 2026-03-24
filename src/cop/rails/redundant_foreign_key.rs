use crate::cop::node_type::{CALL_NODE, STRING_NODE, SYMBOL_NODE};
use crate::cop::util::{keyword_arg_pair_start_offset, keyword_arg_value};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Detects cases where the `:foreign_key` option on associations is redundant.
///
/// ## Investigation findings (2026-03-10)
///
/// **Root causes of FN (100):**
/// - Missing `has_many`, `has_one`, and `has_and_belongs_to_many` support. RuboCop handles all
///   four association types. For `has_*` associations, the default FK is `{model_name}_id`
///   (derived from the enclosing class name via snake_case), not `{assoc_name}_id`.
/// - When `has_*` has an `:as` option (polymorphic), the default FK is `{as_value}_id`.
/// - Missing string association name support (`belongs_to "user"` vs `belongs_to :user`).
///
/// **Root causes of FP (14):**
/// - Not a `class_name` issue: RuboCop's `belongs_to` FK default is always `{assoc_name}_id`
///   regardless of `class_name`. The FPs were likely from `has_*` being incorrectly matched
///   by other means, or edge cases in the old implementation. The `class_name` option does NOT
///   change the default FK for `belongs_to`.
///
/// **Fixes applied:**
/// - Added `has_many`, `has_one`, `has_and_belongs_to_many` support using
///   `find_enclosing_class_name` + `camel_to_snake` for model-based FK derivation.
/// - Added `:as` option handling for polymorphic `has_*` associations.
/// - Added string association name support for `belongs_to`.
/// - For `has_*` outside a class context, the cop correctly skips (no model name to derive FK).
///
/// ## Investigation findings (2026-03-14)
///
/// **FP root cause (89 FPs):** RuboCop's node_matcher pattern
/// `(send nil? method ({sym str} name) $(hash <...>))` strictly matches
/// 2-argument calls (name + options_hash). When a scope lambda/proc is present
/// (e.g., `has_many :hard_disks, -> { ... }, class_name: "Disk", foreign_key: :hardware_id`),
/// the call has 3 arguments and the pattern does NOT match — RuboCop skips it.
/// Nitrocop's `keyword_arg_value` searched through ALL arguments regardless,
/// causing false positives. Fixed by checking for non-hash intermediate arguments
/// (scope lambdas) and returning early when found.
///
/// ## Investigation findings (2026-03-15, round 2)
///
/// **FP root cause (1 FP):** `has_many` with a trailing `do...end` block was
/// flagged but RuboCop skips it. RuboCop's node_matcher pattern
/// `(send nil? method ({sym str} name) $(hash <...>))` doesn't match calls
/// that have a trailing block in the Parser AST.
/// Fix: skip when `call.block().is_some()`.
///
/// ## Investigation findings (2026-03-15)
///
/// **FP+FN root cause (51 FP, 50 FN):** Symmetric per-repo — every FP had a
/// corresponding FN at line+1 in the same file. Root cause was reporting the
/// diagnostic at the CallNode's start (the `has_many`/`belongs_to` keyword) instead
/// of the `foreign_key:` AssocNode pair. For multiline associations like:
/// ```ruby
/// has_many :items,           # line N — nitrocop reported here (FP, wrong line)
///   foreign_key: :model_id   # line N+1 — RuboCop reports here (FN, expected)
/// ```
/// Fixed by using `keyword_arg_pair_start_offset` to locate the `foreign_key:` key
/// and reporting at that position. Also updated the message to match RuboCop's
/// "Specifying the default value for `foreign_key` is redundant."
///
/// ## Investigation findings (2026-03-23)
///
/// **FP root cause (3 FPs):** RuboCop's `parent_module_name` returns `nil` when
/// ANY non-`class_eval` block is in the ancestor chain of the send node. This
/// means `has_*` inside `with_options do...end`, classes defined inside RSpec
/// blocks, or any other non-class_eval block causes RuboCop to skip the check.
/// Nitrocop's `find_enclosing_class_name` traversed through blocks to find the
/// class, producing false positives.
/// Fix: replaced `find_enclosing_class_name` with `find_parent_module_name` which
/// replicates RuboCop's block-respecting ancestor traversal.
///
/// **FN root cause (2 FNs):**
/// 1. `belongs_to` with trailing `do...end` block: the blanket `call.block().is_some()`
///    skip applied to all associations, but RuboCop only skips `has_*` with trailing
///    blocks (for `belongs_to`, the FK is derived from the association name, not the
///    class, so blocks don't interfere).
///    Fix: moved the trailing block check to apply only to `has_*` associations.
/// 2. `has_many` inside `ClassName.class_eval do...end`: RuboCop's `parent_module_name`
///    resolves the class from the `class_eval` receiver when it's a constant.
///    Fix: `find_parent_module_name` now handles `class_eval` on constant receivers.
pub struct RedundantForeignKey;

impl Cop for RedundantForeignKey {
    fn name(&self) -> &'static str {
        "Rails/RedundantForeignKey"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name();
        let method_name_bytes = method_name.as_slice();
        let is_belongs_to = method_name_bytes == b"belongs_to";
        let is_has_association = method_name_bytes == b"has_many"
            || method_name_bytes == b"has_one"
            || method_name_bytes == b"has_and_belongs_to_many";

        if !is_belongs_to && !is_has_association {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args_list: Vec<_> = args.arguments().iter().collect();

        // First argument should be a symbol or string (association name)
        let first_arg = match args_list.first() {
            Some(a) => a,
            None => return,
        };
        let assoc_name = if let Some(s) = first_arg.as_symbol_node() {
            s.unescaped().to_vec()
        } else if let Some(s) = first_arg.as_string_node() {
            s.unescaped().to_vec()
        } else {
            return;
        };

        // RuboCop's node pattern `(send nil? method ({sym str} name) hash)` only matches
        // 2-argument calls (name + options_hash). If a scope lambda/proc is present as
        // the second argument (e.g., `has_many :posts, -> { scope }, foreign_key: :x`),
        // RuboCop skips the check. We replicate this by checking for non-hash/non-keyword-hash
        // intermediate arguments.
        if args_list.len() > 2 {
            // Check if any argument between first and last is not a hash
            for arg in args_list[1..args_list.len() - 1].iter() {
                if arg.as_hash_node().is_none() && arg.as_keyword_hash_node().is_none() {
                    return; // Scope lambda or other non-hash argument present
                }
            }
        }

        // Check for foreign_key keyword arg
        let fk_value = match keyword_arg_value(&call, b"foreign_key") {
            Some(v) => v,
            None => return,
        };

        // foreign_key can be a symbol or string
        let fk_name = if let Some(sym) = fk_value.as_symbol_node() {
            sym.unescaped().to_vec()
        } else if let Some(s) = fk_value.as_string_node() {
            s.unescaped().to_vec()
        } else {
            return;
        };

        // Build expected default FK
        let expected = if is_belongs_to {
            // belongs_to: default FK is {assoc_name}_id
            // belongs_to with trailing blocks is still flagged by RuboCop
            // (the FK is derived from the assoc name, not the class name).
            let mut expected = assoc_name;
            expected.extend_from_slice(b"_id");
            expected
        } else {
            // has_many/has_one/has_and_belongs_to_many:
            // In RuboCop's Parser AST, a trailing do...end block wraps the send
            // node, making the block an ancestor. parent_module_name then returns
            // nil for the block (since it's not class_eval), so RuboCop skips it.
            // For the :as case, parent_module_name is never called, but the
            // trailing block still prevents the node_matcher from matching in
            // the Parser AST (the block node replaces the send node at that
            // position). So we skip all has_* with trailing blocks.
            if call.block().is_some() {
                return;
            }

            // If :as option is present, default FK is {as_value}_id
            // Otherwise, default FK is {snake_case(model_name)}_id
            if let Some(as_value) = keyword_arg_value(&call, b"as") {
                let as_name = if let Some(sym) = as_value.as_symbol_node() {
                    sym.unescaped().to_vec()
                } else if let Some(s) = as_value.as_string_node() {
                    s.unescaped().to_vec()
                } else {
                    return;
                };
                let mut expected = as_name;
                expected.extend_from_slice(b"_id");
                expected
            } else {
                // Derive from enclosing class/module name, respecting block boundaries.
                // RuboCop's parent_module_name returns nil when any non-class_eval block
                // is in the ancestor chain (e.g., with_options do...end, RSpec it blocks).
                // For ClassName.class_eval do...end, it uses the constant receiver name.
                let class_name = match crate::schema::find_parent_module_name(
                    source.as_bytes(),
                    call.location().start_offset(),
                    parse_result,
                ) {
                    Some(n) => n,
                    None => return, // Not inside a class or blocked by a block
                };
                // Use the last segment for namespaced classes (Foo::Bar -> Bar)
                let last_segment = class_name.rsplit("::").next().unwrap_or(&class_name);
                let snake = crate::schema::camel_to_snake(last_segment);
                let mut expected = snake.into_bytes();
                expected.extend_from_slice(b"_id");
                expected
            }
        };

        if fk_name == expected {
            // Report at the `foreign_key:` pair location, not the call start.
            // RuboCop annotates on the `foreign_key: value` pair, which matters
            // for multiline associations where the pair is on a different line.
            let offset = keyword_arg_pair_start_offset(&call, b"foreign_key")
                .unwrap_or_else(|| node.location().start_offset());
            let (line, column) = source.offset_to_line_col(offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Specifying the default value for `foreign_key` is redundant.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantForeignKey, "cops/rails/redundant_foreign_key");
}
