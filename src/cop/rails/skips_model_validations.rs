use crate::cop::node_type::{
    ASSOC_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, FALSE_NODE, HASH_NODE,
    KEYWORD_HASH_NODE, SYMBOL_NODE, TRUE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SkipsModelValidations;

const SKIP_METHODS: &[&[u8]] = &[
    b"update_attribute",
    b"touch",
    b"update_column",
    b"update_columns",
    b"update_all",
    b"toggle!",
    b"increment!",
    b"decrement!",
    b"insert",
    b"insert!",
    b"insert_all",
    b"insert_all!",
    b"upsert",
    b"upsert_all",
    b"increment_counter",
    b"decrement_counter",
    b"update_counters",
];

impl Cop for SkipsModelValidations {
    fn name(&self) -> &'static str {
        "Rails/SkipsModelValidations"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ASSOC_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            FALSE_NODE,
            HASH_NODE,
            KEYWORD_HASH_NODE,
            SYMBOL_NODE,
            TRUE_NODE,
        ]
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
        let forbidden = config.get_string_array("ForbiddenMethods");
        let allowed = config.get_string_array("AllowedMethods");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        let method_name = call.name().as_slice();
        let method_str = std::str::from_utf8(method_name).unwrap_or("");

        // Use ForbiddenMethods if configured, otherwise fall back to hardcoded list
        let is_forbidden = if let Some(ref list) = forbidden {
            list.iter().any(|m| m == method_str)
        } else {
            SKIP_METHODS.contains(&method_name)
        };

        if !is_forbidden {
            return;
        }

        // Skip if method is in AllowedMethods
        if let Some(ref list) = allowed {
            if list.iter().any(|m| m == method_str) {
                return;
            }
        }

        if call.receiver().is_none() {
            return;
        }

        // RuboCop: METHODS_WITH_ARGUMENTS — skip if the method is in this list
        // and has no arguments (e.g. `model.touch` with no args).
        let methods_with_args: &[&[u8]] = &[
            b"decrement!",
            b"decrement_counter",
            b"increment!",
            b"increment_counter",
            b"insert",
            b"insert!",
            b"insert_all",
            b"insert_all!",
            b"toggle!",
            b"update_all",
            b"update_attribute",
            b"update_column",
            b"update_columns",
            b"update_counters",
            b"upsert",
            b"upsert_all",
        ];
        if methods_with_args.contains(&method_name) && call.arguments().is_none() {
            return;
        }

        // RuboCop: good_insert? — for insert/insert!, skip when the second argument
        // is not a hash with :returning or :unique_by keys. This distinguishes
        // String#insert(idx, str) from ActiveRecord insert(attributes_hash).
        if method_name == b"insert" || method_name == b"insert!" {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                // good_insert? pattern: (call _ {:insert :insert!} _ { !(hash ...) | (hash without :returning/:unique_by) } ...)
                // If there are at least 2 args, check the second arg
                if arg_list.len() >= 2 {
                    let second = &arg_list[1];
                    let is_ar_insert = if let Some(hash) = second.as_hash_node() {
                        // It's a hash — only flag if it contains :returning or :unique_by
                        hash.elements().iter().any(|elem| {
                            if let Some(assoc) = elem.as_assoc_node() {
                                if let Some(sym) = assoc.key().as_symbol_node() {
                                    let name: &[u8] = sym.unescaped().as_ref();
                                    return name == b"returning" || name == b"unique_by";
                                }
                            }
                            false
                        })
                    } else if let Some(kw_hash) = second.as_keyword_hash_node() {
                        kw_hash.elements().iter().any(|elem| {
                            if let Some(assoc) = elem.as_assoc_node() {
                                if let Some(sym) = assoc.key().as_symbol_node() {
                                    let name: &[u8] = sym.unescaped().as_ref();
                                    return name == b"returning" || name == b"unique_by";
                                }
                            }
                            false
                        })
                    } else {
                        false // Not a hash — not an AR insert
                    };
                    if !is_ar_insert {
                        return;
                    }
                }
            }
        }

        // RuboCop: good_touch? — FileUtils.touch or _.touch(boolean)
        if method_name == b"touch" {
            if let Some(recv) = call.receiver() {
                if let Some(cr) = recv.as_constant_read_node() {
                    if cr.name().as_slice() == b"FileUtils" {
                        return;
                    }
                }
                if let Some(cp) = recv.as_constant_path_node() {
                    if let Some(name) = cp.name() {
                        if name.as_slice() == b"FileUtils" {
                            return;
                        }
                    }
                }
            }
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() == 1 {
                    let first = &arg_list[0];
                    if first.as_true_node().is_some() || first.as_false_node().is_some() {
                        return;
                    }
                }
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let msg = format!(
            "Avoid `{}` because it skips validations.",
            std::str::from_utf8(method_name).unwrap_or("?")
        );
        diagnostics.push(self.diagnostic(source, line, column, msg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SkipsModelValidations, "cops/rails/skips_model_validations");
}
