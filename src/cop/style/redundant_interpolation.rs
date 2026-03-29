use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Prism reports shorthand interpolations like `"#@ivar"`, `"#@@cvar"`, `"#$1"`,
/// and `"#$/"` as `EmbeddedVariableNode` parts instead of `EmbeddedStatementsNode`.
/// RuboCop still treats those as redundant, and it also flags any lone `#{...}`
/// payload, including nested string expressions such as `"#{"foo"}"`.
/// Keep Ruby 3 pattern matching (`in` / `=>`) exempt when `TargetRubyVersion`
/// is above 2.7, while still skipping implicit concatenation and percent arrays.
pub struct RedundantInterpolation;

impl Cop for RedundantInterpolation {
    fn name(&self) -> &'static str {
        "Style/RedundantInterpolation"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = RedundantInterpVisitor {
            cop: self,
            source,
            in_implicit_concat: false,
            in_percent_array: false,
            target_ruby_version: target_ruby_version(config),
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RedundantInterpVisitor<'a, 'src> {
    cop: &'a RedundantInterpolation,
    source: &'src SourceFile,
    in_implicit_concat: bool,
    in_percent_array: bool,
    target_ruby_version: f64,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for RedundantInterpVisitor<'_, '_> {
    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        // Check if this is a %w[] or %W[] or %i[] or %I[] percent array
        let was_in_percent_array = self.in_percent_array;
        if let Some(open_loc) = node.opening_loc() {
            let open_bytes =
                &self.source.as_bytes()[open_loc.start_offset()..open_loc.end_offset()];
            if open_bytes.starts_with(b"%w")
                || open_bytes.starts_with(b"%W")
                || open_bytes.starts_with(b"%i")
                || open_bytes.starts_with(b"%I")
            {
                self.in_percent_array = true;
            }
        }

        // Visit children manually
        for element in node.elements().iter() {
            self.visit(&element);
        }

        self.in_percent_array = was_in_percent_array;
    }

    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        let is_implicit_concat = node.opening_loc().is_none();

        if is_implicit_concat {
            // This is an implicit concatenation node — skip flagging, but visit children
            let was = self.in_implicit_concat;
            self.in_implicit_concat = true;
            for part in node.parts().iter() {
                self.visit(&part);
            }
            self.in_implicit_concat = was;
            return;
        }

        // Skip if inside implicit concatenation or percent array
        if !self.in_implicit_concat && !self.in_percent_array {
            self.check_redundant_interpolation(node);
        }

        // Visit children
        for part in node.parts().iter() {
            self.visit(&part);
        }
    }
}

impl RedundantInterpVisitor<'_, '_> {
    fn add_offense(&mut self, node: &ruby_prism::InterpolatedStringNode<'_>) {
        let loc = node.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Prefer `to_s` over string interpolation.".to_string(),
        ));
    }

    fn uses_match_pattern(&self, embedded: &ruby_prism::EmbeddedStatementsNode<'_>) -> bool {
        if self.target_ruby_version <= 2.7 {
            return false;
        }

        embedded
            .statements()
            .map(|statements| {
                statements.body().iter().any(|node| {
                    node.as_match_predicate_node().is_some()
                        || node.as_match_required_node().is_some()
                })
            })
            .unwrap_or(false)
    }

    fn check_redundant_interpolation(&mut self, node: &ruby_prism::InterpolatedStringNode<'_>) {
        let mut parts = node.parts().iter();
        let Some(part) = parts.next() else {
            return;
        };
        if parts.next().is_some() {
            return;
        }

        if part.as_embedded_variable_node().is_some() {
            self.add_offense(node);
            return;
        }

        let Some(embedded) = part.as_embedded_statements_node() else {
            return;
        };
        if self.uses_match_pattern(&embedded) {
            return;
        }

        self.add_offense(node);
    }
}

fn target_ruby_version(config: &CopConfig) -> f64 {
    config
        .options
        .get("TargetRubyVersion")
        .and_then(|value| {
            value
                .as_f64()
                .or_else(|| value.as_u64().map(|value| value as f64))
        })
        .unwrap_or(2.7)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::assert_cop_no_offenses_full_with_config;
    use std::collections::HashMap;

    crate::cop_fixture_tests!(RedundantInterpolation, "cops/style/redundant_interpolation");

    fn ruby_3_config() -> CopConfig {
        CopConfig {
            options: HashMap::from([(
                "TargetRubyVersion".into(),
                serde_yml::Value::Number(serde_yml::value::Number::from(3.0)),
            )]),
            ..CopConfig::default()
        }
    }

    #[test]
    fn no_offense_for_ruby_3_pattern_matching_interpolation() {
        assert_cop_no_offenses_full_with_config(
            &RedundantInterpolation,
            b"\"#{42 => var}\"\n",
            ruby_3_config(),
        );
        assert_cop_no_offenses_full_with_config(
            &RedundantInterpolation,
            b"\"#{x; 42 => var}\"\n",
            ruby_3_config(),
        );
        assert_cop_no_offenses_full_with_config(
            &RedundantInterpolation,
            b"\"#{42 in var}\"\n",
            ruby_3_config(),
        );
    }
}
