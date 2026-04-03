use crate::cop::shared::node_type::{UNTIL_NODE, WHILE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Investigation findings:
/// - FN=12 with 862 existing matches and 0 FP in the corpus.
/// - Root cause: the original implementation searched only the first source line
///   of the loop for a trailing `do`, so it missed valid offenses when the
///   predicate wrapped onto later lines, when `do` carried an inline comment,
///   or when the loop body started on the same line as `do`.
/// - Fix: use Prism's `do_keyword_loc()` directly and keep the check narrow to
///   loops whose full node span is multiline, matching RuboCop's `node.multiline? && node.do?`.
pub struct WhileUntilDo;

impl Cop for WhileUntilDo {
    fn name(&self) -> &'static str {
        "Style/WhileUntilDo"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[UNTIL_NODE, WHILE_NODE]
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
        // Check while ... do
        if let Some(while_node) = node.as_while_node() {
            if let Some(diagnostic) = check_loop(
                self,
                source,
                &while_node.location(),
                while_node.do_keyword_loc(),
                "while",
            ) {
                diagnostics.push(diagnostic);
            }
            return;
        }

        // Check until ... do
        if let Some(until_node) = node.as_until_node() {
            if let Some(diagnostic) = check_loop(
                self,
                source,
                &until_node.location(),
                until_node.do_keyword_loc(),
                "until",
            ) {
                diagnostics.push(diagnostic);
            }
        }
    }
}

fn check_loop(
    cop: &WhileUntilDo,
    source: &SourceFile,
    loop_loc: &ruby_prism::Location<'_>,
    do_loc: Option<ruby_prism::Location<'_>>,
    keyword: &str,
) -> Option<Diagnostic> {
    let do_loc = do_loc?;

    let (start_line, _) = source.offset_to_line_col(loop_loc.start_offset());
    let end_offset = loop_loc
        .end_offset()
        .saturating_sub(1)
        .max(loop_loc.start_offset());
    let (end_line, _) = source.offset_to_line_col(end_offset);

    if start_line == end_line {
        return None;
    }

    let (line, column) = source.offset_to_line_col(do_loc.start_offset());

    Some(cop.diagnostic(
        source,
        line,
        column,
        format!("Do not use `do` with multi-line `{}`.", keyword),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhileUntilDo, "cops/style/while_until_do");
}
