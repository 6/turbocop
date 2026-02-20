use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTEGER_NODE, RANGE_NODE};

pub struct RandomWithOffset;

impl Cop for RandomWithOffset {
    fn name(&self) -> &'static str {
        "Style/RandomWithOffset"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTEGER_NODE, RANGE_NODE]
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

        let method_bytes = call.name().as_slice();

        // Pattern 1: rand(n) + offset or offset + rand(n)
        // Pattern 2: rand(n) - offset
        // Pattern 3: rand(n).succ / rand(n).next / rand(n).pred
        if method_bytes == b"+" || method_bytes == b"-" {
            diagnostics.extend(self.check_arithmetic(source, node, &call));
        }

        if method_bytes == b"succ" || method_bytes == b"next" || method_bytes == b"pred" {
            diagnostics.extend(self.check_succ_pred(source, node, &call));
        }

    }
}

impl RandomWithOffset {
    /// Check if a node is `rand(int)` or `rand(int..int)` or `Random.rand(...)` or `Kernel.rand(...)`
    fn is_rand_call(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() != b"rand" {
                return false;
            }
            // Receiver must be nil, Random, or Kernel
            if let Some(recv) = call.receiver() {
                let is_random_or_kernel = recv.as_constant_read_node().is_some_and(|c| {
                    let name = c.name().as_slice();
                    name == b"Random" || name == b"Kernel"
                }) || recv.as_constant_path_node().is_some_and(|cp| {
                    let src = cp.location().as_slice();
                    src == b"Random" || src == b"Kernel" || src == b"::Random" || src == b"::Kernel"
                });
                if !is_random_or_kernel {
                    return false;
                }
            }
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() == 1 {
                    // Argument must be int or (int..int)
                    let arg = &arg_list[0];
                    if arg.as_integer_node().is_some() {
                        return true;
                    }
                    if let Some(range) = arg.as_range_node() {
                        let left_int = range.left().is_some_and(|l| l.as_integer_node().is_some());
                        let right_int = range.right().is_some_and(|r| r.as_integer_node().is_some());
                        return left_int && right_int;
                    }
                }
            }
        }
        false
    }

    /// Check if a node is an integer literal
    fn is_integer(node: &ruby_prism::Node<'_>) -> bool {
        node.as_integer_node().is_some()
    }

    fn check_arithmetic(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        call: &ruby_prism::CallNode<'_>,
    ) -> Vec<Diagnostic> {
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        // Pattern 1: rand(n) + integer or rand(n) - integer
        // Pattern 2: integer + rand(n)
        let is_match = (Self::is_rand_call(&receiver) && Self::is_integer(&arg_list[0]))
            || (Self::is_integer(&receiver) && Self::is_rand_call(&arg_list[0]));

        if is_match {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Prefer ranges when generating random numbers instead of integers with offsets.".to_string(),
            )];
        }

        Vec::new()
    }

    fn check_succ_pred(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        call: &ruby_prism::CallNode<'_>,
    ) -> Vec<Diagnostic> {
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        if Self::is_rand_call(&receiver) {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Prefer ranges when generating random numbers instead of integers with offsets.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RandomWithOffset, "cops/style/random_with_offset");
}
