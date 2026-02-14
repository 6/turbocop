use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct IfUnlessModifier;

impl Cop for IfUnlessModifier {
    fn name(&self) -> &'static str {
        "Style/IfUnlessModifier"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Extract keyword location, predicate, statements, has_else, and keyword name
        // from either IfNode or UnlessNode
        let (kw_loc, predicate, statements, has_else, keyword) =
            if let Some(if_node) = node.as_if_node() {
                let kw_loc = match if_node.if_keyword_loc() {
                    Some(loc) => loc,
                    None => return Vec::new(), // ternary
                };
                (
                    kw_loc,
                    if_node.predicate(),
                    if_node.statements(),
                    if_node.subsequent().is_some(),
                    "if",
                )
            } else if let Some(unless_node) = node.as_unless_node() {
                (
                    unless_node.keyword_loc(),
                    unless_node.predicate(),
                    unless_node.statements(),
                    unless_node.else_clause().is_some(),
                    "unless",
                )
            } else {
                return Vec::new();
            };

        // Must not have an else clause
        if has_else {
            return Vec::new();
        }

        let body = match statements {
            Some(stmts) => stmts,
            None => return Vec::new(),
        };

        let body_stmts = body.body();

        // Must have exactly one statement
        if body_stmts.len() != 1 {
            return Vec::new();
        }

        let body_node = match body_stmts.iter().next() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Check if already modifier form: keyword starts after body
        if kw_loc.start_offset() > body_node.location().start_offset() {
            return Vec::new();
        }

        let max_line_length: usize = config
            .options
            .get("MaxLineLength")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(120);

        // Estimate modifier line length: body + " " + keyword + " " + condition
        let body_text = &source.as_bytes()
            [body_node.location().start_offset()..body_node.location().end_offset()];
        let cond_text = &source.as_bytes()
            [predicate.location().start_offset()..predicate.location().end_offset()];

        let modifier_len = body_text.len() + 1 + keyword.len() + 1 + cond_text.len();

        if modifier_len <= max_line_length {
            let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line, column },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: format!(
                    "Favor modifier `{keyword}` usage when having a single-line body."
                ),
            }];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &IfUnlessModifier,
            include_bytes!("../../../testdata/cops/style/if_unless_modifier/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &IfUnlessModifier,
            include_bytes!("../../../testdata/cops/style/if_unless_modifier/no_offense.rb"),
        );
    }
}
