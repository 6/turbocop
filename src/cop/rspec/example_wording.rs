use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ExampleWording;

/// Example methods that take a description string.
const EXAMPLE_METHODS: &[&[u8]] = &[
    b"it", b"specify", b"example",
    b"fit", b"fspecify", b"fexample",
    b"xit", b"xspecify", b"xexample",
];

impl Cop for ExampleWording {
    fn name(&self) -> &'static str {
        "RSpec/ExampleWording"
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
        if !EXAMPLE_METHODS.iter().any(|m| method_name == *m) {
            return Vec::new();
        }

        // Get the first positional argument (the description string)
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        for arg in args.arguments().iter() {
            if arg.as_keyword_hash_node().is_some() {
                continue;
            }
            // Check string nodes
            if let Some(s) = arg.as_string_node() {
                let desc = s.unescaped();
                if starts_with_should(desc) {
                    let loc = s.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not use should when describing your tests.".to_string(),
                    )];
                }
            }
            // Also check interpolated strings
            if let Some(interp) = arg.as_interpolated_string_node() {
                // Check if the first part starts with "should"
                let parts: Vec<_> = interp.parts().iter().collect();
                if let Some(first) = parts.first() {
                    if let Some(s) = first.as_string_node() {
                        if starts_with_should(s.unescaped()) {
                            let loc = interp.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            return vec![self.diagnostic(
                                source,
                                line,
                                column,
                                "Do not use should when describing your tests.".to_string(),
                            )];
                        }
                    }
                }
            }
            break;
        }

        Vec::new()
    }
}

/// Check if a byte slice starts with "should" (case-insensitive).
fn starts_with_should(desc: &[u8]) -> bool {
    if desc.len() < 6 {
        return false;
    }
    let lower: Vec<u8> = desc[..6].iter().map(|b| b.to_ascii_lowercase()).collect();
    if lower != b"should" {
        return false;
    }
    // "should" alone or followed by space, "'", "n't"
    desc.len() == 6 || desc[6] == b' ' || desc[6] == b'\'' || desc[6] == b'n'
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExampleWording, "cops/rspec/example_wording");
}
