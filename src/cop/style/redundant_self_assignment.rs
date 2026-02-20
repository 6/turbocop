use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, LOCAL_VARIABLE_READ_NODE, LOCAL_VARIABLE_WRITE_NODE};

pub struct RedundantSelfAssignment;

/// In-place modification methods where `x = x.method` is redundant
const INPLACE_METHODS: &[&[u8]] = &[
    b"map!", b"collect!", b"flat_map!", b"collect_concat!",
    b"compact!", b"flatten!", b"reject!", b"select!", b"filter!",
    b"sort!", b"sort_by!", b"uniq!", b"reverse!", b"rotate!",
    b"shuffle!", b"slice!", b"delete_if!", b"keep_if!",
    b"gsub!", b"sub!", b"chomp!", b"chop!", b"strip!", b"lstrip!", b"rstrip!",
    b"squeeze!", b"tr!", b"tr_s!", b"delete!", b"downcase!", b"upcase!",
    b"swapcase!", b"capitalize!", b"encode!", b"unicode_normalize!",
    b"scrub!", b"replace",
    b"merge!", b"update", b"reject!", b"select!", b"filter!",
    b"transform_keys!", b"transform_values!",
    b"push", b"append", b"prepend", b"concat",
    b"clear",
];

impl Cop for RedundantSelfAssignment {
    fn name(&self) -> &'static str {
        "Style/RedundantSelfAssignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, LOCAL_VARIABLE_READ_NODE, LOCAL_VARIABLE_WRITE_NODE]
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
        // Look for `x = x.method!` patterns
        let write = match node.as_local_variable_write_node() {
            Some(w) => w,
            None => return,
        };

        let var_name = write.name().as_slice();

        // Value must be a method call on the same variable
        let value = write.value();
        let call = match value.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Receiver must be the same local variable
        let recv_lvar = match receiver.as_local_variable_read_node() {
            Some(lv) => lv,
            None => return,
        };

        if recv_lvar.name().as_slice() != var_name {
            return;
        }

        // Method must be an in-place modification method
        let method_name = call.name().as_slice();
        if !INPLACE_METHODS.iter().any(|m| *m == method_name) {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Redundant self-assignment. `{}` modifies `{}` in place.",
                String::from_utf8_lossy(method_name),
                String::from_utf8_lossy(var_name),
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantSelfAssignment, "cops/style/redundant_self_assignment");
}
