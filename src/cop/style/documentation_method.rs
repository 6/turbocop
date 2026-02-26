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
            // Check standard private/protected detection
            if is_private_or_protected(source, def_offset) {
                return;
            }
        }

        // Check for documentation comment above the def (or modifier) line.
        // has_documentation_comment uses the offset to find the line, then checks
        // the line(s) above it. Since the modifier and def are on the same line,
        // passing the def offset works correctly.
        if has_documentation_comment(source, def_offset) {
            return;
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
