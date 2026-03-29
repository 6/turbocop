use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Prism represents top-level `define_method` as a `CallNode`, including
/// receiver-qualified forms like `Foo.define_method`, inline/multiline blocks,
/// and proc-argument forms. The original cop only matched `DefNode`, so it
/// missed these top-level dynamic method definitions.
pub struct TopLevelMethodDefinition;

const MESSAGE: &str = "Do not define methods at the top level.";

fn is_top_level_method_definition(node: &ruby_prism::Node<'_>) -> bool {
    node.as_def_node().is_some() || is_top_level_define_method_call(node)
}

fn is_top_level_define_method_call(node: &ruby_prism::Node<'_>) -> bool {
    node.as_call_node()
        .is_some_and(|call| call.name().as_slice() == b"define_method")
}

impl Cop for TopLevelMethodDefinition {
    fn name(&self) -> &'static str {
        "Style/TopLevelMethodDefinition"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let root = parse_result.node();
        if let Some(program) = root.as_program_node() {
            let stmts = program.statements();
            for stmt in stmts.body().iter() {
                if is_top_level_method_definition(&stmt) {
                    let loc = stmt.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(source, line, column, MESSAGE.to_string()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        TopLevelMethodDefinition,
        "cops/style/top_level_method_definition"
    );
}
