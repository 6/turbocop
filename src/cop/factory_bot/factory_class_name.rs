use crate::cop::factory_bot::FACTORY_BOT_DEFAULT_INCLUDE;
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, HASH_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE};

pub struct FactoryClassName;

const ALLOWED_CONSTANTS: &[&[u8]] = &[b"Hash", b"OpenStruct"];

impl Cop for FactoryClassName {
    fn name(&self) -> &'static str {
        "FactoryBot/FactoryClassName"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        FACTORY_BOT_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, HASH_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE]
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

        if call.name().as_slice() != b"factory" {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();

        // Look for a hash argument with `class:` key pointing to a constant
        for arg in &arg_list {
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

                // Key must be :class
                let key_is_class = pair
                    .key()
                    .as_symbol_node()
                    .map_or(false, |s| s.unescaped() == b"class");

                if !key_is_class {
                    continue;
                }

                let value = pair.value();

                // Value must be a constant (ConstantReadNode or ConstantPathNode)
                let const_name = if value.as_constant_read_node().is_some() {
                    Some(util::full_constant_path(source, &value))
                } else if value.as_constant_path_node().is_some() {
                    Some(util::full_constant_path(source, &value))
                } else {
                    None
                };

                let const_name_bytes = match const_name {
                    Some(n) => n,
                    None => continue,
                };

                // Skip allowed constants (last segment check)
                let last_segment = util::constant_name(&value);
                if let Some(name) = last_segment {
                    if ALLOWED_CONSTANTS.contains(&name) {
                        continue;
                    }
                }

                let const_name_str =
                    std::str::from_utf8(const_name_bytes).unwrap_or("<unknown>");

                let loc = value.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Pass '{}' string instead of `{}` constant.",
                        const_name_str, const_name_str
                    ),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FactoryClassName, "cops/factorybot/factory_class_name");
}
