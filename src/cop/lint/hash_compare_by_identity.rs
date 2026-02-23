use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct HashCompareByIdentity;

const HASH_KEY_METHODS: &[&[u8]] = &[b"key?", b"has_key?", b"fetch", b"[]", b"[]="];

impl Cop for HashCompareByIdentity {
    fn name(&self) -> &'static str {
        "Lint/HashCompareByIdentity"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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

        // Check if it's one of the hash key methods
        if !HASH_KEY_METHODS.contains(&method_name) {
            return;
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        // Check if the first argument is a `.object_id` call
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let first_arg = &arg_list[0];
        if let Some(arg_call) = first_arg.as_call_node() {
            if arg_call.name().as_slice() == b"object_id" && arg_call.receiver().is_some() {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(
                    self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `Hash#compare_by_identity` instead of using `object_id` for keys."
                            .to_string(),
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashCompareByIdentity, "cops/lint/hash_compare_by_identity");
}
