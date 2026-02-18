use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RandomWithOffset;

impl Cop for RandomWithOffset {
    fn name(&self) -> &'static str {
        "Style/RandomWithOffset"
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

        let method_bytes = call.name().as_slice();

        // Pattern 1: rand(n) + offset or offset + rand(n)
        // Pattern 2: rand(n) - offset
        // Pattern 3: rand(n).succ / rand(n).next / rand(n).pred
        if method_bytes == b"+" || method_bytes == b"-" {
            return self.check_arithmetic(source, node, &call);
        }

        if method_bytes == b"succ" || method_bytes == b"next" || method_bytes == b"pred" {
            return self.check_succ_pred(source, node, &call);
        }

        Vec::new()
    }
}

impl RandomWithOffset {
    fn is_rand_call(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"rand" && call.receiver().is_none() {
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    return arg_list.len() == 1;
                }
            }
        }
        false
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

        // Check if either side is rand(n)
        if Self::is_rand_call(&receiver) || Self::is_rand_call(&arg_list[0]) {
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
