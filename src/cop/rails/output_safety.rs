use crate::cop::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_STRING_NODE, STRING_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// FP investigation (2026-03-10): 49 FPs from two root causes:
///
/// 1. **Shallow i18n search**: `contains_i18n_call` only walked the receiver chain
///    (`.receiver()` links) but RuboCop's `def_node_search :i18n_method?` recursively
///    searches ALL descendants of the call node. This missed i18n calls nested in
///    argument positions, e.g. `some_helper(t('key')).html_safe` or
///    `format_text(I18n.t('msg'), 'extra').html_safe`.
///    Fix: rewrote `contains_i18n_call` to do a deep recursive search of the entire
///    node tree (receiver, arguments, block, interpolated string parts).
///
/// 2. **Missing exemptions for `safe_concat`**: the i18n and string-literal receiver
///    checks were only applied to `html_safe` and `raw`, but RuboCop applies them
///    at the top of `on_send` before method-specific checks. This caused FPs on
///    `"str".safe_concat(x)` and `out.safe_concat(t('key'))`.
///    Fix: moved exemption checks to the top of `check_node`, before method dispatch.
///
/// ## Investigation (2026-03-14): 24 FPs
///
/// **FP root cause**: `contains_i18n_call` used manual recursion that explicitly handled
/// CallNode, ParenthesesNode, and InterpolatedStringNode — but MISSED KeywordHashNode,
/// HashNode, BlockNode bodies, and other container types. When an i18n call appeared
/// inside a hash argument (e.g., `raw(cell(..., context: { placeholder: t(...) }))`) or
/// inside a block body (e.g., `"#{render(...) { I18n.t(...) }}".html_safe`), the manual
/// recursion didn't descend into those nodes, so the i18n suppression was missed.
///
/// Fix: Replaced manual recursion with a `Visit`-based subtree walker (`I18nSearcher`)
/// that uses `ruby_prism::visit_call_node` for automatic complete traversal, matching
/// RuboCop's `def_node_search :i18n_method?` behavior.
pub struct OutputSafety;

const I18N_METHODS: &[&[u8]] = &[b"t", b"translate", b"l", b"localize"];

/// Check if the receiver is a non-interpolated string literal.
fn is_non_interpolated_string(receiver: &ruby_prism::Node<'_>) -> bool {
    if receiver.as_string_node().is_some() {
        return true;
    }
    // Interpolated string where all parts are string literals (adjacent string concatenation)
    if let Some(dstr) = receiver.as_interpolated_string_node() {
        return dstr
            .parts()
            .iter()
            .all(|part| part.as_string_node().is_some());
    }
    false
}

/// Check if a single call node is an i18n method call.
/// Matches: t(), translate(), l(), localize(), I18n.t(), I18n.translate(), etc.
fn is_i18n_call(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    if !I18N_METHODS.contains(&name) {
        return false;
    }
    // No receiver (bare t/translate/l/localize)
    if call.receiver().is_none() {
        return true;
    }
    if let Some(recv) = call.receiver() {
        if recv
            .as_constant_read_node()
            .is_some_and(|c| c.name().as_slice() == b"I18n")
        {
            return true;
        }
        if recv
            .as_constant_path_node()
            .is_some_and(|cp| cp.name().is_some_and(|n| n.as_slice() == b"I18n"))
        {
            return true;
        }
    }
    false
}

/// Deep recursive search for any i18n method call in the entire node tree.
/// Matches RuboCop's `def_node_search :i18n_method?` which searches ALL descendants
/// (including nodes inside keyword hash arguments, block bodies, etc.).
///
/// The previous manual recursion missed KeywordHashNode/HashNode/BlockNode, causing FPs
/// when i18n calls appeared inside hash arguments (e.g., `t(...)` in `context: {placeholder: t(...)}`).
/// Using a Visit-based traversal ensures complete coverage like RuboCop's node_search.
fn contains_i18n_call(node: &ruby_prism::Node<'_>) -> bool {
    struct I18nSearcher {
        found: bool,
    }

    impl<'pr> Visit<'pr> for I18nSearcher {
        fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
            if self.found {
                return;
            }
            if is_i18n_call(node) {
                self.found = true;
                return;
            }
            ruby_prism::visit_call_node(self, node);
        }
    }

    let mut searcher = I18nSearcher { found: false };
    searcher.visit(node);
    searcher.found
}

impl Cop for OutputSafety {
    fn name(&self) -> &'static str {
        "Rails/OutputSafety"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            INTERPOLATED_STRING_NODE,
            STRING_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();

        // RuboCop applies these exemptions before method-specific checks:
        // 1. Non-interpolated string literal receiver (html_safe/safe_concat)
        // 2. i18n method call anywhere in the node tree
        if let Some(ref receiver) = call.receiver() {
            if is_non_interpolated_string(receiver) {
                return;
            }
        }
        if contains_i18n_call(node) {
            return;
        }

        if name == b"html_safe" {
            // Must have a receiver
            if call.receiver().is_none() {
                return;
            }
            // No arguments allowed for html_safe
            if call.arguments().is_some() {
                return;
            }
        } else if name == b"raw" {
            // raw() must be called without a receiver (command style)
            if call.receiver().is_some() {
                return;
            }
            // Must have exactly one argument
            let args = match call.arguments() {
                Some(a) => a,
                None => return,
            };
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 1 {
                return;
            }
        } else if name == b"safe_concat" {
            // Must have exactly one argument
            let args = match call.arguments() {
                Some(a) => a,
                None => return,
            };
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 1 {
                return;
            }
        } else {
            return;
        }

        // Use message_loc to point to the method name (html_safe/raw/safe_concat)
        // instead of the entire call expression, matching RuboCop's `node.loc.selector`.
        let loc = call.message_loc().unwrap_or(node.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Tagging a string as html safe may be a security risk.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OutputSafety, "cops/rails/output_safety");
}
