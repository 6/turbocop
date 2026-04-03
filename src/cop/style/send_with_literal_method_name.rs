use crate::cop::shared::node_type::{CALL_NODE, STRING_NODE, SYMBOL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus FP=9→0: All 9 FPs were `public_send(:[], N)` — operator method names like `[]`, `+`,
/// `*`, etc. were incorrectly treated as valid direct-call targets. RuboCop only considers
/// identifier methods matching `/[a-z_][a-z0-9_]*[!?]?/i` as replaceable. Removed the
/// OPERATOR_METHODS allowlist so operators fall through to the identifier regex (which rejects them).
pub struct SendWithLiteralMethodName;

/// Valid Ruby method name: starts with letter/underscore, contains alphanumerics/underscores,
/// optionally ends with ! or ?
fn is_valid_ruby_method_name(name: &[u8]) -> bool {
    if name.is_empty() {
        return false;
    }

    // Check for reserved words that cannot be used as direct method calls
    const RESERVED_WORDS: &[&[u8]] = &[
        b"BEGIN",
        b"END",
        b"alias",
        b"and",
        b"begin",
        b"break",
        b"case",
        b"class",
        b"def",
        b"defined?",
        b"do",
        b"else",
        b"elsif",
        b"end",
        b"ensure",
        b"false",
        b"for",
        b"if",
        b"in",
        b"module",
        b"next",
        b"nil",
        b"not",
        b"or",
        b"redo",
        b"rescue",
        b"retry",
        b"return",
        b"self",
        b"super",
        b"then",
        b"true",
        b"undef",
        b"unless",
        b"until",
        b"when",
        b"while",
        b"yield",
    ];
    if RESERVED_WORDS.contains(&name) {
        return false;
    }

    // Match /\A[a-zA-Z_][a-zA-Z0-9_]*[!?]?\z/
    let first = name[0];
    if !first.is_ascii_alphabetic() && first != b'_' {
        return false;
    }

    let last = *name.last().unwrap();
    let check_end = if last == b'!' || last == b'?' {
        &name[1..name.len() - 1]
    } else {
        &name[1..]
    };

    check_end
        .iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'_')
}

impl Cop for SendWithLiteralMethodName {
    fn name(&self) -> &'static str {
        "Style/SendWithLiteralMethodName"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE, SYMBOL_NODE]
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
        let allow_send = config.get_bool("AllowSend", true);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();

        // Check for public_send, __send__, or send
        // When AllowSend is true (default), only public_send is flagged.
        // When AllowSend is false, send and __send__ are also flagged.
        let is_target =
            name == b"public_send" || (!allow_send && (name == b"__send__" || name == b"send"));

        if !is_target {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // First argument must be a static symbol or string with a valid Ruby method name.
        // Setter methods (ending in =) can't be converted because behavior differs.
        // Names with special chars (hyphens, dots, brackets, etc.) require send/public_send.
        // Reserved words (class, if, end, etc.) can't be used as direct method calls.
        let is_valid_literal = if let Some(sym) = arg_list[0].as_symbol_node() {
            let name = sym.unescaped();
            !name.ends_with(b"=") && is_valid_ruby_method_name(name)
        } else if let Some(s) = arg_list[0].as_string_node() {
            let content = s.unescaped();
            !content.ends_with(b"=") && is_valid_ruby_method_name(content)
        } else {
            false
        };

        if !is_valid_literal {
            return;
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use a direct method call instead of `send` with a literal method name.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SendWithLiteralMethodName,
        "cops/style/send_with_literal_method_name"
    );
}
