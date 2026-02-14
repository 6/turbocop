use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct HookArgument;

/// Hook methods to check.
const HOOK_METHODS: &[&[u8]] = &[b"before", b"after", b"around"];

/// Scope args that mean "suite" or "context" — not flagged.
const NON_EXAMPLE_SCOPES: &[&[u8]] = &[b"suite", b"context", b"all"];

impl Cop for HookArgument {
    fn name(&self) -> &'static str {
        "RSpec/HookArgument"
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
        // Default EnforcedStyle is :implicit — flag :each and :example arguments
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();

        // Check both receiverless `before(...)` and `config.before(...)`
        let is_hook = if call.receiver().is_none() {
            HOOK_METHODS.iter().any(|m| method_name == *m)
        } else {
            HOOK_METHODS.iter().any(|m| method_name == *m)
        };

        if !is_hook {
            return Vec::new();
        }

        // Must have a block
        if call.block().is_none() {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(), // No args = implicit style, fine
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();

        // Ignore hooks with more than one argument
        if arg_list.len() > 1 {
            return Vec::new();
        }

        if arg_list.is_empty() {
            return Vec::new();
        }

        let first_arg = &arg_list[0];

        // Check for symbol argument
        if let Some(sym) = first_arg.as_symbol_node() {
            let val = sym.unescaped();

            // Ignore :suite, :context, :all — those are different scopes
            if NON_EXAMPLE_SCOPES.iter().any(|s| val == *s) {
                return Vec::new();
            }

            // Flag :each and :example — should be implicit
            if val == b"each" || val == b"example" {
                let scope_str = std::str::from_utf8(val).unwrap_or("each");
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Omit the default `:{scope_str}` argument for RSpec hooks.",
                    ),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HookArgument, "cops/rspec/hook_argument");
}
