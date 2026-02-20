use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, NIL_NODE, SYMBOL_NODE};

pub struct WhereMissing;

/// Information about a call in a method chain.
#[allow(dead_code)]
struct ChainCallInfo {
    name: Vec<u8>,
    start_offset: usize,
    msg_offset: usize,
    assoc_name: Option<Vec<u8>>,       // For left_joins(:assoc) calls
    where_nil_assocs: Vec<Vec<u8>>,    // For where(assoc: { id: nil }) — which table names matched
}

/// Walk a method chain and collect info about each call.
fn collect_chain_info(node: &ruby_prism::Node<'_>) -> Vec<ChainCallInfo> {
    let mut infos = Vec::new();
    collect_chain_info_inner(node, &mut infos);
    infos
}

fn collect_chain_info_inner(node: &ruby_prism::Node<'_>, infos: &mut Vec<ChainCallInfo>) {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return,
    };

    let name = call.name().as_slice().to_vec();
    let start_offset = call.location().start_offset();
    let msg_offset = call.message_loc().map(|l| l.start_offset()).unwrap_or(start_offset);

    let assoc_name = left_joins_assoc(&call);
    let where_nil_assocs = if name == b"where" {
        extract_where_nil_assocs(&call)
    } else {
        Vec::new()
    };

    infos.push(ChainCallInfo {
        name,
        start_offset,
        msg_offset,
        assoc_name,
        where_nil_assocs,
    });

    if let Some(recv) = call.receiver() {
        collect_chain_info_inner(&recv, infos);
    }
}

/// Extract association table names from `where(assocs: { id: nil })` patterns.
fn extract_where_nil_assocs(call: &ruby_prism::CallNode<'_>) -> Vec<Vec<u8>> {
    let mut assocs = Vec::new();
    let args = match call.arguments() {
        Some(a) => a,
        None => return assocs,
    };
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
            let value = assoc_node.value();
            let has_nil_id = if let Some(hash) = value.as_hash_node() {
                hash_has_id_nil(&hash)
            } else if let Some(kw_hash) = value.as_keyword_hash_node() {
                keyword_hash_has_id_nil(&kw_hash)
            } else {
                false
            };
            if has_nil_id {
                assocs.push(key.unescaped().to_vec());
            }
        }
    }
    assocs
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, NIL_NODE, SYMBOL_NODE]
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
        // We look for a method chain that contains both:
        // 1. left_joins(:assoc) or left_outer_joins(:assoc)
        // 2. where(assoc_table: { id: nil })
        // These two calls should be in the same chain (not separated by `or` or `and`).

        let chain = collect_chain_info(node);
        if chain.is_empty() {
            return;
        }

        // Only process this node if it's the outermost call in the chain.
        // If chain[0] (this node) is a receiver of another call, a higher-level
        // check_node invocation will handle it. We detect this by only processing
        // when chain[0] is the current node AND is either the left_joins or the
        // matching where — i.e., when index 0 is part of the pattern pair.
        // Simpler approach: only fire if chain[0] is part of the pair.

        // Find left_joins calls
        let left_joins_info: Vec<(usize, &Vec<u8>)> = chain
            .iter()
            .enumerate()
            .filter_map(|(i, info)| info.assoc_name.as_ref().map(|a| (i, a)))
            .collect();

        if left_joins_info.is_empty() {
            return;
        }


        for (lj_idx, assoc_name) in &left_joins_info {
            // Build both singular and plural table names for matching
            let mut plural = (*assoc_name).clone();
            plural.push(b's');

            for (i, info) in chain.iter().enumerate() {
                if i == *lj_idx {
                    continue;
                }
                // Check if this is a where call with matching nil-id assoc
                if !info.where_nil_assocs.iter().any(|a| a.as_slice() == assoc_name.as_slice() || a.as_slice() == plural.as_slice()) {
                    continue;
                }
                // Only fire if the closer-to-root element of the pair is at index 0
                // (the current node). This prevents outer chain calls from duplicating.
                let outermost_idx = (*lj_idx).min(i);
                if outermost_idx != 0 {
                    continue;
                }
                let max_idx = (*lj_idx).max(i);
                let has_separator = (outermost_idx + 1..max_idx).any(|j| {
                    let n = &chain[j].name;
                    n == b"or" || n == b"and"
                });
                if !has_separator {
                    let lj_info = &chain[*lj_idx];
                    let (line, column) = source.offset_to_line_col(lj_info.msg_offset);
                    let assoc_str = std::str::from_utf8(assoc_name).unwrap_or("assoc");
                    let method_name = std::str::from_utf8(&lj_info.name).unwrap_or("left_joins");

                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!(
                            "Use `where.missing(:{assoc_str})` instead of `{method_name}(:{assoc_str}).where({assoc_str}s: {{ id: nil }})`."),
                    ));
                    break;
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhereMissing, "cops/rails/where_missing");
}
