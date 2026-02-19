use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct BlockComments;

impl Cop for BlockComments {
    fn name(&self) -> &'static str {
        "Style/BlockComments"
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig, diagnostics: &mut Vec<Diagnostic>) {
        for (i, line) in source.lines().enumerate() {
            // =begin must be at the start of a line
            if line.starts_with(b"=begin") && (line.len() == 6 || line[6].is_ascii_whitespace()) {
                diagnostics.push(self.diagnostic(
                    source,
                    i + 1,
                    0,
                    "Do not use block comments.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BlockComments, "cops/style/block_comments");
}
