use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-04)
///
/// Corpus oracle reported FP=1, FN=0.
///
/// FP=1 was from ternary + inline rescue in
/// `activerecord-hackery__ransack__271cb42/lib/ransack/nodes/value.rb:62`.
/// Prism nests ternary `IfNode` under `RescueModifierNode`, while Parser AST
/// (RuboCop) traverses ternary `if` and `resbody` as siblings under `rescue`.
/// This cop now emulates Parser semantics for the ternary+rescue shape so the
/// two nesting increments do not stack.
///
/// Added fixture coverage in `tests/fixtures/cops/metrics/block_nesting/no_offense.rb`.
/// Local corpus rerun delta vs unchanged baseline binary was repo-local and
/// isolated to the target file (`3 -> 2` offenses in ransack), with no other
/// repo-level count changes.
///
/// ## Failed fix attempt: inline rubocop:disable dedup (2026-03-23, reverted)
///
/// Commit 003b2a06 tried to fix 1 extended-corpus FN by changing how
/// inline `# rubocop:disable Metrics/BlockNesting` interacts with the
/// descendant-offense dedup pass. The idea: RuboCop's `ignore_node` is
/// only called when an offense is actually emitted (not suppressed by a
/// directive), so child offenses inside a disabled parent should still
/// fire. The fix made the visitor always recurse and collect ALL offenses,
/// then dedup only when the parent offense was NOT directive-suppressed.
///
/// This caused +7 FN in the standard corpus (Metrics went from 100% to
/// 99.6%). The dedup logic is tightly coupled to how directives interact
/// with nesting depth — changing it without full directive-aware testing
/// across the corpus is risky. A correct fix needs to reproduce the
/// specific extended-corpus case (GoogleCloudPlatform/inspec-gcp-cis-benchmark
/// controls/1.01-iam.rb:79) and verify against the full standard corpus
/// with `check-cop.py --rerun` before merging.
///
/// ## Corpus investigation (2026-03-25)
///
/// Extended corpus (5592 repos) reported FP=3, FN=270.
///
/// FP=3: All from ruby-rdf/rdf. Root cause: `visit_rescue_modifier_node`
/// added depth for BOTH the expression and rescue_expression, but in
/// RuboCop's Parser AST, modifier rescue creates a `:rescue` node that is
/// NOT in `NESTING_BLOCKS`. Only `:resbody` (the fallback) is counted.
/// The expression and resbody are siblings under `:rescue`, not nested.
/// Fix: visit the expression at the current depth (no increment) and only
/// increment for the rescue_expression (matching `:resbody` semantics).
/// The previous ternary-expression special case was a partial fix for this
/// same issue; the general fix now handles all cases uniformly.
///
/// FN=270: Cross-cutting infrastructure issues. 244 FN from
/// stackneveroverflow (vendored gems), 22 from Tubalr (same), 2 from
/// databasically (extensionless files), 1 from GoogleCloudPlatform
/// (directive dedup, see failed fix above), 1 from pitluga (same class
/// of file discovery issue). No cop-level fix needed for these.
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let max = config.get_usize("Max", 3);
        let count_blocks = config.get_bool("CountBlocks", false);
        let count_modifier_forms = config.get_bool("CountModifierForms", false);

        let mut visitor = NestingVisitor {
            source,
            max,
            count_blocks,
            count_modifier_forms,
            depth: 0,
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
            corrected: false,
        }
    }
}

struct NestingVisitor<'a> {
    source: &'a SourceFile,
    max: usize,
    count_blocks: bool,
    count_modifier_forms: bool,
    depth: usize,
    diagnostics: Vec<Diagnostic>,
}

impl NestingVisitor<'_> {
    /// Check nesting depth and fire offense if exceeded.
    /// Returns true if an offense was fired (caller should skip subtree).
    fn check_nesting(&mut self, loc: &ruby_prism::Location<'_>) -> bool {
        if self.depth > self.max {
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(Diagnostic {
                path: self.source.path_str().to_string(),
                location: crate::diagnostic::Location { line, column },
                severity: crate::diagnostic::Severity::Convention,
                cop_name: "Metrics/BlockNesting".to_string(),
                message: format!("Avoid more than {} levels of block nesting.", self.max),
                corrected: false,
            });
            return true;
        }
        false
    }
}

impl<'pr> Visit<'pr> for NestingVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // RuboCop does NOT reset nesting at method boundaries — it walks the
        // AST recursively, passing current_level through each_child_node without
        // any special handling for def nodes. A def inside nested conditionals
        // inherits the outer nesting depth.
        ruby_prism::visit_def_node(self, node);
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        // In Prism, `elsif` branches are represented as nested IfNodes.
        // RuboCop does not count elsif as additional nesting depth.
        let is_elsif = node
            .if_keyword_loc()
            .is_some_and(|kw| kw.as_slice() == b"elsif");

        // Ternary: `a ? b : c` has no if_keyword_loc (it's None).
        // Modifier if: `foo if bar` has if_keyword_loc but no end_keyword_loc.
        // Only skip modifier forms (not ternaries) when CountModifierForms is false.
        let is_ternary = node.if_keyword_loc().is_none();
        let is_modifier = !is_ternary && node.end_keyword_loc().is_none();
        let should_count = !is_elsif && (self.count_modifier_forms || !is_modifier);

        if should_count {
            self.depth += 1;
            let exceeded = self.check_nesting(&node.location());
            if exceeded {
                // Ignore-subtree: do not recurse into children
                self.depth -= 1;
                return;
            }
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
        let exceeded = self.check_nesting(&node.location());
        if exceeded {
            self.depth -= 1;
            return;
        }
        ruby_prism::visit_unless_node(self, node);
        self.depth -= 1;
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        // RuboCop always counts while/until as nesting, including modifier forms.
        // CountModifierForms only affects if/unless, not while/until.
        self.depth += 1;
        let exceeded = self.check_nesting(&node.location());
        if exceeded {
            self.depth -= 1;
            return;
        }
        ruby_prism::visit_while_node(self, node);
        self.depth -= 1;
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        // RuboCop always counts while/until as nesting, including modifier forms.
        // CountModifierForms only affects if/unless, not while/until.
        self.depth += 1;
        let exceeded = self.check_nesting(&node.location());
        if exceeded {
            self.depth -= 1;
            return;
        }
        ruby_prism::visit_until_node(self, node);
        self.depth -= 1;
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        self.depth += 1;
        let exceeded = self.check_nesting(&node.location());
        if exceeded {
            self.depth -= 1;
            return;
        }
        ruby_prism::visit_case_node(self, node);
        self.depth -= 1;
    }

    fn visit_case_match_node(&mut self, node: &ruby_prism::CaseMatchNode<'pr>) {
        self.depth += 1;
        let exceeded = self.check_nesting(&node.location());
        if exceeded {
            self.depth -= 1;
            return;
        }
        ruby_prism::visit_case_match_node(self, node);
        self.depth -= 1;
    }

    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'pr>) {
        self.depth += 1;
        let exceeded = self.check_nesting(&node.location());
        if exceeded {
            self.depth -= 1;
            return;
        }
        ruby_prism::visit_for_node(self, node);
        self.depth -= 1;
    }

    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        // In Prism, rescue clauses are chained via `subsequent` (each RescueNode
        // contains a pointer to the next one). In the Parser gem AST, `resbody` nodes
        // are siblings under a `rescue` parent. We must NOT increment depth for
        // subsequent rescue clauses — they're at the same nesting level.
        //
        // Manually walk the node: visit statements at incremented depth,
        // then visit subsequent at the ORIGINAL depth.
        self.depth += 1;
        let exceeded = self.check_nesting(&node.location());
        if !exceeded {
            // Visit the rescue body (statements) at incremented depth
            if let Some(stmts) = node.statements() {
                self.visit_statements_node(&stmts);
            }
        }
        self.depth -= 1;

        // Visit subsequent rescue clause at the SAME depth (sibling, not nested)
        if let Some(subsequent) = node.subsequent() {
            self.visit_rescue_node(&subsequent);
        }
    }

    fn visit_rescue_modifier_node(&mut self, node: &ruby_prism::RescueModifierNode<'pr>) {
        let expression = node.expression();
        let rescue_expression = node.rescue_expression();

        // In Parser AST (RuboCop), modifier rescue creates a :rescue node
        // containing the expression and a :resbody as siblings. The :rescue
        // node is NOT in NESTING_BLOCKS, so it doesn't add depth — only
        // :resbody does. Match this by visiting the expression at the current
        // depth and the rescue_expression at depth+1 (resbody equivalent).
        self.visit(&expression);
        self.depth += 1;
        let exceeded = self.check_nesting(&node.keyword_loc());
        if !exceeded {
            self.visit(&rescue_expression);
        }
        self.depth -= 1;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        if self.count_blocks {
            self.depth += 1;
            let exceeded = self.check_nesting(&node.location());
            if exceeded {
                self.depth -= 1;
                return;
            }
            ruby_prism::visit_block_node(self, node);
            self.depth -= 1;
        } else {
            ruby_prism::visit_block_node(self, node);
        }
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        if self.count_blocks {
            self.depth += 1;
            let exceeded = self.check_nesting(&node.location());
            if exceeded {
                self.depth -= 1;
                return;
            }
            ruby_prism::visit_lambda_node(self, node);
            self.depth -= 1;
        } else {
            ruby_prism::visit_lambda_node(self, node);
        }
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
        nested_rescue = "nested_rescue.rb",
        nested_for = "nested_for.rb",
        nested_case_match = "nested_case_match.rb",
        toplevel_nesting = "toplevel_nesting.rb",
        begin_end_while = "begin_end_while.rb",
        ignore_subtree = "ignore_subtree.rb",
        sibling_violations = "sibling_violations.rb",
        modifier_while = "modifier_while.rb",
        modifier_until = "modifier_until.rb",
        inline_rescue = "inline_rescue.rb",
        method_inside_nesting = "method_inside_nesting.rb",
    );
}
