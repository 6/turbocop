use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/I18nLocaleTexts: Enforces use of I18n and locale files instead of locale-specific strings.
///
/// ## Investigation (2026-03-15, updated 2026-03-16)
///
/// **FP root cause (prior, fixed 2026-03-15):** Local variable `flash` was being matched.
/// RuboCop's pattern only matches `(send nil? :flash)` (method call), NOT `(lvar :flash)`.
/// Fixed by removing local variable `flash` handling from `is_flash_receiver`.
///
/// **FN root cause (8 FNs, fixed 2026-03-15):** Recursive AST search was missing several
/// nesting patterns. Fixed by implementing `find_pairs_recursive`.
///
/// **FN root cause (1 FN, fixed 2026-03-16):** `redirect_to` with `**` splatted ternary:
/// `redirect_to path, **(cond ? { alert: "..." } : { warning: "..." })`. The `**` creates
/// `AssocSplatNode` -> `ParenthesesNode` -> `StatementsNode` -> `IfNode` -> `ElseNode` ->
/// `HashNode`. Extended `find_pairs_recursive` to handle `AssocSplatNode`, `IfNode`,
/// `ElseNode`, `ParenthesesNode`, and `StatementsNode`.
///
/// **FP root cause (3 FPs, fixed 2026-03-18):** Multi-line string literals in `flash[:error]`
/// assignments. The Parser gem (used by RuboCop) parses multi-line string literals without
/// interpolation as `dstr` (dynamic string) nodes, so RuboCop's `$str` NodePattern does not
/// match them. Prism correctly parses them as `StringNode`. Fixed by checking whether the
/// string node spans multiple source lines and excluding multi-line strings.
///
/// **FN root cause (1 FN, fixed 2026-03-18):** Backslash (`\`) line continuation strings
/// span multiple physical source lines but do NOT contain actual newline characters in
/// their content. The Parser gem treats these as plain `str` nodes (not `dstr`), so
/// RuboCop's `$str` pattern matches them. Fixed by checking unescaped string content for
/// `\n` instead of comparing source line numbers — this correctly excludes strings with
/// real newlines while including `\` continuation strings.
///
/// **FP root cause (2 FPs, fixed 2026-03-23):** `flash(:category)[:key] = 'string'`
/// from the Scorched framework was being flagged. RuboCop's pattern `(send nil? :flash)`
/// only matches `flash` with NO arguments. Scorched's `flash(:animals)` passes a
/// category argument. Fixed by adding `call.arguments().is_none()` check in
/// `is_flash_receiver`.
///
/// **FN root cause (1 FN, fixed 2026-03-23):** Single-line strings with `\r\n` escape
/// sequences (e.g. `"Thank you!\r\nMore text"`) contain `\n` in unescaped content but
/// are single-line in source. The Parser gem treats them as `str` (not `dstr`). The
/// previous `is_string_literal` check rejected any string with `\n` in content. Fixed
/// by only excluding strings that BOTH span multiple source lines AND contain `\n` in
/// their unescaped content.
pub struct I18nLocaleTexts;

const MSG: &str = "Move locale texts to the locale files in the `config/locales` directory.";

/// Check if a node is a plain string literal (not a symbol, not interpolated) that
/// the Parser gem would classify as a `str` (not `dstr`).
///
/// The Parser gem treats multi-line string literals (those with real newline characters
/// in the source) as `dstr` nodes even without interpolation, so RuboCop's `$str`
/// NodePattern does not match them. However:
/// - `\` line continuation strings span multiple source lines but do NOT have newlines
///   in their unescaped content — Parser treats them as `str`, so we flag them.
/// - Single-line strings with `\n` or `\r\n` escape sequences have newlines in their
///   unescaped content but NOT in the source — Parser treats them as `str`, so we flag them.
///
/// The correct check is: exclude only when the string BOTH spans multiple source lines
/// AND has newlines in its unescaped content.
fn is_string_literal(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    if let Some(s) = node.as_string_node() {
        let has_newline_in_content = s.unescaped().contains(&b'\n');
        if has_newline_in_content {
            // Only exclude if the string also spans multiple source lines.
            // Single-line strings with \n escape sequences should still be flagged.
            let loc = s.location();
            let (start_line, _) = source.offset_to_line_col(loc.start_offset());
            let (end_line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
            if start_line != end_line {
                return false;
            }
        }
        return true;
    }
    false
}

/// Recursively search a node's subtree for `(pair (sym :key) str)` patterns.
/// Mirrors RuboCop's `def_node_search` which walks the entire AST subtree.
fn find_pairs_recursive<'a>(
    source: &SourceFile,
    node: &ruby_prism::Node<'a>,
    key: &[u8],
    results: &mut Vec<ruby_prism::Node<'a>>,
) {
    // Check if this node is an assoc (pair) with matching key and string value
    if let Some(assoc) = node.as_assoc_node() {
        if let Some(sym) = assoc.key().as_symbol_node() {
            if sym.unescaped() == key && is_string_literal(source, &assoc.value()) {
                results.push(assoc.value());
                return; // Don't recurse further into this pair
            }
        }
        // Recurse into assoc value (could contain nested hashes)
        find_pairs_recursive(source, &assoc.value(), key, results);
        return;
    }

    // KeywordHashNode: recurse into elements
    if let Some(kw) = node.as_keyword_hash_node() {
        for elem in kw.elements().iter() {
            find_pairs_recursive(source, &elem, key, results);
        }
        return;
    }

    // HashNode: recurse into elements
    if let Some(hash) = node.as_hash_node() {
        for elem in hash.elements().iter() {
            find_pairs_recursive(source, &elem, key, results);
        }
        return;
    }

    // CallNode: recurse into receiver and arguments
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            find_pairs_recursive(source, &recv, key, results);
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                find_pairs_recursive(source, &arg, key, results);
            }
        }
        return;
    }

    // ArgumentsNode: recurse into each argument
    if let Some(args) = node.as_arguments_node() {
        for arg in args.arguments().iter() {
            find_pairs_recursive(source, &arg, key, results);
        }
        return;
    }

    // AssocSplatNode (**expr): recurse into the splatted expression
    if let Some(splat) = node.as_assoc_splat_node() {
        if let Some(value) = splat.value() {
            find_pairs_recursive(source, &value, key, results);
        }
        return;
    }

    // IfNode (ternary `cond ? then : else`): recurse into both branches
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            for stmt in stmts.body().iter() {
                find_pairs_recursive(source, &stmt, key, results);
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            find_pairs_recursive(source, &subsequent, key, results);
        }
        return;
    }

    // ElseNode: recurse into statements
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            for stmt in stmts.body().iter() {
                find_pairs_recursive(source, &stmt, key, results);
            }
        }
        return;
    }

    // ParenthesesNode: recurse into body
    if let Some(parens) = node.as_parentheses_node() {
        if let Some(body) = parens.body() {
            find_pairs_recursive(source, &body, key, results);
        }
        return;
    }

    // StatementsNode: recurse into each statement
    if let Some(stmts) = node.as_statements_node() {
        for stmt in stmts.body().iter() {
            find_pairs_recursive(source, &stmt, key, results);
        }
    }
}

impl Cop for I18nLocaleTexts {
    fn name(&self) -> &'static str {
        "Rails/I18nLocaleTexts"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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

        let method_name = call.name().as_slice();

        match method_name {
            b"validates" => {
                // Recursively search for (pair (sym :message) str) anywhere in args
                let mut results = Vec::new();
                find_pairs_recursive(source, node, b"message", &mut results);
                for val in results {
                    let loc = val.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
                }
                return;
            }
            b"redirect_to" | b"redirect_back" => {
                // Recursively search for (pair (sym :notice/:alert) str) anywhere in args
                for key in &[b"notice" as &[u8], b"alert"] {
                    let mut results = Vec::new();
                    find_pairs_recursive(source, node, key, &mut results);
                    for val in results {
                        let loc = val.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
                    }
                }
                return;
            }
            b"mail" => {
                // Recursively search for (pair (sym :subject) str) anywhere in args
                let mut results = Vec::new();
                find_pairs_recursive(source, node, b"subject", &mut results);
                for val in results {
                    let loc = val.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
                }
            }
            _ => {}
        }

        // Check flash[:notice] = "string" or flash.now[:notice] = "string"
        // This is `[]=` call on `flash` or `flash.now`
        if method_name == b"[]=" {
            if let Some(receiver) = call.receiver() {
                let is_flash = is_flash_receiver(&receiver);
                if is_flash {
                    // The last argument is the assigned value
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if arg_list.len() == 2 && is_string_literal(source, &arg_list[1]) {
                            let loc = arg_list[1].location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                MSG.to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Check if a node is `flash` or `flash.now` (method call only, not local variable).
/// RuboCop's pattern matches `(send nil? :flash)` and `(send (send nil? :flash) :now)`,
/// which only matches `flash` as a method call with no receiver AND no arguments.
/// `flash(:category)` (e.g. Scorched framework) has arguments and should not match.
fn is_flash_receiver(node: &ruby_prism::Node<'_>) -> bool {
    // Direct `flash` call (no receiver, no arguments)
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"flash"
            && call.receiver().is_none()
            && call.arguments().is_none()
        {
            return true;
        }
        // `flash.now`
        if call.name().as_slice() == b"now" {
            if let Some(recv) = call.receiver() {
                if let Some(inner_call) = recv.as_call_node() {
                    if inner_call.name().as_slice() == b"flash"
                        && inner_call.receiver().is_none()
                        && inner_call.arguments().is_none()
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(I18nLocaleTexts, "cops/rails/i18n_locale_texts");
}
