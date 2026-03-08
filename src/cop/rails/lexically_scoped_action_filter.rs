use crate::cop::node_type::{
    ALIAS_METHOD_NODE, ARRAY_NODE, ASSOC_NODE, CLASS_NODE, DEF_NODE, KEYWORD_HASH_NODE,
    MODULE_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE,
};
use crate::cop::util::{has_keyword_arg, is_dsl_call};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/LexicallyScopedActionFilter checks that methods specified in
/// filter's `only` or `except` options are defined within the same class
/// or module.
///
/// ## Investigation findings
///
/// **FP root cause (29 FP):** nitrocop did not recognize `delegate :name, to: :obj`,
/// `alias_method :new, :old`, or `alias new old` as defining action methods.
/// RuboCop's `defined_action_methods` includes delegated and aliased methods.
///
/// **FN root cause (56 FN):**
/// - Missing filter methods: `append_after_action`, `append_around_action`,
///   `append_before_action`, `prepend_after_action`, `prepend_around_action`,
///   `prepend_before_action`, `skip_action_callback` were not in FILTER_METHODS.
/// - String values in `only`/`except` (e.g., `only: ['show']`) were not handled;
///   only SymbolNode was checked, not StringNode.
/// - ModuleNode context was not handled (only ClassNode).
///
/// **Fixes applied:**
/// - Added all 13 RESTRICT_ON_SEND methods from vendor RuboCop.
/// - Added delegate/alias_method/alias recognition to defined method collection.
/// - Added StringNode handling in extract_action_names_from_keyword.
/// - Added ModuleNode support alongside ClassNode.
pub struct LexicallyScopedActionFilter;

const FILTER_METHODS: &[&[u8]] = &[
    b"after_action",
    b"append_after_action",
    b"append_around_action",
    b"append_before_action",
    b"around_action",
    b"before_action",
    b"prepend_after_action",
    b"prepend_around_action",
    b"prepend_before_action",
    b"skip_action_callback",
    b"skip_after_action",
    b"skip_around_action",
    b"skip_before_action",
];

impl Cop for LexicallyScopedActionFilter {
    fn name(&self) -> &'static str {
        "Rails/LexicallyScopedActionFilter"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["app/controllers/**/*.rb"]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ALIAS_METHOD_NODE,
            ARRAY_NODE,
            ASSOC_NODE,
            CLASS_NODE,
            DEF_NODE,
            KEYWORD_HASH_NODE,
            MODULE_NODE,
            STATEMENTS_NODE,
            STRING_NODE,
            SYMBOL_NODE,
        ]
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
        // Handle both ClassNode and ModuleNode
        let body = if let Some(class) = node.as_class_node() {
            class.body()
        } else if let Some(module) = node.as_module_node() {
            module.body()
        } else {
            return;
        };

        let body = match body {
            Some(b) => b,
            None => return,
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        // Collect defined method names in this class/module
        let mut defined_methods: Vec<Vec<u8>> = Vec::new();
        for stmt_node in stmts.body().iter() {
            if let Some(def_node) = stmt_node.as_def_node() {
                defined_methods.push(def_node.name().as_slice().to_vec());
            }
        }

        // Collect delegated methods: `delegate :name, to: :obj`
        // and alias methods: `alias_method :new, :old`
        for stmt_node in stmts.body().iter() {
            if let Some(call) = stmt_node.as_call_node() {
                if call.receiver().is_none() {
                    let method_name = call.name().as_slice();
                    if method_name == b"delegate" {
                        collect_delegated_methods(&call, &mut defined_methods);
                    } else if method_name == b"alias_method" {
                        collect_alias_method(&call, &defined_methods.clone(), &mut defined_methods);
                    }
                }
            }
            // Handle `alias new old` (AliasMethodNode in Prism)
            if let Some(alias_node) = stmt_node.as_alias_method_node() {
                collect_alias_node(&alias_node, &defined_methods.clone(), &mut defined_methods);
            }
        }

        // Collect filter calls from class/module body
        let calls: Vec<ruby_prism::CallNode<'_>> = stmts
            .body()
            .iter()
            .filter_map(|node| node.as_call_node())
            .collect();

        for call in &calls {
            let is_filter = FILTER_METHODS.iter().any(|&m| is_dsl_call(call, m));
            if !is_filter {
                continue;
            }

            // Check :only and :except keyword args for symbol references
            for keyword in &[b"only".as_slice(), b"except".as_slice()] {
                if !has_keyword_arg(call, keyword) {
                    continue;
                }

                let action_names = extract_action_names_from_keyword(call, keyword);
                for (name, offset) in action_names {
                    if !defined_methods.contains(&name) {
                        let (line, column) = source.offset_to_line_col(offset);
                        let name_str = String::from_utf8_lossy(&name);
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Action `{name_str}` is not defined in this controller."),
                        ));
                    }
                }
            }
        }
    }
}

/// Collect method names from `delegate :name1, :name2, to: :obj`
fn collect_delegated_methods(call: &ruby_prism::CallNode<'_>, defined_methods: &mut Vec<Vec<u8>>) {
    let args = match call.arguments() {
        Some(a) => a,
        None => return,
    };

    // delegate takes symbol args followed by a keyword hash with `to:`
    // Check that the last arg is a keyword hash with `to:` key
    let arg_list: Vec<_> = args.arguments().iter().collect();
    let has_to_key = arg_list.iter().any(|arg| {
        if let Some(kw) = arg.as_keyword_hash_node() {
            kw.elements().iter().any(|elem| {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(key_sym) = assoc.key().as_symbol_node() {
                        return key_sym.unescaped() == b"to";
                    }
                }
                false
            })
        } else {
            false
        }
    });

    if !has_to_key {
        return;
    }

    // Collect all symbol arguments (the delegated method names)
    for arg in args.arguments().iter() {
        if let Some(sym) = arg.as_symbol_node() {
            defined_methods.push(sym.unescaped().to_vec());
        }
    }
}

/// Collect alias from `alias_method :new_name, :old_name`
/// Only adds new_name if old_name is in defined_methods
fn collect_alias_method(
    call: &ruby_prism::CallNode<'_>,
    current_defined: &[Vec<u8>],
    defined_methods: &mut Vec<Vec<u8>>,
) {
    let args = match call.arguments() {
        Some(a) => a,
        None => return,
    };
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() != 2 {
        return;
    }

    let new_name = if let Some(sym) = arg_list[0].as_symbol_node() {
        sym.unescaped().to_vec()
    } else {
        return;
    };

    let old_name = if let Some(sym) = arg_list[1].as_symbol_node() {
        sym.unescaped().to_vec()
    } else {
        return;
    };

    if current_defined.contains(&old_name) {
        defined_methods.push(new_name);
    }
}

/// Collect alias from `alias new_name old_name` (AliasMethodNode)
/// Only adds new_name if old_name is in defined_methods
fn collect_alias_node(
    alias_node: &ruby_prism::AliasMethodNode<'_>,
    current_defined: &[Vec<u8>],
    defined_methods: &mut Vec<Vec<u8>>,
) {
    // In Prism, `alias new old` produces AliasMethodNode with
    // new_name() and old_name() which are SymbolNode
    let new_name_node = alias_node.new_name();
    let old_name_node = alias_node.old_name();

    let new_name = if let Some(sym) = new_name_node.as_symbol_node() {
        sym.unescaped().to_vec()
    } else {
        return;
    };

    let old_name = if let Some(sym) = old_name_node.as_symbol_node() {
        sym.unescaped().to_vec()
    } else {
        return;
    };

    if current_defined.contains(&old_name) {
        defined_methods.push(new_name);
    }
}

/// Extract action names (as symbol or string values) from the :only or :except keyword arg.
/// Returns (name_bytes, symbol_offset) pairs.
/// RuboCop's pattern requires the keyword hash to contain ONLY the only:/except: pair,
/// so we skip hashes that have additional keys like `if:`, `unless:`, etc.
fn extract_action_names_from_keyword(
    call: &ruby_prism::CallNode<'_>,
    key: &[u8],
) -> Vec<(Vec<u8>, usize)> {
    let mut results = Vec::new();

    let args = match call.arguments() {
        Some(a) => a,
        None => return results,
    };

    for arg in args.arguments().iter() {
        let kw = match arg.as_keyword_hash_node() {
            Some(k) => k,
            None => continue,
        };

        // RuboCop's NodePattern `(hash (pair (sym {:only :except}) $_))`
        // matches only when the hash has exactly one pair
        let elements: Vec<_> = kw.elements().iter().collect();
        if elements.len() != 1 {
            continue;
        }

        let assoc = match elements[0].as_assoc_node() {
            Some(a) => a,
            None => continue,
        };
        let key_sym = match assoc.key().as_symbol_node() {
            Some(s) => s,
            None => continue,
        };
        if key_sym.unescaped() != key {
            continue;
        }

        let value = assoc.value();

        // Single symbol: `only: :show`
        if let Some(sym) = value.as_symbol_node() {
            results.push((sym.unescaped().to_vec(), sym.location().start_offset()));
        }

        // Single string: `only: 'show'`
        if let Some(str_node) = value.as_string_node() {
            results.push((
                str_node.unescaped().to_vec(),
                str_node.location().start_offset(),
            ));
        }

        // Array of symbols/strings: `only: [:show, :edit]` or `only: ['show', 'edit']`
        if let Some(arr) = value.as_array_node() {
            for elem in arr.elements().iter() {
                if let Some(sym) = elem.as_symbol_node() {
                    results.push((sym.unescaped().to_vec(), sym.location().start_offset()));
                }
                if let Some(str_node) = elem.as_string_node() {
                    results.push((
                        str_node.unescaped().to_vec(),
                        str_node.location().start_offset(),
                    ));
                }
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        LexicallyScopedActionFilter,
        "cops/rails/lexically_scoped_action_filter"
    );
}
