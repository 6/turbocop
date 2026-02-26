use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SignalException;

impl Cop for SignalException {
    fn name(&self) -> &'static str {
        "Style/SignalException"
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
        let enforced_style = config.get_str("EnforcedStyle", "only_raise");

        let mut visitor = SignalExceptionVisitor {
            cop: self,
            source,
            enforced_style,
            custom_fail_defined: false,
            pending_fail_diagnostics: Vec::new(),
            raise_diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());

        // Emit raise diagnostics unconditionally (only_fail style)
        diagnostics.extend(visitor.raise_diagnostics);

        // Only emit fail diagnostics if no custom fail method is defined
        if !visitor.custom_fail_defined {
            diagnostics.extend(visitor.pending_fail_diagnostics);
        }
    }
}

struct SignalExceptionVisitor<'a> {
    cop: &'a SignalException,
    source: &'a SourceFile,
    enforced_style: &'a str,
    custom_fail_defined: bool,
    /// Diagnostics for bare `fail` calls (only emitted if no custom fail defined)
    pending_fail_diagnostics: Vec<Diagnostic>,
    /// Diagnostics for bare `raise` calls (always emitted for only_fail style)
    raise_diagnostics: Vec<Diagnostic>,
}

impl Visit<'_> for SignalExceptionVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'_>) {
        if node.name().as_slice() == b"fail" {
            self.custom_fail_defined = true;
        }
        // Continue visiting children
        ruby_prism::visit_def_node(self, node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'_>) {
        // Only bare raise/fail (no receiver)
        if node.receiver().is_none() {
            let name = node.name().as_slice();
            let loc = node.message_loc().unwrap_or_else(|| node.location());

            match self.enforced_style {
                "only_raise" => {
                    if name == b"fail" {
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.pending_fail_diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Use `raise` instead of `fail` to rethrow exceptions.".to_string(),
                        ));
                    }
                }
                "only_fail" => {
                    if name == b"raise" {
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.raise_diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Use `fail` instead of `raise` to rethrow exceptions.".to_string(),
                        ));
                    }
                }
                _ => {}
            }
        }

        // Continue visiting children
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full_with_config;

    crate::cop_fixture_tests!(SignalException, "cops/style/signal_exception");

    #[test]
    fn config_only_fail() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("only_fail".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"raise RuntimeError, \"msg\"\n";
        let diags = run_cop_full_with_config(&SignalException, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Use `fail`"));
    }
}
