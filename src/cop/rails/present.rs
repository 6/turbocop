use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{AND_NODE, CALL_NODE, UNLESS_NODE};

pub struct Present;

impl Cop for Present {
    fn name(&self) -> &'static str {
        "Rails/Present"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[AND_NODE, CALL_NODE, UNLESS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let not_nil_and_not_empty = config.get_bool("NotNilAndNotEmpty", true);
        let not_blank = config.get_bool("NotBlank", true);
        let unless_blank = config.get_bool("UnlessBlank", true);

        // Check for `unless foo.blank?` => `if foo.present?` (UnlessBlank)
        if unless_blank {
            if let Some(diag) = self.check_unless_blank(source, node) {
                diagnostics.push(diag);
            }
        }

        // Check for `!nil? && !empty?` => `present?` (NotNilAndNotEmpty)
        if not_nil_and_not_empty {
            if let Some(diag) = self.check_not_nil_and_not_empty(source, node) {
                diagnostics.push(diag);
            }
        }

        // Check for `!blank?` => `present?` (NotBlank)
        if !not_blank {
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"!" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if inner_call.name().as_slice() != b"blank?" {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `present?` instead of `!blank?`.".to_string(),
        ));
    }
}

impl Present {
    /// Check for `unless foo.blank?` pattern.
    fn check_unless_blank(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
    ) -> Option<Diagnostic> {
        let unless_node = node.as_unless_node()?;
        // Predicate should be `foo.blank?`
        let predicate = unless_node.predicate();
        let pred_call = predicate.as_call_node()?;
        if pred_call.name().as_slice() != b"blank?" {
            return None;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        Some(self.diagnostic(
            source,
            line,
            column,
            "Use `if present?` instead of `unless blank?`.".to_string(),
        ))
    }

    /// Check for `!foo.nil? && !foo.empty?` pattern.
    fn check_not_nil_and_not_empty(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
    ) -> Option<Diagnostic> {
        let and_node = node.as_and_node()?;

        // Left: !foo.nil? (call to ! on nil?)
        let left = and_node.left();
        let left_not = left.as_call_node()?;
        if left_not.name().as_slice() != b"!" {
            return None;
        }
        let left_inner = left_not.receiver()?;
        let left_pred = left_inner.as_call_node()?;
        if left_pred.name().as_slice() != b"nil?" {
            return None;
        }

        // Right: !foo.empty? (call to ! on empty?)
        let right = and_node.right();
        let right_not = right.as_call_node()?;
        if right_not.name().as_slice() != b"!" {
            return None;
        }
        let right_inner = right_not.receiver()?;
        let right_pred = right_inner.as_call_node()?;
        if right_pred.name().as_slice() != b"empty?" {
            return None;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        Some(self.diagnostic(
            source,
            line,
            column,
            "Use `present?` instead of `!nil? && !empty?`.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Present, "cops/rails/present");

    #[test]
    fn not_blank_false_skips_bang_blank() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "NotBlank".to_string(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        let source = b"!x.blank?\n";
        assert_cop_no_offenses_full_with_config(&Present, source, config);
    }

    #[test]
    fn not_nil_and_not_empty_false_skips_pattern() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "NotNilAndNotEmpty".to_string(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        let source = b"!foo.nil? && !foo.empty?\n";
        assert_cop_no_offenses_full_with_config(&Present, source, config);
    }

    #[test]
    fn unless_blank_false_skips_unless() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "UnlessBlank".to_string(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        let source = b"unless x.blank?\n  do_something\nend\n";
        assert_cop_no_offenses_full_with_config(&Present, source, config);
    }
}
