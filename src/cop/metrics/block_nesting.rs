use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct BlockNesting;

impl Cop for BlockNesting {
    fn name(&self) -> &'static str {
        "Metrics/BlockNesting"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let max = config.get_usize("Max", 3);
        let _count_blocks = config.get_bool("CountBlocks", false);
        let count_modifier_forms = config.get_bool("CountModifierForms", false);

        let mut visitor = NestingVisitor {
            source,
            max,
            count_modifier_forms,
            depth: 0,
            in_method: false,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }

    fn diagnostic(
        &self,
        source: &SourceFile,
        line: usize,
        column: usize,
        message: String,
    ) -> Diagnostic {
        Diagnostic {
            path: source.path_str().to_string(),
            location: crate::diagnostic::Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message,
        }
    }
}

struct NestingVisitor<'a> {
    source: &'a SourceFile,
    max: usize,
    count_modifier_forms: bool,
    depth: usize,
    in_method: bool,
    diagnostics: Vec<Diagnostic>,
}

impl NestingVisitor<'_> {
    fn check_nesting(&mut self, loc: &ruby_prism::Location<'_>) {
        if self.in_method && self.depth > self.max {
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(Diagnostic {
                path: self.source.path_str().to_string(),
                location: crate::diagnostic::Location { line, column },
                severity: crate::diagnostic::Severity::Convention,
                cop_name: "Metrics/BlockNesting".to_string(),
                message: format!("Avoid more than {} levels of block nesting.", self.max),
            });
        }
    }
}

impl<'pr> Visit<'pr> for NestingVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let was_in_method = self.in_method;
        let old_depth = self.depth;
        self.in_method = true;
        self.depth = 0;
        ruby_prism::visit_def_node(self, node);
        self.in_method = was_in_method;
        self.depth = old_depth;
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        // In Prism, `elsif` branches are represented as nested IfNodes.
        // RuboCop does not count elsif as additional nesting depth.
        let is_elsif = node
            .if_keyword_loc()
            .is_some_and(|kw| kw.as_slice() == b"elsif");

        // Modifier if (e.g. `foo if bar`) has no `end` keyword.
        // Skip if CountModifierForms is false (default).
        let is_modifier = node.end_keyword_loc().is_none();
        let should_count = !is_elsif && (self.count_modifier_forms || !is_modifier);

        if should_count {
            self.depth += 1;
            self.check_nesting(&node.location());
        }
        ruby_prism::visit_if_node(self, node);
        if should_count {
            self.depth -= 1;
        }
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        // Modifier unless (e.g. `foo unless bar`) has no `end` keyword.
        let is_modifier = node.end_keyword_loc().is_none();
        if !self.count_modifier_forms && is_modifier {
            ruby_prism::visit_unless_node(self, node);
            return;
        }
        self.depth += 1;
        self.check_nesting(&node.location());
        ruby_prism::visit_unless_node(self, node);
        self.depth -= 1;
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        let is_modifier = node.closing_loc().is_none();
        if !self.count_modifier_forms && is_modifier {
            ruby_prism::visit_while_node(self, node);
            return;
        }
        self.depth += 1;
        self.check_nesting(&node.location());
        ruby_prism::visit_while_node(self, node);
        self.depth -= 1;
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        let is_modifier = node.closing_loc().is_none();
        if !self.count_modifier_forms && is_modifier {
            ruby_prism::visit_until_node(self, node);
            return;
        }
        self.depth += 1;
        self.check_nesting(&node.location());
        ruby_prism::visit_until_node(self, node);
        self.depth -= 1;
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        self.depth += 1;
        self.check_nesting(&node.location());
        ruby_prism::visit_case_node(self, node);
        self.depth -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_scenario_fixture_tests!(
        BlockNesting,
        "cops/metrics/block_nesting",
        nested_ifs = "nested_ifs.rb",
        nested_unless = "nested_unless.rb",
        nested_while = "nested_while.rb",
    );
}
