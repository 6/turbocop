use ruby_prism::Visit;

use crate::cop::factory_bot::FACTORY_BOT_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct FactoryAssociationWithStrategy;

const HARDCODED_STRATEGIES: &[&[u8]] = &[b"create", b"build", b"build_stubbed"];

impl Cop for FactoryAssociationWithStrategy {
    fn name(&self) -> &'static str {
        "FactoryBot/FactoryAssociationWithStrategy"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        FACTORY_BOT_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Match on CallNode for `factory` or `trait`
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method = outer_call.name().as_slice();
        if method != b"factory" && method != b"trait" {
            return Vec::new();
        }

        if outer_call.receiver().is_some() {
            return Vec::new();
        }

        let block = match outer_call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Walk the body looking for attribute blocks with hardcoded strategy calls
        let mut finder = StrategyFinder {
            source,
            cop_name: self.name(),
            diagnostics: Vec::new(),
        };
        finder.visit(&body);

        finder.diagnostics
    }
}

struct StrategyFinder<'s> {
    source: &'s SourceFile,
    cop_name: &'static str,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for StrategyFinder<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check for attribute blocks: `name { strategy(:factory) }`
        // The call must be a bare method (no receiver = attribute name)
        if node.receiver().is_none() {
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
                    // Must have no block params
                    let has_params = block_node
                        .parameters()
                        .and_then(|p| p.as_block_parameters_node())
                        .and_then(|bp| bp.parameters())
                        .map_or(false, |p| p.requireds().iter().next().is_some());

                    if !has_params {
                        if let Some(body) = block_node.body() {
                            let children: Vec<_> =
                                if let Some(stmts) = body.as_statements_node() {
                                    stmts.body().iter().collect()
                                } else {
                                    vec![body]
                                };

                            for child in &children {
                                if let Some(inner_call) = child.as_call_node() {
                                    if inner_call.receiver().is_none() {
                                        let name = inner_call.name().as_slice();
                                        if HARDCODED_STRATEGIES.contains(&name) {
                                            let loc = inner_call.location();
                                            let (line, column) = self
                                                .source
                                                .offset_to_line_col(loc.start_offset());
                                            self.diagnostics.push(Diagnostic {
                                                path: self.source.path_str().to_string(),
                                                location: crate::diagnostic::Location {
                                                    line,
                                                    column,
                                                },
                                                severity: Severity::Convention,
                                                cop_name: self.cop_name.to_string(),
                                                message: "Use an implicit, explicit or inline definition instead of hard coding a strategy for setting association within factory.".to_string(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Continue visiting children (nested factory/trait blocks)
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        FactoryAssociationWithStrategy,
        "cops/factorybot/factory_association_with_strategy"
    );
}
