use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct CollectionMethods;

impl Cop for CollectionMethods {
    fn name(&self) -> &'static str {
        "Style/CollectionMethods"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let preferred_methods = config.get_string_hash("PreferredMethods").unwrap_or_default();
        let _methods_accepting_symbol = config.get_string_array("MethodsAcceptingSymbol").unwrap_or_default();

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must have a receiver (collection.method)
        if call.receiver().is_none() {
            return Vec::new();
        }

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");

        if let Some(preferred) = preferred_methods.get(method_name) {
            let loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("Prefer `{}` over `{}`.", preferred, method_name),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CollectionMethods, "cops/style/collection_methods");
}
