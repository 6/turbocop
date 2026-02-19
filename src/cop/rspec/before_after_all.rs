use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, SYMBOL_NODE};

pub struct BeforeAfterAll;

/// Flags `before(:all)`, `before(:context)`, `after(:all)`, `after(:context)`.
/// These hooks can cause state to leak between tests.
impl Cop for BeforeAfterAll {
    fn name(&self) -> &'static str {
        "RSpec/BeforeAfterAll"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, SYMBOL_NODE]
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
        if method_name != b"before" && method_name != b"after" {
            return Vec::new();
        }

        // Must be receiverless
        if call.receiver().is_some() {
            return Vec::new();
        }

        // Must have a block (or block pass)
        if call.block().is_none() {
            return Vec::new();
        }

        // Check for :all or :context argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let first_arg = &arg_list[0];
        let scope = if let Some(sym) = first_arg.as_symbol_node() {
            sym.unescaped().to_vec()
        } else {
            return Vec::new();
        };

        if scope != b"all" && scope != b"context" {
            return Vec::new();
        }

        let method_str = std::str::from_utf8(method_name).unwrap_or("before");
        let scope_str = std::str::from_utf8(&scope).unwrap_or("all");
        let hook = format!("{method_str}(:{scope_str})");

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!(
                "Beware of using `{hook}` as it may cause state to leak between tests. \
                 If you are using `rspec-rails`, and `use_transactional_fixtures` is enabled, \
                 then records created in `{hook}` are not automatically rolled back."
            ),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BeforeAfterAll, "cops/rspec/before_after_all");
}
