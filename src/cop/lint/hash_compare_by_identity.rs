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

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();

        // Check if it's one of the hash key methods
        if !HASH_KEY_METHODS.iter().any(|m| *m == method_name) {
            return Vec::new();
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return Vec::new();
        }

        // Check if the first argument is a `.object_id` call
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let first_arg = &arg_list[0];
        if let Some(arg_call) = first_arg.as_call_node() {
            if arg_call.name().as_slice() == b"object_id" && arg_call.receiver().is_some() {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `Hash#compare_by_identity` instead of using `object_id` for keys."
                        .to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashCompareByIdentity, "cops/lint/hash_compare_by_identity");
}
