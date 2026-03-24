// Handles both as_constant_read_node and as_constant_path_node (qualified constants like ::Proc)
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Proc;

impl Cop for Proc {
    fn name(&self) -> &'static str {
        "Style/Proc"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"new" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_name = match constant_name(&receiver) {
            Some(n) => n,
            None => return,
        };

        if recv_name != b"Proc" {
            return;
        }

        // Only flag Proc.new when it has a literal block (Proc.new { ... } or Proc.new do...end).
        // Bare Proc.new (e.g., as a default parameter value) is intentional.
        // Proc.new(&block) / Proc.new(&:sym) use BlockArgumentNode, not BlockNode — skip those.
        match call.block() {
            Some(b) if b.as_block_node().is_some() => {}
            _ => return,
        }

        let loc = receiver.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            "Use `proc` instead of `Proc.new`.".to_string(),
        );
        // Autocorrect: replace `Proc.new` (or `::Proc.new`) with `proc`
        if let Some(ref mut corr) = corrections {
            let msg_loc = call.message_loc().unwrap();
            corr.push(crate::correction::Correction {
                start: receiver.location().start_offset(),
                end: msg_loc.end_offset(),
                replacement: "proc".to_string(),
                cop_name: self.name(),
                cop_index: 0,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(Proc, "cops/style/proc");
    crate::cop_autocorrect_fixture_tests!(Proc, "cops/style/proc");

    #[test]
    fn other_class_new_is_ignored() {
        let source = b"x = Object.new\n";
        let diags = run_cop_full(&Proc, source);
        assert!(diags.is_empty());
    }
}
