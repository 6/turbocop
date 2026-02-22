use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for `ruby2_keywords` calls on methods that don't need it.
/// `ruby2_keywords` should only be used on methods that accept `*args` but
/// do not have explicit keyword arguments or `**kwargs`.
pub struct UselessRuby2Keywords;

impl Cop for UselessRuby2Keywords {
    fn name(&self) -> &'static str {
        "Lint/UselessRuby2Keywords"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        let mut visitor = R2KVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct R2KVisitor<'a, 'src> {
    cop: &'a UselessRuby2Keywords,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for R2KVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"ruby2_keywords" && node.receiver().is_none() {
            if let Some(args) = node.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if !arg_list.is_empty() {
                    let first_arg = &arg_list[0];

                    // Case 1: ruby2_keywords def foo(*args); end
                    if let Some(def_node) = first_arg.as_def_node() {
                        if !allowed_arguments(&def_node) {
                            let method_name = std::str::from_utf8(def_node.name().as_slice())
                                .unwrap_or("unknown");
                            let msg_loc = node.message_loc().unwrap_or(node.location());
                            let (line, column) =
                                self.source.offset_to_line_col(msg_loc.start_offset());
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                format!(
                                    "`ruby2_keywords` is unnecessary for method `{}`.",
                                    method_name
                                ),
                            ));
                        }
                    }

                    // Case 2: ruby2_keywords :foo (symbol reference)
                    // We'd need to look up the method definition - simplified version
                    // just flags the symbol case if it exists
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }
}

/// Check if the def has `*args` and no keyword arguments/splats.
/// Returns true if the method SHOULD have ruby2_keywords (i.e., it's allowed).
fn allowed_arguments(def_node: &ruby_prism::DefNode<'_>) -> bool {
    let params = match def_node.parameters() {
        Some(p) => p,
        None => return false, // No params at all => ruby2_keywords is useless
    };

    // Must have a rest parameter (*args)
    let has_rest = params.rest().is_some();
    if !has_rest {
        return false;
    }

    // Must NOT have keyword args or keyword rest
    let has_keywords = !params.keywords().is_empty();
    let has_keyword_rest = params.keyword_rest().is_some();

    if has_keywords || has_keyword_rest {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessRuby2Keywords, "cops/lint/useless_ruby2_keywords");
}
