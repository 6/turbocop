use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE};

pub struct WhereNotWithMultipleConditions;

fn hash_has_multiple_pairs(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(hash) = node.as_hash_node() {
        let pairs: Vec<_> = hash.elements().iter().collect();
        if pairs.len() >= 2 {
            return true;
        }
        // Check for nested hash with multiple pairs
        if pairs.len() == 1 {
            if let Some(assoc) = pairs[0].as_assoc_node() {
                let val = assoc.value();
                if let Some(inner_hash) = val.as_hash_node() {
                    let inner_pairs: Vec<_> = inner_hash.elements().iter().collect();
                    return inner_pairs.len() >= 2;
                }
            }
        }
        return false;
    }
    if let Some(kw_hash) = node.as_keyword_hash_node() {
        let pairs: Vec<_> = kw_hash.elements().iter().collect();
        if pairs.len() >= 2 {
            return true;
        }
        if pairs.len() == 1 {
            if let Some(assoc) = pairs[0].as_assoc_node() {
                let val = assoc.value();
                if let Some(inner_hash) = val.as_hash_node() {
                    let inner_pairs: Vec<_> = inner_hash.elements().iter().collect();
                    return inner_pairs.len() >= 2;
                }
            }
        }
    }
    false
}

impl Cop for WhereNotWithMultipleConditions {
    fn name(&self) -> &'static str {
        "Rails/WhereNotWithMultipleConditions"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
    ) -> Vec<Diagnostic> {
        let chain = match util::as_method_chain(node) {
            Some(c) => c,
            None => return Vec::new(),
        };

        // outer must be `not`, inner must be `where`
        if chain.outer_method != b"not" || chain.inner_method != b"where" {
            return Vec::new();
        }

        // The `not` call must have hash arguments with multiple conditions
        let call = node.as_call_node().unwrap();
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        if !hash_has_multiple_pairs(&arg_list[0]) {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use a SQL statement instead of `where.not` with multiple conditions.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhereNotWithMultipleConditions, "cops/rails/where_not_with_multiple_conditions");
}
