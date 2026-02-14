use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct PluckInWhere;

impl Cop for PluckInWhere {
    fn name(&self) -> &'static str {
        "Rails/PluckInWhere"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"where" {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Look for pluck inside argument values (keyword hash args)
        for arg in args.arguments().iter() {
            if self.has_pluck_call(&arg) {
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use a subquery instead of `pluck` inside `where`.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

impl PluckInWhere {
    fn has_pluck_call(&self, node: &ruby_prism::Node<'_>) -> bool {
        // Direct pluck call
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"pluck" {
                return true;
            }
        }
        // Check keyword hash values
        if let Some(kw) = node.as_keyword_hash_node() {
            for elem in kw.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(call) = assoc.value().as_call_node() {
                        if call.name().as_slice() == b"pluck" {
                            return true;
                        }
                    }
                }
            }
        }
        // Check hash literal values
        if let Some(hash) = node.as_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(call) = assoc.value().as_call_node() {
                        if call.name().as_slice() == b"pluck" {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PluckInWhere, "cops/rails/pluck_in_where");
}
