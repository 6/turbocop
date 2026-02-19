use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ARRAY_NODE, CALL_NODE};

pub struct CompoundHash;

impl Cop for CompoundHash {
    fn name(&self) -> &'static str {
        "Security/CompoundHash"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE]
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

        if call.name().as_slice() != b"hash" {
            return Vec::new();
        }

        // Must have no arguments
        if call.arguments().is_some() {
            return Vec::new();
        }

        // Receiver must be an array literal
        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let array_node = match recv.as_array_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let elements: Vec<ruby_prism::Node<'_>> = array_node.elements().iter().collect();

        // RuboCop's CompoundHash cop detects:
        // 1. Manual hash combining with ^/+/*/| inside def hash (COMBINATOR pattern)
        // 2. [single_value].hash (MONUPLE pattern - wrapping single value is redundant)
        // 3. [a.hash, b.hash].hash (REDUNDANT pattern - .hash on elements is redundant)
        //
        // [a, b].hash is the RECOMMENDED pattern - never flag it.

        // Check for monuple: [single_value].hash
        if elements.len() == 1 {
            let msg_loc = call.message_loc().unwrap();
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Delegate hash directly without wrapping in an array when only using a single value."
                    .to_string(),
            )];
        }

        // Check for redundant: all elements call .hash
        if elements.len() >= 2 {
            let all_call_hash = elements.iter().all(|e| {
                if let Some(c) = e.as_call_node() {
                    c.name().as_slice() == b"hash"
                        && c.arguments().is_none()
                        && c.receiver().is_some()
                } else {
                    false
                }
            });
            if all_call_hash {
                let msg_loc = call.message_loc().unwrap();
                let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Calling `.hash` on elements of a hashed array is redundant."
                        .to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(CompoundHash, "cops/security/compound_hash");
}
