use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct RedundantTravelBack;

impl Cop for RedundantTravelBack {
    fn name(&self) -> &'static str {
        "Rails/RedundantTravelBack"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["spec/**/*.rb"]
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::cop::CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = TravelBackVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_teardown_or_after: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct TravelBackVisitor<'a> {
    cop: &'a RedundantTravelBack,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    in_teardown_or_after: bool,
}

impl<'a, 'pr> Visit<'pr> for TravelBackVisitor<'a> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        // Check if we're entering an `after` block.
        // RuboCop only matches `after do...end` blocks (not `teardown do...end`).
        // For teardown, only `def teardown` is matched (handled in visit_def_node).
        // Shoulda-context `teardown do...end` blocks are NOT flagged by RuboCop.
        let enters_teardown = node.block().is_some()
            && node.receiver().is_none()
            && method_name == b"after";

        // Check if this is a `travel_back` call inside teardown/after
        if self.in_teardown_or_after
            && method_name == b"travel_back"
            && node.receiver().is_none()
        {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Redundant `travel_back` detected. It is automatically called after each test."
                    .to_string(),
            ));
        }

        let was = self.in_teardown_or_after;
        if enters_teardown {
            self.in_teardown_or_after = true;
        }
        ruby_prism::visit_call_node(self, node);
        self.in_teardown_or_after = was;
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Also match `def teardown; ... travel_back; end`
        let is_teardown = node.name().as_slice() == b"teardown";

        let was = self.in_teardown_or_after;
        if is_teardown {
            self.in_teardown_or_after = true;
        }
        ruby_prism::visit_def_node(self, node);
        self.in_teardown_or_after = was;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantTravelBack, "cops/rails/redundant_travel_back");
}
