use crate::cop::node_type::DEF_NODE;
use crate::cop::style::documentation::has_documentation_comment;
use crate::cop::util::is_private_or_protected;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Modifiers that wrap a def on the same line but are considered non-public.
const NON_PUBLIC_MODIFIERS: &[&[u8]] = &[b"private_class_method "];

/// Modifiers that wrap a def on the same line but are still public.
/// Documentation should be checked above the modifier line, and the offense
/// reported at the modifier start.
const PUBLIC_MODIFIERS: &[&[u8]] = &[b"module_function ", b"ruby2_keywords "];

/// Style/DocumentationMethod: checks for missing documentation comment on public methods.
///
/// **Investigation (2026-03-08):** 1,768 FPs, 453 FNs at 99.5% match rate.
/// Root cause of FPs: retroactive visibility via `private :method_name` or
/// `protected :method_name` after the def. RuboCop's `VisibilityHelp` mixin checks
/// right-siblings for `(send nil? :private (sym :method_name))`, making the method
/// non-public. nitrocop's `is_private_or_protected` only checked preceding standalone
/// visibility keywords and inline prefixes — it missed the retroactive pattern.
///
/// Fix: Added `has_retroactive_visibility()` which scans lines after the def for
/// `private :sym` / `protected :sym` / `private "str"` patterns with matching method name.
///
/// **Investigation (2026-03-15):** 918 FPs, 508 FNs at 99.7% match rate.
/// Root cause of remaining FPs: three patterns in `is_private_or_protected`:
/// 1. Nested class/module at same indent as `private`/`def` incorrectly reset
///    `in_private` state. E.g., `private; class Inner; end; def method` — the
///    `class Inner` is a peer in the same scope, not a scope boundary. Fix: changed
///    class/module reset from `indent <= def_col` to `indent < def_col` (strictly less).
/// 2. Trailing whitespace on `private` line (e.g., `private \n`) was not matched by
///    bare visibility keyword patterns. Fix: strip trailing whitespace from trimmed lines.
/// 3. `private(def foo)` and `protected(def foo)` — parenthesized form not recognized
///    as inline visibility modifier. Fix: added `private(` / `protected(` patterns to
///    both `is_private_or_protected` and `has_unknown_inline_prefix`.
///
/// Remaining FN gap (508): singleton methods (`def self.foo`, `def obj.bar`) not handled.
/// RuboCop uses `on_defs` (aliased from `on_def`), nitrocop only handles `DefNode`.
///
/// **Investigation (2026-03-18):** 426 FPs, 383 FNs.
/// Root causes of remaining FPs in `is_private_or_protected`:
/// 1. Single-line class/module defs (`class Error < StandardError; end`) at indent ==
///    def_col were incrementing `peer_scope_depth` but never decrementing it (the `end`
///    is on the same line). This caused all subsequent `private` keywords to be ignored.
///    Fix: added `is_single_line_class_or_module()` check — skip peer_scope_depth++ when
///    the class/module opens and closes on the same line.
/// 2. Heredoc content containing `end` at column 0 (e.g., `buf.puts(<<-RUBY)\nend\nRUBY`)
///    was incorrectly treated as a scope boundary, resetting `in_private` state.
///    Fix: added heredoc tracking in `is_private_or_protected` — detect `<<-WORD` / `<<~WORD`
///    patterns and skip all lines until the closing heredoc marker.
/// 3. Some FPs from enclosing class at same indent as def (inconsistent indentation) —
///    inherent limitation of line-based visibility tracking vs RuboCop's AST approach.
///
/// **Heredoc tracking reverted (2026-03-18):** The heredoc tracking added in the previous
/// investigation caused a 20,000+ offense regression. Even with conservative `<<` matching
/// (skip comments, check preceding chars), the fix correctly detected real heredocs but
/// produced worse results: the line-based scanner incidentally processes heredoc content,
/// and `private`/`end` keywords inside heredocs happen to give correct visibility results
/// more often than skipping them. The single-line class/module fix (point 1 above) is
/// retained. A proper fix for heredoc-related FPs requires AST-based visibility tracking.
pub struct DocumentationMethod;

/// Detect if the line containing the def has a modifier prefix before the `def` keyword.
/// Returns `Some((modifier_bytes, indent))` if found, where `indent` is the column of the
/// modifier's first non-whitespace character.
fn detect_inline_modifier(source: &SourceFile, def_offset: usize) -> Option<(&[u8], usize)> {
    let bytes = source.as_bytes();
    // Find the start of the line containing the def
    let mut line_start = def_offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    let line_prefix = &bytes[line_start..def_offset];

    // Compute indent (leading whitespace)
    let indent = line_prefix
        .iter()
        .take_while(|&&b| b == b' ' || b == b'\t')
        .count();
    let trimmed = &line_prefix[indent..];

    // Check all known modifiers
    for modifier in NON_PUBLIC_MODIFIERS.iter().chain(PUBLIC_MODIFIERS.iter()) {
        if trimmed.starts_with(modifier) {
            return Some((modifier, indent));
        }
    }
    None
}

/// Check if the detected modifier is a non-public modifier.
fn is_non_public_modifier(modifier: &[u8]) -> bool {
    NON_PUBLIC_MODIFIERS.contains(&modifier)
}

/// Check if a method is made private/protected retroactively via `private :method_name`
/// or `protected :method_name` appearing after the def in the same scope.
/// This is a common Ruby pattern: `def foo; end; private :foo`
fn has_retroactive_visibility(source: &SourceFile, def_offset: usize, method_name: &str) -> bool {
    let (def_line, def_col) = source.offset_to_line_col(def_offset);
    let lines: Vec<&[u8]> = source.lines().collect();

    // Build target patterns to match: `:method_name` or `"method_name"` or `'method_name'`
    let sym_pattern = format!(":{}", method_name);
    let dq_pattern = format!("\"{}\"", method_name);
    let sq_pattern = format!("'{}'", method_name);

    // Scan lines after the def line for retroactive visibility declarations
    for line in &lines[def_line..] {
        let indent = line
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();
        let trimmed = &line[indent..];

        // Scope boundary at lower indent — stop searching
        if indent < def_col
            && (trimmed.starts_with(b"class ")
                || trimmed.starts_with(b"module ")
                || trimmed == b"end"
                || trimmed.starts_with(b"end ")
                || trimmed.starts_with(b"end\n")
                || trimmed.starts_with(b"end\r"))
        {
            break;
        }

        // Check for `private :method_name` or `protected :method_name` at same or lower indent
        if indent <= def_col {
            let line_str = std::str::from_utf8(trimmed).unwrap_or("");
            let line_str = line_str.trim_end();
            if (line_str.starts_with("private ")
                || line_str.starts_with("private(")
                || line_str.starts_with("protected ")
                || line_str.starts_with("protected("))
                && (line_str.contains(&sym_pattern)
                    || line_str.contains(&dq_pattern)
                    || line_str.contains(&sq_pattern))
            {
                return true;
            }
        }
    }

    false
}

/// Check if the inline prefix before `def` contains a visibility keyword (`private` or
/// `protected`) anywhere in the chain. For example, `memoized internal private def baz`
/// has `private` in the prefix, making the method non-public even though the prefix starts
/// with an unknown modifier.
fn has_inline_visibility_keyword(source: &SourceFile, def_offset: usize) -> bool {
    let bytes = source.as_bytes();
    let mut line_start = def_offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    let line_prefix = &bytes[line_start..def_offset];
    let trimmed: &[u8] = &line_prefix[line_prefix
        .iter()
        .take_while(|&&b| b == b' ' || b == b'\t')
        .count()..];

    if trimmed.is_empty() {
        return false;
    }

    // Check if any word in the prefix is `private` or `protected`
    for word in trimmed.split(|&b| b == b' ' || b == b'\t') {
        if word == b"private" || word == b"protected" {
            return true;
        }
    }

    false
}

/// Check if the `def` has an unknown (non-visibility, non-registered modifier) prefix
/// on the same line. For example: `memoize def foo` or `register_element def bar`.
///
/// RuboCop's AST-based approach associates comments with the wrapping `send` node, not
/// the `def` node, so methods wrapped by unknown method calls are treated as undocumented
/// even when comments exist above the line. We match that behavior.
fn has_unknown_inline_prefix(source: &SourceFile, def_offset: usize) -> bool {
    let bytes = source.as_bytes();
    let mut line_start = def_offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    let line_prefix = &bytes[line_start..def_offset];

    // Strip leading whitespace
    let trimmed: &[u8] = &line_prefix[line_prefix
        .iter()
        .take_while(|&&b| b == b' ' || b == b'\t')
        .count()..];

    // Nothing before def — no prefix
    if trimmed.is_empty() {
        return false;
    }

    // Known visibility keywords that is_private_or_protected already handles
    if trimmed.starts_with(b"private ")
        || trimmed.starts_with(b"private(")
        || trimmed.starts_with(b"protected ")
        || trimmed.starts_with(b"protected(")
        || trimmed.starts_with(b"public ")
        || trimmed.starts_with(b"public(")
    {
        return false;
    }

    // Known modifiers already handled by detect_inline_modifier
    for modifier in NON_PUBLIC_MODIFIERS.iter().chain(PUBLIC_MODIFIERS.iter()) {
        if trimmed.starts_with(modifier) {
            return false;
        }
    }

    // Something else before def — unknown prefix
    true
}

impl Cop for DocumentationMethod {
    fn name(&self) -> &'static str {
        "Style/DocumentationMethod"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let require_for_non_public = config.get_bool("RequireForNonPublicMethods", false);
        let allowed_methods = config.get_string_array("AllowedMethods");

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let method_name = std::str::from_utf8(def_node.name().as_slice()).unwrap_or("");

        // Skip initialize
        if method_name == "initialize" {
            return;
        }

        // Skip allowed methods
        if let Some(ref allowed) = allowed_methods {
            if allowed.iter().any(|m| m == method_name) {
                return;
            }
        }

        let loc = def_node.location();
        let def_offset = loc.start_offset();

        // Detect inline modifier (module_function def, ruby2_keywords def, private_class_method def)
        let modifier = detect_inline_modifier(source, def_offset);

        // Skip private/protected methods unless configured
        if !require_for_non_public {
            // Check if the modifier itself is non-public (private_class_method)
            if let Some((mod_bytes, _)) = modifier {
                if is_non_public_modifier(mod_bytes) {
                    return;
                }
            }
            // Check standard private/protected detection (preceding `private`/`protected`)
            if is_private_or_protected(source, def_offset) {
                return;
            }
            // Check retroactive visibility: `private :method_name` after the def
            if has_retroactive_visibility(source, def_offset, method_name) {
                return;
            }
            // Check if the inline prefix contains `private` or `protected` anywhere
            // in the chain (e.g., `memoized internal private def baz`)
            if has_inline_visibility_keyword(source, def_offset) {
                return;
            }
        }

        // When def is wrapped by an unknown method call (e.g., `memoize def foo`),
        // RuboCop treats the def as undocumented because comments above the line
        // are associated with the wrapping call, not the def node. Skip the
        // documentation check in this case to match RuboCop's behavior.
        if !has_unknown_inline_prefix(source, def_offset) {
            // Check for documentation comment above the def (or modifier) line.
            if has_documentation_comment(source, def_offset) {
                return;
            }
        }

        // Report offense - for modifiers, report at the start of the modifier
        let (line, column) = if let Some((_, indent)) = modifier {
            let (line, _) = source.offset_to_line_col(def_offset);
            (line, indent)
        } else {
            source.offset_to_line_col(def_offset)
        };

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Missing method documentation comment.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DocumentationMethod, "cops/style/documentation_method");
}
