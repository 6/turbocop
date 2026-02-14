use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct HttpPositionalArguments;

const HTTP_METHODS: &[&[u8]] = &[
    b"get", b"post", b"put", b"patch", b"delete", b"head",
];

impl Cop for HttpPositionalArguments {
    fn name(&self) -> &'static str {
        "Rails/HttpPositionalArguments"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        if !HTTP_METHODS.contains(&method_name) {
            return Vec::new();
        }
        // Only flag receiverless calls (test helpers like `get :index, ...`)
        if call.receiver().is_some() {
            return Vec::new();
        }
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        // Flag if there are 3+ positional arguments (action, params hash, headers hash)
        // or if second arg is a hash literal (not keyword hash)
        if arg_list.len() >= 3 {
            // Check that the extra args are not keyword hashes (which would be fine)
            // If we have 3+ args with a hash literal as 2nd, it's the old positional style
            if arg_list[1].as_hash_node().is_some() {
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use keyword arguments for HTTP request methods.".to_string(),
                )];
            }
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        HttpPositionalArguments,
        "cops/rails/http_positional_arguments"
    );
}
