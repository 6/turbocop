use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Sum;

impl Cop for Sum {
    fn name(&self) -> &'static str {
        "Performance/Sum"
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
        if method_name != b"inject" && method_name != b"reduce" {
            return Vec::new();
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return Vec::new();
        }

        // Must not have a block
        if call.block().is_some() {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_nodes: Vec<_> = args.arguments().iter().collect();

        let is_sum_pattern = match arg_nodes.len() {
            1 => {
                // inject(:+) or reduce(:+)
                is_plus_symbol(&arg_nodes[0])
            }
            2 => {
                // inject(0, :+) or reduce(0, :+)
                is_zero_literal(&arg_nodes[0]) && is_plus_symbol(&arg_nodes[1])
            }
            _ => false,
        };

        if !is_sum_pattern {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        let method_str = std::str::from_utf8(method_name).unwrap_or("inject");
        let args_str = if arg_nodes.len() == 2 {
            format!("{method_str}(0, :+)")
        } else {
            format!("{method_str}(:+)")
        };

        vec![self.diagnostic(source, line, column, format!("Use `sum` instead of `{args_str}`."))]
    }
}

fn is_plus_symbol(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(sym) = node.as_symbol_node() {
        return sym.unescaped() == b"+";
    }
    false
}

fn is_zero_literal(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(int) = node.as_integer_node() {
        let value = int.value();
        let (negative, digits) = value.to_u32_digits();
        return !negative && digits == [0];
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Sum, "cops/performance/sum");
}
