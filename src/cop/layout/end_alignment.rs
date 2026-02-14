use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EndAlignment;

impl Cop for EndAlignment {
    fn name(&self) -> &'static str {
        "Layout/EndAlignment"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        if let Some(class_node) = node.as_class_node() {
            return self.check_keyword_end(
                source,
                class_node.class_keyword_loc().start_offset(),
                class_node.end_keyword_loc().start_offset(),
                "class",
            );
        }

        if let Some(module_node) = node.as_module_node() {
            return self.check_keyword_end(
                source,
                module_node.module_keyword_loc().start_offset(),
                module_node.end_keyword_loc().start_offset(),
                "module",
            );
        }

        if let Some(if_node) = node.as_if_node() {
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return Vec::new(),
            };
            // Only check top-level if/unless, not elsif
            let kw_slice = kw_loc.as_slice();
            if kw_slice != b"if" && kw_slice != b"unless" {
                return Vec::new();
            }
            let end_kw_loc = match if_node.end_keyword_loc() {
                Some(loc) => loc,
                None => return Vec::new(),
            };
            let keyword = if kw_slice == b"if" { "if" } else { "unless" };
            return self.check_keyword_end(
                source,
                kw_loc.start_offset(),
                end_kw_loc.start_offset(),
                keyword,
            );
        }

        if let Some(while_node) = node.as_while_node() {
            let kw_loc = while_node.keyword_loc();
            if let Some(end_loc) = while_node.closing_loc() {
                return self.check_keyword_end(
                    source,
                    kw_loc.start_offset(),
                    end_loc.start_offset(),
                    "while",
                );
            }
        }

        if let Some(until_node) = node.as_until_node() {
            let kw_loc = until_node.keyword_loc();
            if let Some(end_loc) = until_node.closing_loc() {
                return self.check_keyword_end(
                    source,
                    kw_loc.start_offset(),
                    end_loc.start_offset(),
                    "until",
                );
            }
        }

        if let Some(case_node) = node.as_case_node() {
            let kw_loc = case_node.case_keyword_loc();
            let end_loc = case_node.end_keyword_loc();
            return self.check_keyword_end(
                source,
                kw_loc.start_offset(),
                end_loc.start_offset(),
                "case",
            );
        }

        if let Some(begin_node) = node.as_begin_node() {
            if let Some(begin_kw_loc) = begin_node.begin_keyword_loc() {
                if let Some(end_kw_loc) = begin_node.end_keyword_loc() {
                    return self.check_keyword_end(
                        source,
                        begin_kw_loc.start_offset(),
                        end_kw_loc.start_offset(),
                        "begin",
                    );
                }
            }
        }

        Vec::new()
    }
}

impl EndAlignment {
    fn check_keyword_end(
        &self,
        source: &SourceFile,
        kw_offset: usize,
        end_offset: usize,
        keyword: &str,
    ) -> Vec<Diagnostic> {
        let (_, kw_col) = source.offset_to_line_col(kw_offset);
        let (end_line, end_col) = source.offset_to_line_col(end_offset);

        if end_col != kw_col {
            return vec![self.diagnostic(
                source,
                end_line,
                end_col,
                format!("Align `end` with `{keyword}`."),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(EndAlignment, "cops/layout/end_alignment");

    #[test]
    fn modifier_if_no_offense() {
        let source = b"x = 1 if true\n";
        let diags = run_cop_full(&EndAlignment, source);
        assert!(diags.is_empty());
    }
}
