use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct WhereMissing;

/// Collect all call nodes in a method chain, walking from outermost inward.
fn collect_chain_calls<'a>(node: &ruby_prism::Node<'a>) -> Vec<ruby_prism::CallNode<'a>> {
    let mut calls = Vec::new();
    let mut current = node.clone();
    loop {
        if let Some(call) = current.as_call_node() {
            calls.push(call.clone());
            if let Some(recv) = call.receiver() {
                current = recv;
                continue;
            }
        }
        break;
    }
    calls
}

/// Check if a call is `left_joins(:assoc)` or `left_outer_joins(:assoc)`.
/// Returns the association name as bytes if matched.
fn left_joins_assoc<'a>(call: &ruby_prism::CallNode<'a>) -> Option<Vec<u8>> {
    let name = call.name().as_slice();
    if name != b"left_joins" && name != b"left_outer_joins" {
        return None;
    }
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() != 1 {
        return None;
    }
    // Must be a simple symbol argument, not a hash like `left_joins(foo: :bar)`
    let sym = arg_list[0].as_symbol_node()?;
    Some(sym.unescaped().to_vec())
}

/// Check if a `where` call has `assoc_table: { id: nil }` pattern.
/// assoc_name is the singular or plural form.
fn where_has_nil_id_for_assoc(
    call: &ruby_prism::CallNode<'_>,
    assoc_name: &[u8],
) -> bool {
    let args = match call.arguments() {
        Some(a) => a,
        None => return false,
    };

    // Build both singular and plural table names
    let mut table_names: Vec<Vec<u8>> = Vec::new();
    // Plural form (assoc_name + "s")
    let mut plural = assoc_name.to_vec();
    plural.push(b's');
    table_names.push(plural);
    // Singular form (the assoc name itself)
    table_names.push(assoc_name.to_vec());

    for arg in args.arguments().iter() {
        let kw = match arg.as_keyword_hash_node() {
            Some(k) => k,
            None => continue,
        };
        for elem in kw.elements().iter() {
            let assoc_node = match elem.as_assoc_node() {
                Some(a) => a,
                None => continue,
            };
            let key = match assoc_node.key().as_symbol_node() {
                Some(s) => s,
                None => continue,
            };
            let key_name = key.unescaped();
            if !table_names.iter().any(|tn| tn == key_name) {
                continue;
            }
            // Value must be a hash with { id: nil }
            let value = assoc_node.value();
            if let Some(hash) = value.as_hash_node() {
                if hash_has_id_nil(&hash) {
                    return true;
                }
            }
            if let Some(kw_hash) = value.as_keyword_hash_node() {
                if keyword_hash_has_id_nil(&kw_hash) {
                    return true;
                }
            }
        }
    }
    false
}

fn hash_has_id_nil(hash: &ruby_prism::HashNode<'_>) -> bool {
    for elem in hash.elements().iter() {
        if let Some(assoc) = elem.as_assoc_node() {
            if let Some(sym) = assoc.key().as_symbol_node() {
                if sym.unescaped() == b"id" && assoc.value().as_nil_node().is_some() {
                    return true;
                }
            }
        }
    }
    false
}

fn keyword_hash_has_id_nil(hash: &ruby_prism::KeywordHashNode<'_>) -> bool {
    for elem in hash.elements().iter() {
        if let Some(assoc) = elem.as_assoc_node() {
            if let Some(sym) = assoc.key().as_symbol_node() {
                if sym.unescaped() == b"id" && assoc.value().as_nil_node().is_some() {
                    return true;
                }
            }
        }
    }
    false
}

impl Cop for WhereMissing {
    fn name(&self) -> &'static str {
        "Rails/WhereMissing"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // We look for a method chain that contains both:
        // 1. left_joins(:assoc) or left_outer_joins(:assoc)
        // 2. where(assoc_table: { id: nil })
        // These two calls should be in the same chain (not separated by `or` or `and`).

        let calls = collect_chain_calls(node);
        if calls.is_empty() {
            return Vec::new();
        }

        // Find left_joins calls
        let mut left_joins_info: Vec<(usize, Vec<u8>)> = Vec::new();
        for (i, call) in calls.iter().enumerate() {
            if let Some(assoc) = left_joins_assoc(call) {
                left_joins_info.push((i, assoc));
            }
        }

        if left_joins_info.is_empty() {
            return Vec::new();
        }

        // Check if there's also a matching where(...) call with {assoc: {id: nil}}
        // Make sure we don't cross `or` or `and` boundaries
        let mut diagnostics = Vec::new();

        for (lj_idx, assoc_name) in &left_joins_info {
            // Check for a matching where(...) in the chain.
            // Only skip if `or`/`and` appears BETWEEN left_joins and where
            // (in chain index order, not between them in the code).
            let mut found_where = false;

            for (i, call) in calls.iter().enumerate() {
                if i == *lj_idx {
                    continue;
                }
                let name = call.name().as_slice();
                if name == b"where" {
                    if where_has_nil_id_for_assoc(call, assoc_name) {
                        // Check if there's an `or` or `and` between this where and left_joins
                        let min_idx = (*lj_idx).min(i);
                        let max_idx = (*lj_idx).max(i);
                        let has_separator = (min_idx + 1..max_idx).any(|j| {
                            let n = calls[j].name().as_slice();
                            n == b"or" || n == b"and"
                        });
                        if !has_separator {
                            found_where = true;
                            break;
                        }
                    }
                }
            }

            if found_where {
                let lj_call = &calls[*lj_idx];
                let lj_loc = lj_call.message_loc().unwrap_or(lj_call.location());
                let (line, column) = source.offset_to_line_col(lj_loc.start_offset());
                let assoc_str = std::str::from_utf8(assoc_name).unwrap_or("assoc");
                let method_name = std::str::from_utf8(lj_call.name().as_slice())
                    .unwrap_or("left_joins");

                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Use `where.missing(:{assoc_str})` instead of `{method_name}(:{assoc_str}).where({assoc_str}s: {{ id: nil }})`."),
                ));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhereMissing, "cops/rails/where_missing");
}
