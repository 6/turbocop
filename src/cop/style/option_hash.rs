use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct OptionHash;

impl Cop for OptionHash {
    fn name(&self) -> &'static str {
        "Style/OptionHash"
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
        let suspicious_names = config
            .get_string_array("SuspiciousParamNames")
            .unwrap_or_else(|| {
                vec![
                    "options".to_string(),
                    "opts".to_string(),
                    "args".to_string(),
                    "params".to_string(),
                    "parameters".to_string(),
                ]
            });
        let _allowlist = config.get_string_array("Allowlist");

        let mut visitor = OptionHashVisitor {
            cop: self,
            source,
            suspicious_names,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct OptionHashVisitor<'a> {
    cop: &'a OptionHash,
    source: &'a SourceFile,
    suspicious_names: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for OptionHashVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(params) = node.parameters() {
            // Check optional parameters with hash defaults
            for opt in params.optionals().iter() {
                if let Some(opt_param) = opt.as_optional_parameter_node() {
                    let name = opt_param.name();
                    let name_str = std::str::from_utf8(name.as_slice()).unwrap_or("");
                    if self.suspicious_names.iter().any(|s| s == name_str) {
                        // Check if default value is a hash
                        let value = opt_param.value();
                        if value.as_hash_node().is_some() || value.as_keyword_hash_node().is_some()
                        {
                            let loc = opt_param.location();
                            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                format!("Use keyword arguments instead of an options hash argument `{name_str}`."),
                            ));
                        }
                    }
                }
            }
        }

        // Visit body
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OptionHash, "cops/style/option_hash");
}
