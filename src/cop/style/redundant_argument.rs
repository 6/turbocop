use crate::cop::shared::node_type::{CALL_NODE, FALSE_NODE, INTEGER_NODE, STRING_NODE, TRUE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for redundant arguments passed to certain methods where the argument
/// matches the method's default value.
///
/// ## Corpus investigation findings (94 FP, 93 FN, symmetric):
/// All FP/FN pairs were caused by reporting the offense at `call.location()`
/// (start of entire call expression including receiver) instead of at the
/// argument range. For multiline receivers (e.g., `[...].join("")`), the
/// receiver spans multiple lines, so the offense was reported on the wrong
/// line entirely. RuboCop reports at `opening_loc` (the `(`) for parenthesized
/// calls, or at the argument node for unparenthesized calls.
///
/// Fix: use `call.opening_loc()` to get the `(` position for parenthesized
/// calls, falling back to `arg.location()` for unparenthesized calls.
///
/// ## Additional fix: block argument FP (1 FP):
/// `"a b".split(" ", &proc {})` was flagged as redundant because Prism stores
/// `&expr` block arguments in `call.block()` rather than in `arguments()`, so
/// `arg_list.len() == 1` and the cop only saw `" "`. Added early return when
/// the block is a `BlockArgumentNode` since a block-pass argument changes method
/// semantics.
///
/// ## Fix: block literal FN (7 FN):
/// The original block check (`call.block().is_some()`) was too broad — it also
/// skipped literal blocks (`{ }` / `do..end`), but those don't make the
/// positional argument non-redundant (e.g., `sum(0) { |x| x }` still has
/// redundant `0`). Narrowed the check to only skip `BlockArgumentNode` (`&expr`).
pub struct RedundantArgument;

impl Cop for RedundantArgument {
    fn name(&self) -> &'static str {
        "Style/RedundantArgument"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, FALSE_NODE, INTEGER_NODE, STRING_NODE, TRUE_NODE]
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
        let _methods = config.get_string_hash("Methods");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        // If the call has a block-pass argument (&proc, &block), the default
        // argument is not redundant because the block-pass changes method semantics.
        // However, a literal block ({ } or do..end) does NOT affect redundancy
        // of the positional argument — `sum(0) { |x| x }` still has redundant `0`.
        if let Some(block) = call.block() {
            if block.as_block_argument_node().is_some() {
                return;
            }
        }

        let arg = &arg_list[0];

        if arg.as_block_argument_node().is_some() {
            return;
        }

        // RuboCop skips receiverless calls (except exit/exit!) because `split(" ")`
        // without an explicit receiver may be a different method than String#split.
        if call.receiver().is_none() && method_bytes != b"exit" && method_bytes != b"exit!" {
            return;
        }

        // Default redundant arguments
        let redundant = match method_bytes {
            b"join" => self.is_string_value(arg, source, ""),
            b"sum" => self.is_integer_value(arg, 0),
            b"exit" => self.is_boolean_value(arg, true),
            b"exit!" => self.is_boolean_value(arg, false),
            b"split" => self.is_string_value(arg, source, " "),
            b"chomp" | b"chomp!" => self.is_string_value(arg, source, "\n"),
            b"to_i" => self.is_integer_value(arg, 10),
            _ => false,
        };

        if redundant {
            let arg_src = std::str::from_utf8(arg.location().as_slice()).unwrap_or("");

            // Report at the argument range, matching RuboCop's behavior:
            // - Parenthesized: span from opening paren to closing paren, e.g. ('')
            // - Non-parenthesized: span just the argument
            let offset = if let Some(open) = call.opening_loc() {
                open.start_offset()
            } else {
                arg.location().start_offset()
            };

            let (line, column) = source.offset_to_line_col(offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Argument `{arg_src}` is redundant because it is implied by default."),
            ));
        }
    }
}

impl RedundantArgument {
    fn is_string_value(
        &self,
        node: &ruby_prism::Node<'_>,
        _source: &SourceFile,
        expected: &str,
    ) -> bool {
        if let Some(str_node) = node.as_string_node() {
            let content = str_node.unescaped();
            return content == expected.as_bytes();
        }
        false
    }

    fn is_integer_value(&self, node: &ruby_prism::Node<'_>, expected: i64) -> bool {
        if let Some(int_node) = node.as_integer_node() {
            // Prism's IntegerNode value
            let flags = int_node.flags();
            let src = int_node.location().as_slice();
            if let Ok(s) = std::str::from_utf8(src) {
                if let Ok(val) = s.parse::<i64>() {
                    return val == expected;
                }
            }
            let _ = flags;
        }
        false
    }

    fn is_boolean_value(&self, node: &ruby_prism::Node<'_>, expected: bool) -> bool {
        if expected {
            node.as_true_node().is_some()
        } else {
            node.as_false_node().is_some()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantArgument, "cops/style/redundant_argument");
}
