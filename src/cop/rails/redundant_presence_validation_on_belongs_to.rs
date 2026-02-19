use crate::cop::util::{class_body_calls, is_dsl_call, keyword_arg_value};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, CLASS_NODE, FALSE_NODE, HASH_NODE, KEYWORD_HASH_NODE, NIL_NODE, SYMBOL_NODE, TRUE_NODE};

pub struct RedundantPresenceValidationOnBelongsTo;

impl Cop for RedundantPresenceValidationOnBelongsTo {
    fn name(&self) -> &'static str {
        "Rails/RedundantPresenceValidationOnBelongsTo"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CLASS_NODE, FALSE_NODE, HASH_NODE, KEYWORD_HASH_NODE, NIL_NODE, SYMBOL_NODE, TRUE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let class = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        let calls = class_body_calls(&class);

        // Collect belongs_to association names (only non-optional ones)
        // belongs_to with `optional: true` does NOT add implicit presence validation
        let mut belongs_to_names: Vec<Vec<u8>> = Vec::new();
        for call in &calls {
            if is_dsl_call(call, b"belongs_to") {
                if has_optional_true(call) {
                    continue;
                }
                if let Some(name) = extract_first_symbol_arg(call) {
                    belongs_to_names.push(name);
                }
            }
        }

        if belongs_to_names.is_empty() {
            return;
        }

        // Build both direct name and foreign key name matches
        // belongs_to :problem -> matches :problem and :problem_id
        let mut match_names: Vec<Vec<u8>> = Vec::new();
        for name in &belongs_to_names {
            match_names.push(name.clone());
            let mut fk = name.clone();
            fk.extend_from_slice(b"_id");
            match_names.push(fk);
        }

        // Check validates calls for presence on belongs_to associations
        for call in &calls {
            if !is_dsl_call(call, b"validates") {
                continue;
            }

            // Check that presence: is set to a truthy value.
            // `presence: false` explicitly disables validation and should not be flagged.
            let presence_value = match keyword_arg_value(call, b"presence") {
                Some(v) => v,
                None => continue,
            };
            // Skip if presence: false or presence: nil (explicitly disabling validation)
            if presence_value.as_false_node().is_some() || presence_value.as_nil_node().is_some() {
                continue;
            }

            // Check ALL symbol args (not just the first)
            for name in extract_all_symbol_args(call) {
                if match_names.contains(&name) {
                    let loc = call.message_loc().unwrap_or(call.location());
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let name_str = String::from_utf8_lossy(&name);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Remove explicit `presence` validation for `{name_str}` `belongs_to` association (validated by default since Rails 5)."),
                    ));
                }
            }
        }

    }
}

/// Check if a belongs_to call has `optional: true`
fn has_optional_true(call: &ruby_prism::CallNode<'_>) -> bool {
    let args = match call.arguments() {
        Some(a) => a,
        None => return false,
    };
    for arg in args.arguments().iter() {
        let kw = match arg.as_keyword_hash_node() {
            Some(k) => k,
            None => continue,
        };
        for elem in kw.elements().iter() {
            let assoc = match elem.as_assoc_node() {
                Some(a) => a,
                None => continue,
            };
            let key_sym = match assoc.key().as_symbol_node() {
                Some(s) => s,
                None => continue,
            };
            if key_sym.unescaped() == b"optional" {
                // Check if value is `true`
                if assoc.value().as_true_node().is_some() {
                    return true;
                }
            }
        }
        // Also check HashNode (explicit braces)
        let hash = match arg.as_hash_node() {
            Some(h) => h,
            None => continue,
        };
        for elem in hash.elements().iter() {
            let assoc = match elem.as_assoc_node() {
                Some(a) => a,
                None => continue,
            };
            let key_sym = match assoc.key().as_symbol_node() {
                Some(s) => s,
                None => continue,
            };
            if key_sym.unescaped() == b"optional" {
                if assoc.value().as_true_node().is_some() {
                    return true;
                }
            }
        }
    }
    false
}

fn extract_first_symbol_arg(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let first_arg = args.arguments().iter().next()?;
    let sym = first_arg.as_symbol_node()?;
    Some(sym.unescaped().to_vec())
}

fn extract_all_symbol_args(call: &ruby_prism::CallNode<'_>) -> Vec<Vec<u8>> {
    let mut names = Vec::new();
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            if let Some(sym) = arg.as_symbol_node() {
                names.push(sym.unescaped().to_vec());
            }
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantPresenceValidationOnBelongsTo,
        "cops/rails/redundant_presence_validation_on_belongs_to"
    );
}
