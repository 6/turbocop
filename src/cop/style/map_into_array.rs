use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MapIntoArray;

impl Cop for MapIntoArray {
    fn name(&self) -> &'static str {
        "Style/MapIntoArray"
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

        // Check for each { |x| arr << transform(x) } pattern
        // Suggesting map instead
        if call.name().as_slice() != b"each" {
            return Vec::new();
        }

        if call.receiver().is_none() {
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

        // Check for arr << expr or arr.push(expr) or arr.append(expr)
        if let Some(push_call) = body_nodes[0].as_call_node() {
            let push_method = push_call.name().as_slice();
            if push_method == b"<<" || push_method == b"push" || push_method == b"append" {
                // The receiver of << / push / append must be a local variable
                // (not an instance var, class var, global var, or method call)
                let push_receiver = match push_call.receiver() {
                    Some(r) => r,
                    None => return Vec::new(),
                };
                if push_receiver.as_local_variable_read_node().is_none() {
                    return Vec::new();
                }

                // Receiver of `each` must not be `self` or bare (no receiver)
                if let Some(each_receiver) = call.receiver() {
                    if each_receiver.as_self_node().is_some() {
                        return Vec::new();
                    }
                }

                // `each` must have no arguments (e.g., StringIO.new.each(':') should not fire)
                if call.arguments().is_some() {
                    return Vec::new();
                }

                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `map` instead of `each` to map elements into an array.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MapIntoArray, "cops/style/map_into_array");
}
