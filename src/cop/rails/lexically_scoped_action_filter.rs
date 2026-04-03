use crate::cop::shared::node_type::{
    ALIAS_METHOD_NODE, ARRAY_NODE, ASSOC_NODE, CLASS_NODE, DEF_NODE, KEYWORD_HASH_NODE,
    MODULE_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE,
};
use crate::cop::shared::util::is_dsl_call;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/LexicallyScopedActionFilter checks that methods specified in
/// filter's `only` or `except` options are defined within the same class
/// or module.
///
/// ## Investigation findings
///
/// **FP root causes (28 FP):**
/// - nitrocop emitted one offense per unmatched action name, while RuboCop
///   emits one offense per filter call aggregating all unmatched names.
///   A filter with 3 unmatched methods produced 3 offenses vs RuboCop's 1.
/// - Offense was located at the individual symbol, not the filter call.
///
/// **FN root causes (46 FN):**
/// - Filter calls nested inside blocks (e.g., `included do ... end`) were
///   not found because we only searched direct children of the class body.
///   RuboCop's `on_send` fires on all send nodes and walks up to the parent
///   class/module via `each_ancestor`.
/// - RuboCop's NodePattern matches the hash with exactly one pair, which
///   correctly skips multi-key hashes like `only: :show, if: :admin?`.
///
/// **Fixes applied (round 2):**
/// - Changed to emit one offense per filter call with aggregated message.
/// - Offense now points to the filter call (matching RuboCop location).
/// - Message format changed to match RuboCop: `` `name` is not explicitly
///   defined on the class. `` / `` `a`, `b` are not explicitly defined on
///   the module. ``
/// - Added recursive search for filter calls in nested blocks to handle
///   `included do ... end` pattern.
/// - Maintained existing delegate/alias_method/alias recognition.
///
/// **Fixes applied (round 3): FP=8**
/// - Block forms (`before_action(only: :x) do...end`) and multi-name forms
///   (`skip_before_action :a, :b, only: [...]`) produced FPs. RuboCop's
///   NodePattern `(send nil? {filters} _ (hash (pair (sym {:only :except}) $_)))`
///   requires exactly ONE non-hash positional arg. Fixed by counting non-hash
///   args and skipping if not exactly 1.
///
/// **Fixes applied (round 4): FN=5**
/// - Filter calls wrapped in modifier conditionals (`before_action(:auth,
///   only: :x) unless cond`) were not found because `collect_filter_calls_recursive`
///   only checked direct `CallNode` children, missing calls inside `IfNode` and
///   `UnlessNode` wrappers. Fixed by extracting `collect_filter_call_from_node`
///   that also descends into conditional node bodies.
///
/// **Fixes applied (round 5): FN=3**
/// - `default_include` was missing `**/app/mailers/**/*.rb`. The vendor config
///   includes both `**/app/controllers/**/*.rb` and `**/app/mailers/**/*.rb`.
///   Mailer files that use `before_action` with `only:` were not scanned.
///   Also changed `app/controllers/**/*.rb` to `**/app/controllers/**/*.rb` to
///   match the vendor's glob format with leading `**`.
///
/// **Fixes applied (round 6): FN=3**
/// - `def self.foo` (class methods) were incorrectly counted as defined action
///   methods. RuboCop's `each_child_node(:def)` only matches instance methods;
///   added `receiver().is_none()` check to filter out class methods.
/// - Filter calls inside `def` bodies (metaprogramming patterns) were not found
///   because `collect_filter_call_from_node` didn't descend into `DefNode`.
///   RuboCop's `on_send` fires on ALL send nodes regardless of nesting. Added
///   recursion into `DefNode` bodies to match.
pub struct LexicallyScopedActionFilter;

/// (call_start_offset, only_action_names, except_action_names)
type FilterCallInfo = (usize, Vec<Vec<u8>>, Vec<Vec<u8>>);

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
        &["**/app/controllers/**/*.rb", "**/app/mailers/**/*.rb"]
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
        // Determine if this is a ClassNode or ModuleNode
        let (body, type_name) = if let Some(class) = node.as_class_node() {
            (class.body(), "class")
        } else if let Some(module) = node.as_module_node() {
            (module.body(), "module")
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

        // Collect defined instance method names in this class/module (direct children only).
        // Only count instance methods (receiver is None), not class methods (def self.foo).
        // RuboCop uses `each_child_node(:def)` which only matches instance methods.
        let mut defined_methods: Vec<Vec<u8>> = Vec::new();
        for stmt_node in stmts.body().iter() {
            if let Some(def_node) = stmt_node.as_def_node() {
                if def_node.receiver().is_none() {
                    defined_methods.push(def_node.name().as_slice().to_vec());
                }
            }
        }

        // Collect delegated methods and alias methods (direct children only)
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

        // Find filter calls recursively (handles `included do ... end` blocks)
        // We collect (start_offset, action_names_for_only, action_names_for_except)
        let mut filter_info: Vec<FilterCallInfo> = Vec::new();
        collect_filter_calls_recursive(&stmts, &mut filter_info);

        for (call_offset, only_names, except_names) in &filter_info {
            for action_names in [only_names, except_names] {
                if action_names.is_empty() {
                    continue;
                }

                let unmatched: Vec<&Vec<u8>> = action_names
                    .iter()
                    .filter(|name| !defined_methods.contains(name))
                    .collect();

                if unmatched.is_empty() {
                    continue;
                }

                let (line, column) = source.offset_to_line_col(*call_offset);

                let message = if unmatched.len() == 1 {
                    let name_str = String::from_utf8_lossy(unmatched[0]);
                    format!("`{name_str}` is not explicitly defined on the {type_name}.")
                } else {
                    let names: Vec<String> = unmatched
                        .iter()
                        .map(|n| format!("`{}`", String::from_utf8_lossy(n)))
                        .collect();
                    let joined = names.join(", ");
                    format!("{joined} are not explicitly defined on the {type_name}.")
                };

                diagnostics.push(self.diagnostic(source, line, column, message));
            }
        }
    }
}

/// Recursively collect filter call info from statements, including inside blocks.
/// This handles patterns like `included do before_action ... end`.
/// Collects (call_start_offset, only_action_names, except_action_names) tuples.
fn collect_filter_calls_recursive(
    stmts: &ruby_prism::StatementsNode<'_>,
    results: &mut Vec<FilterCallInfo>,
) {
    for stmt_node in stmts.body().iter() {
        collect_filter_call_from_node(&stmt_node, results);
    }
}

/// Check a single node for filter calls, recursing into blocks, conditionals, and def bodies.
/// RuboCop's `on_send` fires on ALL send nodes in the file and walks up to the nearest
/// class/module ancestor. We approximate this by recursively descending into all nested
/// structures: blocks, if/unless, and method def bodies.
fn collect_filter_call_from_node(node: &ruby_prism::Node<'_>, results: &mut Vec<FilterCallInfo>) {
    if let Some(call) = node.as_call_node() {
        check_call_for_filter(&call, results);
    } else if let Some(if_node) = node.as_if_node() {
        // Modifier `if`: `before_action(:auth, only: :x) if condition`
        if let Some(body) = if_node.statements() {
            for child in body.body().iter() {
                collect_filter_call_from_node(&child, results);
            }
        }
    } else if let Some(unless_node) = node.as_unless_node() {
        // Modifier `unless`: `before_action(:auth, only: :x) unless condition`
        if let Some(body) = unless_node.statements() {
            for child in body.body().iter() {
                collect_filter_call_from_node(&child, results);
            }
        }
    } else if let Some(def_node) = node.as_def_node() {
        // Filter calls inside method bodies (e.g., metaprogramming patterns like
        // `def setup_filters; before_action :foo, only: :bar; end`)
        if let Some(body) = def_node.body() {
            if let Some(inner_stmts) = body.as_statements_node() {
                for child in inner_stmts.body().iter() {
                    collect_filter_call_from_node(&child, results);
                }
            }
        }
    }
}

/// Check if a CallNode is a filter call; if not, recurse into its block body.
fn check_call_for_filter(call: &ruby_prism::CallNode<'_>, results: &mut Vec<FilterCallInfo>) {
    let is_filter = FILTER_METHODS.iter().any(|&m| is_dsl_call(call, m));
    if is_filter {
        let offset = call.location().start_offset();
        let only_names = extract_action_names_from_call(call, b"only");
        let except_names = extract_action_names_from_call(call, b"except");
        if !only_names.is_empty() || !except_names.is_empty() {
            results.push((offset, only_names, except_names));
        }
    } else {
        // Check inside block bodies (e.g., `included do ... end`)
        if let Some(block) = call.block() {
            if let Some(block_node) = block.as_block_node() {
                if let Some(body) = block_node.body() {
                    if let Some(inner_stmts) = body.as_statements_node() {
                        collect_filter_calls_recursive(&inner_stmts, results);
                    }
                }
            }
        }
    }
}

/// Extract action names (as symbol or string values) from the :only or :except keyword arg
/// of a filter call. Returns just the names (no offsets needed since we report on the call).
/// RuboCop's pattern: `(send nil? {filters} _ (hash (pair (sym {:only :except}) $_)))`
/// This requires EXACTLY ONE non-hash positional arg before the keyword hash. Forms with
/// 0 positional args (block-only forms like `before_action(only: :x) do...end`) or 2+
/// positional args (`skip_before_action :a, :b, only: [...]`) are not matched by RuboCop.
fn extract_action_names_from_call(call: &ruby_prism::CallNode<'_>, key: &[u8]) -> Vec<Vec<u8>> {
    let mut results = Vec::new();

    let args = match call.arguments() {
        Some(a) => a,
        None => return results,
    };

    let arg_list: Vec<_> = args.arguments().iter().collect();

    // Must have exactly 1 non-keyword-hash positional arg (the filter proc/symbol).
    let non_hash_count = arg_list
        .iter()
        .filter(|a| a.as_keyword_hash_node().is_none())
        .count();
    if non_hash_count != 1 {
        return results;
    }

    for arg in arg_list.iter() {
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
            results.push(sym.unescaped().to_vec());
        }

        // Single string: `only: 'show'`
        if let Some(str_node) = value.as_string_node() {
            results.push(str_node.unescaped().to_vec());
        }

        // Array of symbols/strings: `only: [:show, :edit]` or `only: ['show', 'edit']`
        if let Some(arr) = value.as_array_node() {
            for elem in arr.elements().iter() {
                if let Some(sym) = elem.as_symbol_node() {
                    results.push(sym.unescaped().to_vec());
                }
                if let Some(str_node) = elem.as_string_node() {
                    results.push(str_node.unescaped().to_vec());
                }
            }
        }
    }

    results
}

/// Collect method names from `delegate :name1, :name2, to: :obj`
fn collect_delegated_methods(call: &ruby_prism::CallNode<'_>, defined_methods: &mut Vec<Vec<u8>>) {
    let args = match call.arguments() {
        Some(a) => a,
        None => return,
    };

    // delegate takes symbol args followed by a keyword hash with `to:`
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

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        LexicallyScopedActionFilter,
        "cops/rails/lexically_scoped_action_filter"
    );
}
