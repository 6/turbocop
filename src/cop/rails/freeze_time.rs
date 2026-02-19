use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct FreezeTime;

impl Cop for FreezeTime {
    fn name(&self) -> &'static str {
        "Rails/FreezeTime"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"travel_to" {
            return;
        }

        if call.receiver().is_some() {
            return;
        }

        // Argument should be Time.now, Time.current, or Time.zone.now
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let is_time_now_or_current = is_time_now_pattern(&arg_list[0]);

        if !is_time_now_or_current {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `freeze_time` instead of `travel_to(Time.now)`.".to_string(),
        ));
    }
}

/// Check if a node represents Time.now, Time.current, or Time.zone.now
fn is_time_now_pattern(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    let method_name = call.name().as_slice();

    // Time.now or Time.current
    // Handle both ConstantReadNode (Time) and ConstantPathNode (::Time)
    if method_name == b"now" || method_name == b"current" {
        if let Some(recv) = call.receiver() {
            if util::constant_name(&recv) == Some(b"Time") {
                return true;
            }
            // Time.zone.now
            if method_name == b"now" {
                if let Some(zone_call) = recv.as_call_node() {
                    if zone_call.name().as_slice() == b"zone" {
                        if let Some(time_recv) = zone_call.receiver() {
                            if util::constant_name(&time_recv) == Some(b"Time") {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FreezeTime, "cops/rails/freeze_time");
}
