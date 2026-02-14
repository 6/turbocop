use std::collections::HashMap;

use crate::cop::util::{class_body_calls, is_dsl_call};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DuplicateScope;

impl Cop for DuplicateScope {
    fn name(&self) -> &'static str {
        "Rails/DuplicateScope"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let class = match node.as_class_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();
        let calls = class_body_calls(&class);

        let mut seen: HashMap<Vec<u8>, usize> = HashMap::new();

        for call in &calls {
            if !is_dsl_call(call, b"scope") {
                continue;
            }

            let name = match extract_first_symbol_arg(call) {
                Some(n) => n,
                None => continue,
            };

            if seen.contains_key(&name) {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let name_str = String::from_utf8_lossy(&name);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Duplicate scope `{name_str}` detected."),
                ));
            } else {
                seen.insert(name, 0);
            }
        }

        diagnostics
    }
}

fn extract_first_symbol_arg(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let first_arg = args.arguments().iter().next()?;
    let sym = first_arg.as_symbol_node()?;
    Some(sym.unescaped().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateScope, "cops/rails/duplicate_scope");
}
