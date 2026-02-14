use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct WordArray;

impl Cop for WordArray {
    fn name(&self) -> &'static str {
        "Style/WordArray"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let array_node = match node.as_array_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Must have `[` opening (not %w or %W)
        let opening = match array_node.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        if opening.as_slice() != b"[" {
            return Vec::new();
        }

        let elements = array_node.elements();
        let min_size: usize = config
            .options
            .get("MinSize")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(2);

        if elements.len() < min_size {
            return Vec::new();
        }

        // All elements must be simple string nodes
        for elem in elements.iter() {
            let string_node = match elem.as_string_node() {
                Some(s) => s,
                None => return Vec::new(),
            };

            // Must have an opening quote (not a bare string)
            if string_node.opening_loc().is_none() {
                return Vec::new();
            }

            // Content must not contain spaces
            let content = string_node.content_loc().as_slice();
            if content.contains(&b' ') {
                return Vec::new();
            }

            // Must not have escape sequences (backslash in content)
            if content.contains(&b'\\') {
                return Vec::new();
            }
        }

        let (line, column) = source.offset_to_line_col(opening.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: Severity::Convention,
            cop_name: self.name().to_string(),
            message: "Use `%w` or `%W` for an array of words.".to_string(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &WordArray,
            include_bytes!("../../../testdata/cops/style/word_array/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &WordArray,
            include_bytes!("../../../testdata/cops/style/word_array/no_offense.rb"),
        );
    }
}
