use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct HashTransformValues;

impl Cop for HashTransformValues {
    fn name(&self) -> &'static str {
        "Style/HashTransformValues"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Look for CallNode `each_with_object({})` with a block
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"each_with_object" {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Check that the argument is an empty hash
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1
            || (arg_list[0].as_hash_node().is_none()
                && arg_list[0].as_keyword_hash_node().is_none())
        {
            return Vec::new();
        }

        if let Some(hash) = arg_list[0].as_hash_node() {
            let hash_src = hash.location().as_slice();
            let trimmed: Vec<u8> = hash_src
                .iter()
                .filter(|&&b| b != b' ' && b != b'{' && b != b'}')
                .copied()
                .collect();
            if !trimmed.is_empty() {
                return Vec::new();
            }
        }

        // Check body: should be h[k] = something(v)
        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return Vec::new();
        }

        // Check for h[k] = transform(v) pattern
        if let Some(assign_call) = body_nodes[0].as_call_node() {
            if assign_call.name().as_slice() == b"[]=" {
                if let Some(assign_args) = assign_call.arguments() {
                    let aargs: Vec<_> = assign_args.arguments().iter().collect();
                    if aargs.len() == 2 {
                        let key_is_simple = aargs[0].as_local_variable_read_node().is_some();
                        let value_is_simple = aargs[1].as_local_variable_read_node().is_some();

                        if key_is_simple && !value_is_simple {
                            let loc = call.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            return vec![self.diagnostic(
                                source,
                                line,
                                column,
                                "Prefer `transform_values` over `each_with_object`."
                                    .to_string(),
                            )];
                        }
                    }
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashTransformValues, "cops/style/hash_transform_values");
}
