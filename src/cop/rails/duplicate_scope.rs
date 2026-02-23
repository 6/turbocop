use std::collections::HashMap;

use crate::cop::node_type::{CLASS_NODE, SYMBOL_NODE};
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let class = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        let calls = class_body_calls(&class);

        // Group scopes by their body expression (everything after the name).
        // RuboCop flags scopes that share the same expression, not the same name.
        let mut seen: HashMap<Vec<u8>, Vec<&ruby_prism::CallNode<'_>>> = HashMap::new();

        for call in &calls {
            if !is_dsl_call(call, b"scope") {
                continue;
            }

            let body_key = match extract_scope_body_source(call) {
                Some(k) => k,
                None => continue,
            };

            seen.entry(body_key).or_default().push(call);
        }

        for calls in seen.values() {
            if calls.len() < 2 {
                continue;
            }
            // Flag all scopes in the group (RuboCop flags every duplicate)
            for call in calls {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Multiple scopes share this same expression.".to_string(),
                ));
            }
        }
    }
}

/// Extract the source bytes of the scope body expression (everything after the
/// first argument, i.e., the scope name). Returns None if the call doesn't have
/// at least two arguments.
fn extract_scope_body_source<'a>(call: &ruby_prism::CallNode<'a>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() < 2 {
        return None;
    }
    // Body is everything from the second argument to the end of the last argument
    let start = arg_list[1].location().start_offset();
    let end = arg_list.last().unwrap().location().end_offset();
    Some(
        call.location().as_slice()
            [start - call.location().start_offset()..end - call.location().start_offset()]
            .to_vec(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateScope, "cops/rails/duplicate_scope");
}
