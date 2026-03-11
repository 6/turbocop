use crate::cop::node_type::{CALL_NODE, STRING_NODE, SYMBOL_NODE};
use crate::cop::util::keyword_arg_value;
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

        // First argument should be a symbol or string (association name)
        let first_arg = match args.arguments().iter().next() {
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
            let mut expected = assoc_name;
            expected.extend_from_slice(b"_id");
            expected
        } else {
            // has_many/has_one/has_and_belongs_to_many:
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
                // Derive from enclosing class name
                let class_name = match crate::schema::find_enclosing_class_name(
                    source.as_bytes(),
                    call.location().start_offset(),
                    parse_result,
                ) {
                    Some(n) => n,
                    None => return, // Not inside a class, can't determine default FK
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
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Redundant `foreign_key` -- it matches the default.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantForeignKey, "cops/rails/redundant_foreign_key");
}
