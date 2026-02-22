use std::collections::HashMap;

use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EnumUniqueness;

/// Extract hash elements from enum arguments and check for duplicate values.
fn find_duplicate_values(
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
) -> Vec<(usize, usize, String)> {
    let args = match call.arguments() {
        Some(a) => a,
        None => return Vec::new(),
    };

    let arg_list: Vec<_> = args.arguments().iter().collect();

    // Collect hash elements from either old or new syntax
    let mut hash_elements: Vec<ruby_prism::Node<'_>> = Vec::new();

    for arg in &arg_list {
        // Old syntax: enum status: { active: 0, inactive: 0 }
        if let Some(kw) = arg.as_keyword_hash_node() {
            for elem in kw.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    let val = assoc.value();
                    if let Some(hash) = val.as_hash_node() {
                        for h_elem in hash.elements().iter() {
                            hash_elements.push(h_elem);
                        }
                    }
                }
            }
        }
        // New syntax: enum :status, { active: 0, inactive: 0 }
        if let Some(hash) = arg.as_hash_node() {
            for elem in hash.elements().iter() {
                hash_elements.push(elem);
            }
        }
    }

    if hash_elements.is_empty() {
        return Vec::new();
    }

    // Collect values and detect duplicates
    let mut value_map: HashMap<Vec<u8>, Vec<usize>> = HashMap::new();

    for elem in &hash_elements {
        if let Some(assoc) = elem.as_assoc_node() {
            let val = assoc.value();
            let val_loc = val.location();
            let val_bytes = &source.as_bytes()[val_loc.start_offset()..val_loc.end_offset()];
            value_map
                .entry(val_bytes.to_vec())
                .or_default()
                .push(val_loc.start_offset());
        }
    }

    let mut results = Vec::new();
    for (val_bytes, offsets) in &value_map {
        if offsets.len() > 1 {
            // Report on each duplicate occurrence after the first
            let val_str = String::from_utf8_lossy(val_bytes).to_string();
            for &offset in &offsets[1..] {
                let (line, col) = source.offset_to_line_col(offset);
                results.push((line, col, val_str.clone()));
            }
        }
    }

    results
}

impl Cop for EnumUniqueness {
    fn name(&self) -> &'static str {
        "Rails/EnumUniqueness"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.receiver().is_some() {
            return;
        }

        if call.name().as_slice() != b"enum" {
            return;
        }

        let duplicates = find_duplicate_values(source, &call);

        diagnostics.extend(duplicates.into_iter().map(|(_line, _col, val)| {
            // Report at the call location for simplicity
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            self.diagnostic(
                source,
                line,
                column,
                format!("Duplicate enum value `{val}` detected."),
            )
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EnumUniqueness, "cops/rails/enum_uniqueness");
}
