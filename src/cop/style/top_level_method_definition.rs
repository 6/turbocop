use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TopLevelMethodDefinition;

impl Cop for TopLevelMethodDefinition {
    fn name(&self) -> &'static str {
        "Style/TopLevelMethodDefinition"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {

        let root = parse_result.node();
        if let Some(program) = root.as_program_node() {
            let stmts = program.statements();
            for stmt in stmts.body().iter() {
                if stmt.as_def_node().is_some() {
                    let loc = stmt.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not define methods at the top level.".to_string(),
                    ));
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TopLevelMethodDefinition, "cops/style/top_level_method_definition");
}
