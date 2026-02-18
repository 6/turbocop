use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::collections::HashMap;

pub struct InverseMethods;

impl InverseMethods {
    /// Build the inverse methods map from config or defaults.
    fn build_inverse_map(config: &CopConfig) -> HashMap<Vec<u8>, String> {
        let mut map = HashMap::new();

        if let Some(configured) = config.get_string_hash("InverseMethods") {
            for (key, val) in &configured {
                let k = key.trim_start_matches(':');
                let v = val.trim_start_matches(':');
                map.insert(k.as_bytes().to_vec(), v.to_string());
            }
        } else {
            // RuboCop defaults — note: relationship only defined one direction
            // but we need both directions for lookup
            let defaults: &[(&[u8], &str)] = &[
                (b"any?", "none?"),
                (b"none?", "any?"),
                (b"even?", "odd?"),
                (b"odd?", "even?"),
                (b"==", "!="),
                (b"!=", "=="),
                (b"=~", "!~"),
                (b"!~", "=~"),
                (b"<", ">="),
                (b">=", "<"),
                (b">", "<="),
                (b"<=", ">"),
            ];
            for &(k, v) in defaults {
                map.insert(k.to_vec(), v.to_string());
            }
        }
        map
    }

    fn build_inverse_blocks(config: &CopConfig) -> HashMap<Vec<u8>, String> {
        let mut map = HashMap::new();

        if let Some(configured) = config.get_string_hash("InverseBlocks") {
            for (key, val) in &configured {
                let k = key.trim_start_matches(':');
                let v = val.trim_start_matches(':');
                map.insert(k.as_bytes().to_vec(), v.to_string());
            }
        } else {
            // RuboCop defaults
            let defaults: &[(&[u8], &str)] = &[
                (b"select", "reject"),
                (b"reject", "select"),
            ];
            for &(k, v) in defaults {
                map.insert(k.to_vec(), v.to_string());
            }
        }
        map
    }
}

impl Cop for InverseMethods {
    fn name(&self) -> &'static str {
        "Style/InverseMethods"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_bytes = call.name().as_slice();

        // Pattern: !receiver.method - the call is `!` with the inner being a method call
        if method_bytes != b"!" {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let inner_method = inner_call.name().as_slice();

        // Check InverseMethods (predicate methods: !foo.any? -> foo.none?)
        let inverse_methods = Self::build_inverse_map(config);
        if let Some(inv) = inverse_methods.get(inner_method) {
            let inner_name = std::str::from_utf8(inner_method).unwrap_or("method");
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("Use `{}` instead of inverting `{}`.", inv, inner_name),
            )];
        }

        // Check InverseBlocks (block methods: !foo.select { } -> foo.reject { })
        let inverse_blocks = Self::build_inverse_blocks(config);
        if inner_call.block().is_some() {
            if let Some(inv) = inverse_blocks.get(inner_method) {
                let inner_name = std::str::from_utf8(inner_method).unwrap_or("method");
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `{}` instead of inverting `{}`.", inv, inner_name),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InverseMethods, "cops/style/inverse_methods");
}
