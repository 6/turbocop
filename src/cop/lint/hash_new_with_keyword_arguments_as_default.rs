use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE};

/// Checks for the deprecated use of keyword arguments as a default in `Hash.new`.
/// In Ruby 3.4, keyword arguments will be used to change hash behavior (e.g., `capacity:`).
pub struct HashNewWithKeywordArgumentsAsDefault;

impl Cop for HashNewWithKeywordArgumentsAsDefault {
    fn name(&self) -> &'static str {
        "Lint/HashNewWithKeywordArgumentsAsDefault"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE]
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

        if call.name().as_slice() != b"new" {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let name = match constant_name(&receiver) {
            Some(n) => n,
            None => return Vec::new(),
        };

        if name != b"Hash" {
            return Vec::new();
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args: Vec<_> = arguments.arguments().iter().collect();

        // We're looking for Hash.new(key: :value) - a keyword hash without braces
        if args.len() != 1 {
            return Vec::new();
        }

        let first_arg = &args[0];

        // Check for keyword hash (no braces)
        let kw_hash = match first_arg.as_keyword_hash_node() {
            Some(h) => h,
            None => return Vec::new(),
        };

        // If the single pair has key `:capacity`, skip (it's a valid Ruby 3.4 option)
        let elements: Vec<_> = kw_hash.elements().iter().collect();
        if elements.len() == 1 {
            if let Some(pair) = elements[0].as_assoc_node() {
                if let Some(sym) = pair.key().as_symbol_node() {
                    if sym.unescaped() == b"capacity" {
                        return Vec::new();
                    }
                }
            }
        }

        let loc = first_arg.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use a hash literal instead of keyword arguments.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        HashNewWithKeywordArgumentsAsDefault,
        "cops/lint/hash_new_with_keyword_arguments_as_default"
    );
}
