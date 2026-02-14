use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RescueStandardError;

fn check_rescue_node(
    cop: &dyn Cop,
    source: &SourceFile,
    rescue_node: &ruby_prism::RescueNode<'_>,
    enforced_style: &str,
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    let exceptions: Vec<_> = rescue_node.exceptions().iter().collect();

    match enforced_style {
        "implicit" => {
            if exceptions.len() == 1 {
                if let Some(const_read) = exceptions[0].as_constant_read_node() {
                    if const_read.name().as_slice() == b"StandardError" {
                        let kw_loc = rescue_node.keyword_loc();
                        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                        diags.push(cop.diagnostic(source, line, column, "Omit the error class when rescuing `StandardError` by itself.".to_string()));
                    }
                }
            }
        }
        "explicit" => {
            if exceptions.is_empty() {
                let kw_loc = rescue_node.keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                diags.push(cop.diagnostic(source, line, column, "Specify `StandardError` explicitly when rescuing.".to_string()));
            }
        }
        _ => {}
    }

    // Check subsequent rescue clauses in the chain
    if let Some(subsequent) = rescue_node.subsequent() {
        diags.extend(check_rescue_node(cop, source, &subsequent, enforced_style));
    }

    diags
}

impl Cop for RescueStandardError {
    fn name(&self) -> &'static str {
        "Style/RescueStandardError"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let begin_node = match node.as_begin_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let rescue_clause = match begin_node.rescue_clause() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let enforced_style = config.get_str("EnforcedStyle", "implicit");

        check_rescue_node(self, source, &rescue_clause, enforced_style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};

    crate::cop_fixture_tests!(RescueStandardError, "cops/style/rescue_standard_error");

    #[test]
    fn explicit_style_flags_bare_rescue() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("explicit".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"begin\n  foo\nrescue\n  bar\nend\n";
        let diags = run_cop_full_with_config(&RescueStandardError, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Specify"));
    }

    #[test]
    fn multiple_exceptions_not_flagged() {
        let source = b"begin\n  foo\nrescue StandardError, RuntimeError\n  bar\nend\n";
        let diags = run_cop_full(&RescueStandardError, source);
        assert!(diags.is_empty());
    }
}
