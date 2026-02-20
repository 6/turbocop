use crate::cop::util::{is_rspec_example, is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, SYMBOL_NODE};

pub struct DuplicatedMetadata;

impl Cop for DuplicatedMetadata {
    fn name(&self) -> &'static str {
        "RSpec/DuplicatedMetadata"
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
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Must be an RSpec method that takes metadata
        if !is_rspec_example_group(method_name)
            && !is_rspec_example(method_name)
            && method_name != b"shared_examples"
            && method_name != b"shared_examples_for"
            && method_name != b"shared_context"
            && method_name != b"before"
            && method_name != b"after"
            && method_name != b"around"
        {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        // Collect symbol arguments and check for duplicates
        let mut seen_symbols: Vec<Vec<u8>> = Vec::new();

        for arg in args.arguments().iter() {
            if let Some(sym) = arg.as_symbol_node() {
                let name = sym.unescaped().to_vec();
                if seen_symbols.contains(&name) {
                    let loc = sym.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Avoid duplicated metadata.".to_string(),
                    ));
                } else {
                    seen_symbols.push(name);
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicatedMetadata, "cops/rspec/duplicated_metadata");
}
