use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Corpus investigation (FP=24, FN=5):
///
/// Root causes of 24 FPs:
/// 1. Accepted integer conditions (`when 32`), but RuboCop only allows str/sym.
/// 2. Accepted nil bodies (`when :x; nil`), but RuboCop's `!nil?` excludes them.
/// 3. Did not enforce same-type constraint on conditions and bodies. Mixed
///    true/false, int/float, or string/symbol conditions caused false positives.
///
/// Root cause of original 2 FNs:
/// - Bodies were restricted to scalar literals. RuboCop's `recursive_basic_literal?`
///   also matches arrays and hashes of literals (e.g., `["#BackupSuccess"]`).
///
/// 5 Additional FNs investigated (2026-03-21):
/// - FN #1 & #2 (hash bodies): Cop correctly detects with 3+ branches. Snippets
///   showed only 1 branch, likely truncated in report.
/// - FN #3 (regex bodies): Correctly NOT detected. RuboCop's `recursive_basic_literal?`
///   does not include regex literals.
/// - FN #4 & #5 (array/string bodies): Cop correctly detects with 3+ branches.
///
/// Fixes applied:
/// - Removed integer_node from `is_simple_when` (only str/sym allowed).
/// - Removed nil_node from `when_body_is_simple_value`.
/// - Added `is_recursive_basic_literal` to handle array/hash bodies.
/// - Added same-type check for all condition nodes and all body nodes.
pub struct HashLikeCase;

impl Cop for HashLikeCase {
    fn name(&self) -> &'static str {
        "Style/HashLikeCase"
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
        let min_branches = config.get_usize("MinBranchesCount", 3);
        let mut visitor = HashLikeCaseVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            min_branches,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct HashLikeCaseVisitor<'a, 'src> {
    cop: &'a HashLikeCase,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    min_branches: usize,
}

impl HashLikeCaseVisitor<'_, '_> {
    fn is_simple_when(when_node: &ruby_prism::WhenNode<'_>) -> bool {
        // Must have exactly one condition
        let conditions: Vec<_> = when_node.conditions().iter().collect();
        if conditions.len() != 1 {
            return false;
        }
        // Condition must be a string or symbol literal (RuboCop: str_type? | sym_type?)
        let cond = &conditions[0];
        cond.as_string_node().is_some() || cond.as_symbol_node().is_some()
    }

    /// Matches RuboCop's `[!nil? recursive_basic_literal?]`:
    /// a basic literal (string, symbol, integer, float, true, false) but NOT nil,
    /// or an array/hash whose elements are all recursive basic literals.
    fn is_recursive_basic_literal(node: &ruby_prism::Node<'_>) -> bool {
        // Scalar literals (excluding nil — RuboCop's !nil? constraint)
        if node.as_string_node().is_some()
            || node.as_symbol_node().is_some()
            || node.as_integer_node().is_some()
            || node.as_float_node().is_some()
            || node.as_true_node().is_some()
            || node.as_false_node().is_some()
        {
            return true;
        }
        // Array of literals
        if let Some(arr) = node.as_array_node() {
            return arr
                .elements()
                .iter()
                .all(|el| Self::is_recursive_basic_literal(&el));
        }
        // Hash of literals (HashNode for `{}`, KeywordHashNode for keyword args)
        let hash_elements = node
            .as_hash_node()
            .map(|h| h.elements())
            .or_else(|| node.as_keyword_hash_node().map(|kh| kh.elements()));
        if let Some(elements) = hash_elements {
            return elements.iter().all(|el| {
                if let Some(assoc) = el.as_assoc_node() {
                    Self::is_recursive_basic_literal(&assoc.key())
                        && Self::is_recursive_basic_literal(&assoc.value())
                } else {
                    false
                }
            });
        }
        false
    }

    fn when_body_is_simple_value(when_node: &ruby_prism::WhenNode<'_>) -> bool {
        if let Some(stmts) = when_node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            if body.len() == 1 {
                return Self::is_recursive_basic_literal(&body[0]);
            }
        }
        false
    }

    /// Returns a simple type tag for a node, used to check that all conditions
    /// (or all bodies) share the same AST node type.
    fn node_type_tag(node: &ruby_prism::Node<'_>) -> u8 {
        if node.as_string_node().is_some() {
            1
        } else if node.as_symbol_node().is_some() {
            2
        } else if node.as_integer_node().is_some() {
            3
        } else if node.as_float_node().is_some() {
            4
        } else if node.as_true_node().is_some() {
            5
        } else if node.as_false_node().is_some() {
            6
        } else if node.as_nil_node().is_some() {
            7
        } else if node.as_array_node().is_some() {
            8
        } else if node.as_hash_node().is_some() || node.as_keyword_hash_node().is_some() {
            9
        } else {
            0
        }
    }
}

impl<'pr> Visit<'pr> for HashLikeCaseVisitor<'_, '_> {
    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        // Must have a case subject (predicate) - `case x; when ...`
        // `case; when ...` without subject is a different pattern
        if node.predicate().is_none() {
            ruby_prism::visit_case_node(self, node);
            return;
        }

        // Must not have an else clause — a case with else can't be trivially
        // replaced with a hash lookup
        if node.else_clause().is_some() {
            ruby_prism::visit_case_node(self, node);
            return;
        }

        let conditions: Vec<_> = node.conditions().iter().collect();
        let when_count = conditions.len();

        if when_count < self.min_branches {
            ruby_prism::visit_case_node(self, node);
            return;
        }

        // All when branches must be simple 1:1 mappings
        let all_simple = conditions.iter().all(|c| {
            if let Some(when_node) = c.as_when_node() {
                Self::is_simple_when(&when_node) && Self::when_body_is_simple_value(&when_node)
            } else {
                false
            }
        });

        if !all_simple {
            ruby_prism::visit_case_node(self, node);
            return;
        }

        // RuboCop's nodes_of_same_type?: all condition nodes must share the same
        // AST type, and all body nodes must share the same AST type.
        let mut cond_tags = Vec::new();
        let mut body_tags = Vec::new();
        for c in &conditions {
            if let Some(when_node) = c.as_when_node() {
                for cond in when_node.conditions().iter() {
                    cond_tags.push(Self::node_type_tag(&cond));
                }
                if let Some(stmts) = when_node.statements() {
                    for body_node in stmts.body().iter() {
                        body_tags.push(Self::node_type_tag(&body_node));
                    }
                }
            }
        }
        let same_cond_type = cond_tags.windows(2).all(|w| w[0] == w[1]);
        let same_body_type = body_tags.windows(2).all(|w| w[0] == w[1]);

        if same_cond_type && same_body_type {
            let loc = node.case_keyword_loc();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Consider replacing `case-when` with a hash lookup.".to_string(),
            ));
        }

        ruby_prism::visit_case_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashLikeCase, "cops/style/hash_like_case");
}
