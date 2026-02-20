use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BEGIN_NODE, DEF_NODE, RESCUE_NODE};

pub struct RescueEnsureAlignment;

impl Cop for RescueEnsureAlignment {
    fn name(&self) -> &'static str {
        "Layout/RescueEnsureAlignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE, DEF_NODE, RESCUE_NODE]
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

        if let Some(begin_node) = node.as_begin_node() {
            let begin_kw_loc = match begin_node.begin_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };
            let (begin_line, begin_col) = source.offset_to_line_col(begin_kw_loc.start_offset());

            // When begin is used as an assignment value (e.g., `x = begin`),
            // RuboCop aligns rescue/ensure with the start of the line (the variable),
            // not with the `begin` keyword.
            let align_col = {
                let bytes = source.as_bytes();
                let mut line_start = begin_kw_loc.start_offset();
                while line_start > 0 && bytes[line_start - 1] != b'\n' {
                    line_start -= 1;
                }
                let mut indent = 0;
                while line_start + indent < bytes.len()
                    && bytes[line_start + indent] == b' '
                {
                    indent += 1;
                }
                // If begin is NOT at the start of the line, the line likely has
                // an assignment (e.g., `x = begin`). Use the line's indent.
                if indent != begin_col { indent } else { begin_col }
            };
            let _ = begin_line;

            if let Some(rescue_node) = begin_node.rescue_clause() {
                let rescue_kw_loc = rescue_node.keyword_loc();
                let (rescue_line, rescue_col) =
                    source.offset_to_line_col(rescue_kw_loc.start_offset());
                if rescue_col != align_col {
                    diagnostics.push(self.diagnostic(
                        source,
                        rescue_line,
                        rescue_col,
                        "Align `rescue` with `begin`.".to_string(),
                    ));
                }
            }

            if let Some(ensure_node) = begin_node.ensure_clause() {
                let ensure_kw_loc = ensure_node.ensure_keyword_loc();
                let (ensure_line, ensure_col) =
                    source.offset_to_line_col(ensure_kw_loc.start_offset());
                if ensure_col != align_col {
                    diagnostics.push(self.diagnostic(
                        source,
                        ensure_line,
                        ensure_col,
                        "Align `ensure` with `begin`.".to_string(),
                    ));
                }
            }
        } else if let Some(def_node) = node.as_def_node() {
            let def_kw_loc = def_node.def_keyword_loc();
            let (_, def_col) = source.offset_to_line_col(def_kw_loc.start_offset());

            // Check if the def body is a rescue node
            if let Some(body) = def_node.body() {
                if let Some(rescue_node) = body.as_rescue_node() {
                    let rescue_kw_loc = rescue_node.keyword_loc();
                    let (rescue_line, rescue_col) =
                        source.offset_to_line_col(rescue_kw_loc.start_offset());
                    if rescue_col != def_col {
                        diagnostics.push(self.diagnostic(
                            source,
                            rescue_line,
                            rescue_col,
                            "Align `rescue` with `def`.".to_string(),
                        ));
                    }
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(RescueEnsureAlignment, "cops/layout/rescue_ensure_alignment");

    #[test]
    fn no_rescue_no_offense() {
        let source = b"begin\n  foo\nend\n";
        let diags = run_cop_full(&RescueEnsureAlignment, source);
        assert!(diags.is_empty());
    }
}
