use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantMerge;

impl Cop for RedundantMerge {
    fn name(&self) -> &'static str {
        "Performance/RedundantMerge"
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

        if call.name().as_slice() != b"merge!" {
            return Vec::new();
        }

        // Must have a receiver (hash.merge!)
        if call.receiver().is_none() {
            return Vec::new();
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();

        // Check for single keyword argument: merge!(a: 1)
        // In Prism, keyword args are wrapped in a KeywordHashNode
        let is_single_kv = if args.len() == 1 {
            let first = args.iter().next().unwrap();
            if let Some(kw_hash) = first.as_keyword_hash_node() {
                kw_hash.elements().len() == 1
            } else if let Some(hash) = first.as_hash_node() {
                hash.elements().len() == 1
            } else {
                false
            }
        } else {
            false
        };

        if !is_single_kv {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, "Use `[]=` instead of `merge!` with a single key-value pair.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantMerge, "cops/performance/redundant_merge");
}
