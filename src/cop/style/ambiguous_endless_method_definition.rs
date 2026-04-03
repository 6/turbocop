use crate::cop::shared::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for ambiguous endless method definitions that use low-precedence operators.
///
/// ## Corpus investigation (2026-03-11)
///
/// Corpus oracle reported FP=13, FN=0.
///
/// FP root cause 1: the old line-based parser treated any one-line `def` containing
/// ` = ` plus a later `if`/`or` token as an endless method, so regular method
/// definitions with default arguments or inline assignments were misclassified.
///
/// FP root cause 2: RuboCop only checks `def`, not singleton `defs`. Prism represents
/// both with `DefNode`, using `receiver().is_some()` for singleton methods, so nitrocop
/// was still flagging `def obj.foo = ... unless ...` cases that RuboCop ignores.
///
/// Fix: keep the Ruby >= 3.0 gate, inspect only the source tail that appears *after* an
/// actual endless instance-method `def` node's range, and skip receiver-bearing defs.
pub struct AmbiguousEndlessMethodDefinition;

impl Cop for AmbiguousEndlessMethodDefinition {
    fn name(&self) -> &'static str {
        "Style/AmbiguousEndlessMethodDefinition"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // RuboCop: minimum_target_ruby_version 3.0
        // Endless methods were introduced in Ruby 3.0
        let ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(2.7);
        if ruby_version < 3.0 {
            return;
        }

        let def_node = match node.as_def_node() {
            Some(def_node) => def_node,
            None => return,
        };
        if def_node.receiver().is_some()
            || def_node.equal_loc().is_none()
            || def_node.end_keyword_loc().is_some()
        {
            return;
        }

        let loc = def_node.location();
        let source_bytes = source.as_bytes();
        let tail_start = loc.end_offset();
        let mut tail_end = tail_start;
        while tail_end < source_bytes.len() && source_bytes[tail_end] != b'\n' {
            tail_end += 1;
        }

        let tail = match std::str::from_utf8(&source_bytes[tail_start..tail_end]) {
            Ok(tail) => tail,
            Err(_) => return,
        };
        let tail = tail.split('#').next().unwrap_or("").trim_end();
        if tail.trim_start().starts_with(')') {
            return;
        }

        let low_precedence_ops = [
            (" and ", "and"),
            (" or ", "or"),
            (" if ", "if"),
            (" unless ", "unless"),
            (" while ", "while"),
            (" until ", "until"),
        ];

        let Some((_, op_name)) = low_precedence_ops
            .iter()
            .filter_map(|(needle, op_name)| tail.find(needle).map(|idx| (idx, *op_name)))
            .min_by_key(|(idx, _)| *idx)
        else {
            return;
        };

        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Avoid using `{}` statements with endless methods.", op_name),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;

    fn ruby30_config() -> CopConfig {
        let mut config = CopConfig::default();
        config.options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::Value::Number(serde_yml::Number::from(3.0)),
        );
        config
    }

    #[test]
    fn offense_with_ruby30() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &AmbiguousEndlessMethodDefinition,
            include_bytes!(
                "../../../tests/fixtures/cops/style/ambiguous_endless_method_definition/offense.rb"
            ),
            ruby30_config(),
        );
    }

    #[test]
    fn no_offense_with_ruby30() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &AmbiguousEndlessMethodDefinition,
            include_bytes!(
                "../../../tests/fixtures/cops/style/ambiguous_endless_method_definition/no_offense.rb"
            ),
            ruby30_config(),
        );
    }

    #[test]
    fn no_offense_below_ruby30() {
        // Default Ruby version (2.7) — cop should be completely silent
        crate::testutil::assert_cop_no_offenses_full(
            &AmbiguousEndlessMethodDefinition,
            include_bytes!(
                "../../../tests/fixtures/cops/style/ambiguous_endless_method_definition/offense.rb"
            ),
        );
    }
}
