use crate::cop::util::{is_rspec_example_group, is_rspec_let, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MultipleMemoizedHelpers;

impl Cop for MultipleMemoizedHelpers {
    fn name(&self) -> &'static str {
        "RSpec/MultipleMemoizedHelpers"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();

        // Check for example group calls (describe, context, etc.)
        let is_example_group = if let Some(recv) = call.receiver() {
            if let Some(rc) = recv.as_constant_read_node() {
                rc.name().as_slice() == b"RSpec" && method_name == b"describe"
            } else {
                false
            }
        } else {
            is_rspec_example_group(method_name)
        };

        if !is_example_group {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        let max = config.get_usize("Max", 5);

        // Count direct let/let! declarations in this block
        let body = match block.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut count = 0;
        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                if c.receiver().is_none() && is_rspec_let(c.name().as_slice()) {
                    count += 1;
                }
            }
        }

        if count > max {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            vec![self.diagnostic(
                source,
                line,
                column,
                format!("Example group has too many memoized helpers [{count}/{max}]"),
            )]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultipleMemoizedHelpers, "cops/rspec/multiple_memoized_helpers");
}
