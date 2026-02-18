use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct SuperArguments;

impl Cop for SuperArguments {
    fn name(&self) -> &'static str {
        "Style/SuperArguments"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let mut visitor = SuperArgumentsVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct SuperArgumentsVisitor<'a> {
    cop: &'a SuperArguments,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for SuperArgumentsVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Get the method's parameter names (only for simple positional params)
        let param_names = if let Some(params) = node.parameters() {
            match extract_param_names(&params) {
                Some(names) => names,
                None => return, // Complex params (rest, keyword, block) - skip
            }
        } else {
            Vec::new()
        };

        if let Some(body) = node.body() {
            let mut finder = SuperFinder {
                super_calls: Vec::new(),
            };
            finder.visit(&body);

            for (offset, arg_names) in finder.super_calls {
                // Check if super is called with exact same args as the method params
                if arg_names == param_names && !param_names.is_empty() {
                    let (line, column) = self.source.offset_to_line_col(offset);
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Call `super` without arguments and parentheses when the signature is identical.".to_string(),
                    ));
                }
            }
        }

        // Don't recurse into nested defs
    }
}

struct SuperFinder {
    super_calls: Vec<(usize, Vec<Vec<u8>>)>, // (offset, arg_names)
}

impl<'pr> Visit<'pr> for SuperFinder {
    fn visit_super_node(&mut self, node: &ruby_prism::SuperNode<'pr>) {
        let mut arg_names = Vec::new();
        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                if let Some(lv) = arg.as_local_variable_read_node() {
                    arg_names.push(lv.name().as_slice().to_vec());
                } else {
                    // Non-variable argument — can't simplify
                    return;
                }
            }
        }
        self.super_calls.push((node.location().start_offset(), arg_names));
    }

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
}

fn extract_param_names(params: &ruby_prism::ParametersNode<'_>) -> Option<Vec<Vec<u8>>> {
    // Only handle methods with purely positional params (required + optional).
    // If there are rest, keyword, keyword_rest, or block params, return None
    // because the super forwarding is more complex.
    if params.rest().is_some()
        || params.keywords().iter().next().is_some()
        || params.keyword_rest().is_some()
        || params.block().is_some()
        || params.posts().iter().next().is_some()
    {
        return None;
    }

    let mut names = Vec::new();
    for p in params.requireds().iter() {
        if let Some(rp) = p.as_required_parameter_node() {
            names.push(rp.name().as_slice().to_vec());
        }
    }
    for p in params.optionals().iter() {
        if let Some(op) = p.as_optional_parameter_node() {
            names.push(op.name().as_slice().to_vec());
        }
    }
    Some(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SuperArguments, "cops/style/super_arguments");
}
