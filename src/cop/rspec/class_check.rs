use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct ClassCheck;

impl Cop for ClassCheck {
    fn name(&self) -> &'static str {
        "RSpec/ClassCheck"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = call.name().as_slice();
        let style = config.get_str("EnforcedStyle", "be_a");

        match style {
            "be_a" => {
                // Flag be_kind_of and be_a_kind_of, suggest be_a
                if method == b"be_kind_of" || method == b"be_a_kind_of" {
                    // Must not have a non-expect-chain receiver (skip Foo.be_kind_of)
                    if let Some(recv) = call.receiver() {
                        if recv.as_call_node().is_none()
                            && util::constant_name(&recv).is_some()
                        {
                            return;
                        }
                    }

                    let method_str = std::str::from_utf8(method).unwrap_or("be_kind_of");
                    let loc = call.message_loc().unwrap_or_else(|| call.location());
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Prefer `be_a` over `{method_str}`."),
                    ));
                }
            }
            "be_kind_of" => {
                // Flag be_a and be_an, suggest be_kind_of
                if method == b"be_a" || method == b"be_an" {
                    if let Some(recv) = call.receiver() {
                        if recv.as_call_node().is_none()
                            && util::constant_name(&recv).is_some()
                        {
                            return;
                        }
                    }

                    let method_str = std::str::from_utf8(method).unwrap_or("be_a");
                    let loc = call.message_loc().unwrap_or_else(|| call.location());
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Prefer `be_kind_of` over `{method_str}`."),
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
    crate::cop_fixture_tests!(ClassCheck, "cops/rspec/class_check");
}
