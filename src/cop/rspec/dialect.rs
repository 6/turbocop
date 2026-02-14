use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Dialect;

/// Default preferred methods: context -> describe.
const DEFAULT_PREFERRED: &[(&str, &str)] = &[("context", "describe")];

impl Cop for Dialect {
    fn name(&self) -> &'static str {
        "RSpec/Dialect"
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
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must have a block to be an RSpec DSL call
        if call.block().is_none() {
            return Vec::new();
        }

        let method_name = call.name().as_slice();
        let method_str = match std::str::from_utf8(method_name) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        // Check against preferred methods
        for &(bad, good) in DEFAULT_PREFERRED {
            if method_str == bad {
                // Can be receiverless or RSpec.context
                let is_rspec_call = if call.receiver().is_none() {
                    true
                } else if let Some(recv) = call.receiver() {
                    if let Some(cr) = recv.as_constant_read_node() {
                        cr.name().as_slice() == b"RSpec"
                    } else {
                        false
                    }
                } else {
                    false
                };

                if !is_rspec_call {
                    return Vec::new();
                }

                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Prefer `{good}` over `{bad}`."),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Dialect, "cops/rspec/dialect");
}
