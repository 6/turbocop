use crate::cop::factory_bot::FACTORY_BOT_SPEC_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ARRAY_NODE, ASSOC_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE};

pub struct RedundantFactoryOption;

impl Cop for RedundantFactoryOption {
    fn name(&self) -> &'static str {
        "FactoryBot/RedundantFactoryOption"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        FACTORY_BOT_SPEC_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, ASSOC_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"association" {
            return;
        }

        // Must have no receiver
        if call.receiver().is_some() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // First argument: association name (symbol)
        let assoc_name = match arg_list[0].as_symbol_node() {
            Some(s) => s.unescaped().to_vec(),
            None => return,
        };

        // Look for hash argument with factory: key
        for arg in arg_list.iter().skip(1) {
            let pairs = if let Some(hash) = arg.as_keyword_hash_node() {
                hash.elements().iter().collect::<Vec<_>>()
            } else if let Some(hash) = arg.as_hash_node() {
                hash.elements().iter().collect::<Vec<_>>()
            } else {
                continue;
            };

            for elem in &pairs {
                let pair = match elem.as_assoc_node() {
                    Some(p) => p,
                    None => continue,
                };

                // Key must be :factory
                let is_factory_key = pair
                    .key()
                    .as_symbol_node()
                    .map_or(false, |s| s.unescaped() == b"factory");

                if !is_factory_key {
                    continue;
                }

                // Value must be a symbol matching the association name,
                // or an array with a single symbol matching the association name
                let factory_name = if let Some(sym) = pair.value().as_symbol_node() {
                    Some(sym.unescaped().to_vec())
                } else if let Some(arr) = pair.value().as_array_node() {
                    let elems: Vec<_> = arr.elements().iter().collect();
                    if elems.len() == 1 {
                        elems[0]
                            .as_symbol_node()
                            .map(|s| s.unescaped().to_vec())
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(name) = factory_name {
                    if name == assoc_name {
                        let loc = pair.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Remove redundant `factory` option.".to_string(),
                        ));
                    }
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantFactoryOption,
        "cops/factorybot/redundant_factory_option"
    );
}
