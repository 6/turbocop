use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for blocks that are known to need more positional arguments than given.
/// By default checks `reduce`/`inject` which need 2 arguments.
pub struct UnexpectedBlockArity;

impl Cop for UnexpectedBlockArity {
    fn name(&self) -> &'static str {
        "Lint/UnexpectedBlockArity"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Read configured methods
        let methods = get_methods(config);

        let mut visitor = ArityVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            methods,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

fn get_methods(config: &CopConfig) -> Vec<(String, usize)> {
    if let Some(hash) = config.get_string_hash("Methods") {
        return hash
            .iter()
            .filter_map(|(k, v)| {
                let arity: usize = v.parse().ok()?;
                Some((k.clone(), arity))
            })
            .collect();
    }
    // Defaults
    vec![
        ("reduce".to_string(), 2),
        ("inject".to_string(), 2),
    ]
}

struct ArityVisitor<'a, 'src> {
    cop: &'a UnexpectedBlockArity,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    methods: Vec<(String, usize)>,
}

impl<'pr> Visit<'pr> for ArityVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check if this call has a block and is one of the configured methods
        if let Some(block) = node.block() {
            if node.receiver().is_some() {
                let method_name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
                if let Some(expected) = self.expected_arity(method_name) {
                    if let Some(block_node) = block.as_block_node() {
                        let actual = count_block_args(&block_node);
                        if actual < expected {
                            let loc = node.location();
                            let (line, column) =
                                self.source.offset_to_line_col(loc.start_offset());
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                format!(
                                    "`{}` expects at least {} positional arguments, got {}.",
                                    method_name, expected, actual
                                ),
                            ));
                        }
                    }
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }
}

impl ArityVisitor<'_, '_> {
    fn expected_arity(&self, method_name: &str) -> Option<usize> {
        for (name, arity) in &self.methods {
            if name == method_name {
                return Some(*arity);
            }
        }
        None
    }
}

fn count_block_args(block: &ruby_prism::BlockNode<'_>) -> usize {
    let params = match block.parameters() {
        Some(p) => p,
        None => return 0,
    };

    let block_params = match params.as_block_parameters_node() {
        Some(bp) => bp,
        None => return 0,
    };

    let parameters = match block_params.parameters() {
        Some(p) => p,
        None => return 0,
    };

    // Check for rest args (splat) - if present, the block accepts unlimited args
    if parameters.rest().is_some() {
        return usize::MAX;
    }

    // Count positional args (required + optional)
    parameters.requireds().len() + parameters.optionals().len()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnexpectedBlockArity, "cops/lint/unexpected_block_arity");
}
