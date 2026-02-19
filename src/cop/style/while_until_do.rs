use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct WhileUntilDo;

impl Cop for WhileUntilDo {
    fn name(&self) -> &'static str {
        "Style/WhileUntilDo"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check while ... do
        if let Some(while_node) = node.as_while_node() {
            return check_loop(self, source, &while_node.location(), while_node.closing_loc(), "while");
        }

        // Check until ... do
        if let Some(until_node) = node.as_until_node() {
            return check_loop(self, source, &until_node.location(), until_node.closing_loc(), "until");
        }

        Vec::new()
    }
}

fn check_loop(
    cop: &WhileUntilDo,
    source: &SourceFile,
    outer_loc: &ruby_prism::Location<'_>,
    closing_loc: Option<ruby_prism::Location<'_>>,
    keyword: &str,
) -> Vec<Diagnostic> {
    // Must be multiline (closing_loc exists and is on a different line than keyword)
    let closing = match closing_loc {
        Some(c) => c,
        None => return Vec::new(),
    };

    let (start_line, _) = source.offset_to_line_col(outer_loc.start_offset());
    let (end_line, _) = source.offset_to_line_col(closing.start_offset());

    // Single-line: no offense
    if start_line == end_line {
        return Vec::new();
    }

    // Check if there's a `do` keyword
    // In Prism, while/until nodes have a keyword_loc and optionally a "do" keyword
    // We look at source between predicate end and body/closing start for "do"
    let src = &source.content[outer_loc.start_offset()..outer_loc.end_offset()];
    let src_str = std::str::from_utf8(src).unwrap_or("");

    // Find "do" after the keyword line. Look at first line for "do" at end.
    let first_line = src_str.lines().next().unwrap_or("");
    let trimmed = first_line.trim_end();
    if !trimmed.ends_with(" do") && !trimmed.ends_with("\tdo") {
        return Vec::new();
    }

    // Find the position of "do" in the source
    let do_offset_in_first_line = trimmed.len() - 2;
    let do_offset = outer_loc.start_offset() + do_offset_in_first_line;
    let (line, column) = source.offset_to_line_col(do_offset);

    vec![cop.diagnostic(
        source,
        line,
        column,
        format!("Do not use `do` with multi-line `{}`.", keyword),
    )]
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhileUntilDo, "cops/style/while_until_do");
}
