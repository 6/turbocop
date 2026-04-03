use crate::cop::shared::node_type::{
    ASSOC_NODE, CALL_NODE, INTEGER_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for `render` calls that specify body content along with a non-content
/// status code (100-199, 204, 205, 304). Such body content is dropped by the
/// HTTP layer and is never sent to the client.
///
/// ## Investigation findings (2026-03-18)
///
/// 1 FN in the corpus: `render 'foo', status: :continue` style calls where the
/// body content is a positional string/symbol argument rather than a keyword arg.
/// RuboCop's pattern matches both:
///   `(hash <#non_content_status? $(pair (sym BODY_OPTIONS) _) ...>)` -- keyword body
///   `$({str sym} _) (hash <#non_content_status? ...>)`                -- positional body
///
/// Also fixed: offense location. RuboCop reports at the body content option
/// (the pair or positional arg), not at the whole render call.
pub struct UnusedRenderContent;

const NON_CONTENT_SYMBOLS: &[&[u8]] = &[
    b"continue",
    b"switching_protocols",
    b"processing",
    b"no_content",
    b"reset_content",
    b"not_modified",
    b"early_hints",
];

fn is_non_content_code(code: i64) -> bool {
    (100..=199).contains(&code) || code == 204 || code == 205 || code == 304
}

const BODY_OPTIONS: &[&[u8]] = &[
    b"action",
    b"body",
    b"content_type",
    b"file",
    b"html",
    b"inline",
    b"json",
    b"js",
    b"layout",
    b"plain",
    b"raw",
    b"template",
    b"text",
    b"xml",
];

impl Cop for UnusedRenderContent {
    fn name(&self) -> &'static str {
        "Rails/UnusedRenderContent"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ASSOC_NODE,
            CALL_NODE,
            INTEGER_NODE,
            KEYWORD_HASH_NODE,
            SYMBOL_NODE,
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

        if call.name().as_slice() != b"render" {
            return;
        }

        if call.receiver().is_some() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        // Collect arguments as a Vec for easier processing
        let arg_list: Vec<_> = args.arguments().iter().collect();

        // Check for non-content status in keyword hash
        let has_non_content_status = has_non_content_status_in_args(&arg_list);
        if !has_non_content_status {
            return;
        }

        // Case 1: positional string/symbol argument as body content
        // e.g., `render 'foo', status: :continue` or `render :action, status: :no_content`
        // The first positional arg is a Str or Symbol (not a keyword hash) → offense at that arg
        for arg in &arg_list {
            let is_positional_content =
                arg.as_string_node().is_some() || arg.as_symbol_node().is_some();
            if is_positional_content {
                let loc = arg.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(
                    self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not specify body content for a response with a non-content status code"
                            .to_string(),
                    ),
                );
                return;
            }
        }

        // Case 2: keyword body option in hash
        // e.g., `render status: :no_content, json: { error: ... }`
        // Offense at the body option pair (e.g., `json: { error: ... }`)
        for arg in &arg_list {
            let kw = match arg.as_keyword_hash_node() {
                Some(k) => k,
                None => continue,
            };
            for elem in kw.elements().iter() {
                let assoc = match elem.as_assoc_node() {
                    Some(a) => a,
                    None => continue,
                };
                let key = match assoc.key().as_symbol_node() {
                    Some(s) => s,
                    None => continue,
                };
                let key_name = key.unescaped();
                if BODY_OPTIONS.contains(&key_name) {
                    // Report at this pair (key: value), matching RuboCop
                    let loc = assoc.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not specify body content for a response with a non-content status code"
                            .to_string(),
                    ));
                    return;
                }
            }
        }
    }
}

/// Check if the argument list contains a non-content status code.
fn has_non_content_status_in_args(args: &[ruby_prism::Node<'_>]) -> bool {
    for arg in args {
        let kw = match arg.as_keyword_hash_node() {
            Some(k) => k,
            None => continue,
        };
        for elem in kw.elements().iter() {
            let assoc = match elem.as_assoc_node() {
                Some(a) => a,
                None => continue,
            };
            let key = match assoc.key().as_symbol_node() {
                Some(s) => s,
                None => continue,
            };
            if key.unescaped() != b"status" {
                continue;
            }
            // Check symbol status
            if let Some(sym) = assoc.value().as_symbol_node() {
                if NON_CONTENT_SYMBOLS.contains(&sym.unescaped()) {
                    return true;
                }
            }
            // Check numeric status
            if let Some(_int) = assoc.value().as_integer_node() {
                let int_loc = assoc.value().location();
                let code_text = std::str::from_utf8(int_loc.as_slice()).unwrap_or("");
                if let Ok(code_num) = code_text.parse::<i64>() {
                    if is_non_content_code(code_num) {
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
    use crate::testutil::run_cop_full;
    crate::cop_fixture_tests!(UnusedRenderContent, "cops/rails/unused_render_content");

    #[test]
    fn positional_string_with_non_content_status() {
        // render 'foo', status: :continue -- offense at 'foo'
        let source = b"render 'foo', status: :continue\n";
        let diags = run_cop_full(&UnusedRenderContent, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected offense for positional string with non-content status"
        );
        // Column should be at 'foo', not at render
        assert_eq!(
            diags[0].location.column, 7,
            "Offense should be at 'foo' (col 7)"
        );
    }

    #[test]
    fn positional_symbol_with_non_content_status() {
        // render :action_name, status: :no_content -- offense at :action_name
        let source = b"render :action_name, status: :no_content\n";
        let diags = run_cop_full(&UnusedRenderContent, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected offense for positional symbol with non-content status"
        );
        assert_eq!(
            diags[0].location.column, 7,
            "Offense should be at :action_name (col 7)"
        );
    }
}
