use crate::cop::util::{self, is_rspec_example, is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE, TRUE_NODE};

pub struct MetadataStyle;

/// Default enforces symbol style: `:foo` instead of `foo: true`.
impl Cop for MetadataStyle {
    fn name(&self) -> &'static str {
        "RSpec/MetadataStyle"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE, TRUE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        if !is_rspec_example_group(method_name) && !is_rspec_example(method_name) {
            return;
        }

        // Must be receiverless or RSpec.describe / ::RSpec.describe
        if let Some(recv) = call.receiver() {
            if util::constant_name(&recv).map_or(true, |n| n != b"RSpec") {
                return;
            }
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let style = config.get_str("EnforcedStyle", "symbol");

        if style == "symbol" {
            // Flag `key: true` keyword args — should be `:key` symbol style
            for arg in args.arguments().iter() {
                if let Some(kw) = arg.as_keyword_hash_node() {
                    for elem in kw.elements().iter() {
                        if let Some(assoc) = elem.as_assoc_node() {
                            // Key must be a symbol
                            if assoc.key().as_symbol_node().is_none() {
                                continue;
                            }
                            // Value must be `true`
                            if assoc.value().as_true_node().is_some() {
                                let loc = elem.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    "Use symbol style for metadata.".to_string(),
                                ));
                            }
                        }
                    }
                }
            }
        } else if style == "hash" {
            // Flag `:key` symbol args — should be `key: true` hash style
            for arg in args.arguments().iter() {
                if let Some(sym) = arg.as_symbol_node() {
                    let loc = sym.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use hash style for metadata.".to_string(),
                    ));
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MetadataStyle, "cops/rspec/metadata_style");
}
