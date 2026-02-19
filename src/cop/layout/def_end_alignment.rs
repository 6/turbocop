use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::DEF_NODE;

pub struct DefEndAlignment;

impl Cop for DefEndAlignment {
    fn name(&self) -> &'static str {
        "Layout/DefEndAlignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let style = config.get_str("EnforcedStyleAlignWith", "start_of_line");
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Skip endless methods (no end keyword)
        let end_kw_loc = match def_node.end_keyword_loc() {
            Some(loc) => loc,
            None => return,
        };

        // Skip single-line defs (e.g., `def foo; 42; end`)
        let def_kw_offset = def_node.def_keyword_loc().start_offset();
        let (def_line, _) = source.offset_to_line_col(def_kw_offset);
        let (end_line, end_col) = source.offset_to_line_col(end_kw_loc.start_offset());
        if def_line == end_line {
            return;
        }

        match style {
            "def" => {
                // Align `end` with `def` keyword
                let (_, def_col) = source.offset_to_line_col(def_kw_offset);
                if end_col != def_col {
                    diagnostics.push(self.diagnostic(
                        source,
                        end_line,
                        end_col,
                        "Align `end` with `def`.".to_string(),
                    ));
                }

            }
            _ => {
                // "start_of_line" (default): align `end` with start of the line containing `def`
                diagnostics.extend(util::check_keyword_end_alignment(
                    self.name(),
                    source,
                    "def",
                    def_kw_offset,
                    end_kw_loc.start_offset(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(DefEndAlignment, "cops/layout/def_end_alignment");

    #[test]
    fn endless_method_no_offense() {
        let source = b"def foo = 42\n";
        let diags = run_cop_full(&DefEndAlignment, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn def_style_aligns_with_def_keyword() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyleAlignWith".into(), serde_yml::Value::String("def".into())),
            ]),
            ..CopConfig::default()
        };
        // `end` aligned with `def` (both at column 2)
        let src = b"  def foo\n    42\n  end\n";
        let diags = run_cop_full_with_config(&DefEndAlignment, src, config.clone());
        assert!(diags.is_empty(), "def style should accept end aligned with def");

        // `end` at column 0, `def` at column 2 → mismatch
        let src2 = b"  def foo\n    42\nend\n";
        let diags2 = run_cop_full_with_config(&DefEndAlignment, src2, config);
        assert_eq!(diags2.len(), 1, "def style should flag end not aligned with def");
    }
}
