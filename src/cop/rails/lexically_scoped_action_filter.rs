use crate::cop::node_type::{
    ARRAY_NODE, ASSOC_NODE, CLASS_NODE, DEF_NODE, KEYWORD_HASH_NODE, STATEMENTS_NODE, SYMBOL_NODE,
};
use crate::cop::util::{class_body_calls, has_keyword_arg, is_dsl_call};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct LexicallyScopedActionFilter;

const FILTER_METHODS: &[&[u8]] = &[
    b"before_action",
    b"after_action",
    b"around_action",
    b"skip_before_action",
    b"skip_after_action",
    b"skip_around_action",
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
            ARRAY_NODE,
            ASSOC_NODE,
            CLASS_NODE,
            DEF_NODE,
            KEYWORD_HASH_NODE,
            STATEMENTS_NODE,
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
        let class = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        let body = match class.body() {
            Some(b) => b,
            None => return,
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        // Collect defined method names in this class
        let mut defined_methods: Vec<Vec<u8>> = Vec::new();
        for stmt_node in stmts.body().iter() {
            if let Some(def_node) = stmt_node.as_def_node() {
                defined_methods.push(def_node.name().as_slice().to_vec());
            }
        }

        let calls = class_body_calls(&class);

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

/// Extract action names (as symbol values) from the :only or :except keyword arg.
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

        // Array of symbols: `only: [:show, :edit]`
        if let Some(arr) = value.as_array_node() {
            for elem in arr.elements().iter() {
                if let Some(sym) = elem.as_symbol_node() {
                    results.push((sym.unescaped().to_vec(), sym.location().start_offset()));
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
