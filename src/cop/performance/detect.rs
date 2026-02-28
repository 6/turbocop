use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

const CANDIDATE_METHODS: &[&[u8]] = &[b"select", b"find_all", b"filter"];

pub struct Detect;

impl Cop for Detect {
    fn name(&self) -> &'static str {
        "Performance/Detect"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let outer_method = outer_call.name().as_slice();

        // Determine the second_method type: first, last, or [0]/[-1]
        let (is_last, is_index) = match outer_method {
            b"first" => (false, false),
            b"last" => (true, false),
            b"[]" => {
                // Must have exactly one integer argument: 0 or -1
                let args = match outer_call.arguments() {
                    Some(a) => a,
                    None => return,
                };
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 {
                    return;
                }
                let arg_text = std::str::from_utf8(arg_list[0].location().as_slice()).unwrap_or("");
                match arg_text {
                    "0" => (false, true),
                    "-1" => (true, true),
                    _ => return,
                }
            }
            _ => return,
        };

        // For first/last, must have NO arguments (first(n) / last(n) should not flag)
        if !is_index && outer_call.arguments().is_some() {
            return;
        }

        // Get the inner call (receiver of the outer call)
        let receiver = match outer_call.receiver() {
            Some(r) => r,
            None => return,
        };
        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let inner_method = inner_call.name().as_slice();

        // Inner method must be select, find_all, or filter
        if !CANDIDATE_METHODS.contains(&inner_method) {
            return;
        }

        // The inner call must have a block or block_pass
        let has_block = inner_call.block().is_some();
        if !has_block {
            return;
        }

        // Check if the inner call's receiver is `lazy` with its own receiver
        // e.g., `adapter.lazy.select { }.first` should not flag
        // but `lazy.select { }.first` (bare lazy without receiver) should flag
        if let Some(inner_receiver) = inner_call.receiver() {
            if let Some(lazy_call) = inner_receiver.as_call_node() {
                if lazy_call.name().as_slice() == b"lazy" && lazy_call.receiver().is_some() {
                    return;
                }
            }
        }

        let inner_method_str = std::str::from_utf8(inner_method).unwrap_or("select");
        let msg = if is_index {
            let idx = if is_last { -1 } else { 0 };
            if is_last {
                format!("Use `reverse.detect` instead of `{inner_method_str}[{idx}]`.")
            } else {
                format!("Use `detect` instead of `{inner_method_str}[{idx}]`.")
            }
        } else if is_last {
            format!("Use `reverse.detect` instead of `{inner_method_str}.last`.")
        } else {
            format!("Use `detect` instead of `{inner_method_str}.first`.")
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, msg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Detect, "cops/performance/detect");
}
