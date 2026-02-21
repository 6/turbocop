use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Caller;

impl Cop for Caller {
    fn name(&self) -> &'static str {
        "Performance/Caller"
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
        // Pattern: caller.first, caller[n], caller_locations.first, caller_locations[n]
        if let Some(chain) = as_method_chain(node) {
            let is_caller = chain.inner_method == b"caller";
            let is_caller_locations = chain.inner_method == b"caller_locations";
            if (is_caller || is_caller_locations) && chain.inner_call.receiver().is_none() {
                // inner call is bare `caller` or `caller_locations` (no receiver)
                if chain.inner_call.arguments().is_none() {
                    if chain.outer_method == b"first" || chain.outer_method == b"[]" {
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        let method = if is_caller { "caller" } else { "caller_locations" };
                        diagnostics.push(self.diagnostic(source, line, column,
                            format!("Use `{method}(n..n).first` instead of `{method}.first` or `{method}[n]`.")));
                    }
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Caller, "cops/performance/caller");
}
