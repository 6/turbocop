use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct IndexedLet;

impl Cop for IndexedLet {
    fn name(&self) -> &'static str {
        "RSpec/IndexedLet"
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

        if call.receiver().is_some() {
            return Vec::new();
        }

        let method_name = call.name().as_slice();
        if method_name != b"let" && method_name != b"let!" {
            return Vec::new();
        }

        // Get the first argument (the name)
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        for arg in args.arguments().iter() {
            if arg.as_keyword_hash_node().is_some() {
                continue;
            }
            let name: Vec<u8> = if let Some(sym) = arg.as_symbol_node() {
                sym.unescaped().to_vec()
            } else if let Some(s) = arg.as_string_node() {
                s.unescaped().to_vec()
            } else {
                break;
            };

            // Check if the name ends with a numeric suffix (e.g., item_1, item1)
            if let Some(digit) = find_trailing_digit(&name) {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "This `let` statement uses `{}` in its name. Please give it a meaningful name.",
                        digit
                    ),
                )];
            }
            break;
        }

        Vec::new()
    }
}

/// Find trailing numeric suffix in a name like `item_1` or `item1`.
/// Returns the first digit found at the end of the name.
fn find_trailing_digit(name: &[u8]) -> Option<char> {
    // Walk backwards to find trailing digits
    let mut i = name.len();
    while i > 0 && name[i - 1].is_ascii_digit() {
        i -= 1;
    }
    if i < name.len() && i > 0 {
        // There's at least one trailing digit and something before it
        Some(name[i] as char)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IndexedLet, "cops/rspec/indexed_let");
}
