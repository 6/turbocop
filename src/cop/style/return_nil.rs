use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{NIL_NODE, RETURN_NODE};

pub struct ReturnNil;

impl Cop for ReturnNil {
    fn name(&self) -> &'static str {
        "Style/ReturnNil"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[NIL_NODE, RETURN_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "return");

        let ret_node = match node.as_return_node() {
            Some(r) => r,
            None => return,
        };

        match enforced_style {
            "return" => {
                // Flag `return nil` — prefer `return`
                if let Some(args) = ret_node.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if arg_list.len() == 1 && arg_list[0].as_nil_node().is_some() {
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Use `return` instead of `return nil`.".to_string(),
                        ));
                    }
                }
            }
            "return_nil" => {
                // Flag bare `return` — prefer `return nil`
                if ret_node.arguments().is_none() {
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `return nil` instead of `return`.".to_string(),
                    ));
                }
            }
            _ => {}
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ReturnNil, "cops/style/return_nil");
}
