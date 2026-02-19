use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{AND_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, DEFINED_NODE};

pub struct CombinableDefined;

impl Cop for CombinableDefined {
    fn name(&self) -> &'static str {
        "Style/CombinableDefined"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[AND_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, DEFINED_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Check for `defined?(Foo) && defined?(Foo::Bar)` or `defined?(Foo) and defined?(Foo::Bar)`
        let call = match node.as_call_node() {
            Some(c) => c,
            None => {
                // Also check AndNode
                if let Some(and_node) = node.as_and_node() {
                    diagnostics.extend(check_and(self, source, &and_node.left(), &and_node.right()));
                    return;
                }
                return;
            }
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        // `and` is parsed as a call in some contexts
        if method_name == "and" {
            if let Some(receiver) = call.receiver() {
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if arg_list.len() == 1 {
                        diagnostics.extend(check_and(self, source, &receiver, &arg_list[0]));
                        return;
                    }
                }
            }
        }

    }
}

fn check_and(
    cop: &CombinableDefined,
    source: &SourceFile,
    left: &ruby_prism::Node<'_>,
    right: &ruby_prism::Node<'_>,
) -> Vec<Diagnostic> {
    let left_defined = get_defined_const(left);
    let right_defined = get_defined_const(right);

    if let (Some(left_name), Some(right_name)) = (left_defined, right_defined) {
        // Check if one is a prefix of the other (nested constants)
        if right_name.starts_with(&format!("{}::", left_name))
            || left_name.starts_with(&format!("{}::", right_name))
        {
            let loc = left.location();
            let end_loc = right.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let _ = end_loc;
            return vec![cop.diagnostic(
                source,
                line,
                column,
                "Combine nested `defined?` calls.".to_string(),
            )];
        }
    }

    Vec::new()
}

fn get_defined_const(node: &ruby_prism::Node<'_>) -> Option<String> {
    if let Some(defined) = node.as_defined_node() {
        let value = defined.value();
        let name = extract_const_name(&value)?;
        return Some(name);
    }
    None
}

fn extract_const_name(node: &ruby_prism::Node<'_>) -> Option<String> {
    if let Some(read) = node.as_constant_read_node() {
        return Some(std::str::from_utf8(read.name().as_slice()).ok()?.to_string());
    }
    if let Some(path) = node.as_constant_path_node() {
        let name = std::str::from_utf8(path.name_loc().as_slice()).ok()?.to_string();
        if let Some(parent) = path.parent() {
            if let Some(parent_name) = extract_const_name(&parent) {
                return Some(format!("{}::{}", parent_name, name));
            }
        }
        // ::Foo case
        return Some(format!("::{}", name));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CombinableDefined, "cops/style/combinable_defined");
}
