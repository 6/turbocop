use crate::cop::shared::node_type::{ARRAY_NODE, CALL_NODE, STRING_NODE, SYMBOL_NODE};
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

fn is_single_chain_argument(arg: &ruby_prism::Node<'_>) -> bool {
    if arg.as_symbol_node().is_some() {
        true
    } else if let Some(s) = arg.as_string_node() {
        !s.unescaped().contains(&b'.')
    } else if let Some(arr) = arg.as_array_node() {
        arr.elements().iter().count() == 1
    } else {
        let elements: Vec<_> = if let Some(hash) = arg.as_hash_node() {
            hash.elements().iter().collect()
        } else if let Some(hash) = arg.as_keyword_hash_node() {
            hash.elements().iter().collect()
        } else {
            return false;
        };

        elements.len() == 1 && elements[0].as_assoc_node().is_some()
    }
}

fn is_bare_single_hash_argument_call(
    call: &ruby_prism::CallNode<'_>,
    arg_list: &[ruby_prism::Node<'_>],
) -> bool {
    call.opening_loc().is_none()
        && matches!(
            arg_list,
            [arg] if arg.as_hash_node().is_some() || arg.as_keyword_hash_node().is_some()
        )
}

/// RuboCop still flags parenthesized single-key hash arguments like
/// `receive_message_chain("foo" => 1)`, but bare command-call hashes like
/// `receive_message_chain "foo" => 1` and `stub_chain "foo" => 1` crash in
/// RuboCop's autocorrect path and produce no offense in the corpus baseline.
/// Skip only that no-parens single-hash form while preserving the existing
/// single-argument detections.
pub struct SingleArgumentMessageChain;

impl Cop for SingleArgumentMessageChain {
    fn name(&self) -> &'static str {
        "RSpec/SingleArgumentMessageChain"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE, STRING_NODE, SYMBOL_NODE]
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

        let method = call.name().as_slice();
        let replacement = if method == b"receive_message_chain" {
            "receive"
        } else if method == b"stub_chain" {
            "stub"
        } else {
            return;
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();

        if is_bare_single_hash_argument_call(&call, &arg_list) {
            return;
        }

        let is_single_arg = matches!(arg_list.as_slice(), [arg] if is_single_chain_argument(arg));

        if !is_single_arg {
            return;
        }

        let method_str = std::str::from_utf8(method).unwrap_or("receive_message_chain");
        let loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `{replacement}` instead of calling `{method_str}` with a single argument."
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SingleArgumentMessageChain,
        "cops/rspec/single_argument_message_chain"
    );
}
